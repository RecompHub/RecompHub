mod achievements;
mod catalog;
mod error;
mod github;
mod install;
mod launch;
mod settings;
mod state;
mod trophies;

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;

use catalog::{Catalog, Game, PlatformAsset};
use error::{Error, Result};
use github::{GitHub, Release};
use settings::Settings;
use state::{InstalledGame, Library, LibraryEntry};

struct AppState {
    catalog: RwLock<Catalog>,
    catalog_cache_path: PathBuf,
    platform: String,
    http: reqwest::Client,
    github: GitHub,
    data_dir: PathBuf,
    default_games_dir: PathBuf,
    settings_path: PathBuf,
    settings: Mutex<Settings>,
    library_path: PathBuf,
    library: Mutex<Library>,
    /// Jogos com instalação/remoção em andamento.
    busy: Mutex<HashSet<String>>,
    running: Mutex<HashSet<String>>,
}

impl AppState {
    fn game(&self, id: &str) -> Result<Game> {
        self.catalog
            .read()
            .unwrap()
            .games
            .iter()
            .find(|g| g.id == id)
            .cloned()
            .ok_or_else(|| Error::UnknownGame(id.to_string()))
    }

    fn platform_asset<'a>(&self, game: &'a Game) -> Result<&'a PlatformAsset> {
        game.platforms
            .get(&self.platform)
            .ok_or_else(|| Error::UnsupportedPlatform(game.name.clone(), self.platform.clone()))
    }

    fn installed_game(&self, id: &str) -> Result<InstalledGame> {
        self.library
            .lock()
            .unwrap()
            .get(id)
            .and_then(|e| e.installed.clone())
            .ok_or_else(|| Error::NotInstalled(id.to_string()))
    }

    fn update_library<T>(&self, f: impl FnOnce(&mut Library) -> T) -> Result<T> {
        let mut library = self.library.lock().unwrap();
        let result = f(&mut library);
        state::save(&self.library_path, &library)?;
        Ok(result)
    }

    /// Pasta onde a próxima instalação vai: a escolhida pelo usuário ou a padrão.
    fn games_dir(&self) -> PathBuf {
        self.settings
            .lock()
            .unwrap()
            .games_dir
            .clone()
            .unwrap_or_else(|| self.default_games_dir.clone())
    }

    fn settings_view(&self) -> SettingsView {
        let custom = self.settings.lock().unwrap().games_dir.clone();
        SettingsView {
            games_dir: custom
                .clone()
                .unwrap_or_else(|| self.default_games_dir.clone()),
            default_games_dir: self.default_games_dir.clone(),
            is_default_games_dir: custom.is_none(),
        }
    }

    fn is_running(&self, id: &str) -> bool {
        self.running.lock().unwrap().contains(id)
    }
}

/// Marca um jogo como ocupado enquanto viver; libera no `drop`, inclusive em erro.
struct BusyGuard<'a> {
    state: &'a AppState,
    id: String,
}

impl<'a> BusyGuard<'a> {
    fn acquire(state: &'a AppState, id: &str) -> Result<Self> {
        if state.is_running(id) {
            return Err(Error::Running);
        }
        if !state.busy.lock().unwrap().insert(id.to_string()) {
            return Err(Error::Busy(id.to_string()));
        }
        Ok(Self {
            state,
            id: id.to_string(),
        })
    }
}

impl Drop for BusyGuard<'_> {
    fn drop(&mut self) {
        self.state.busy.lock().unwrap().remove(&self.id);
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallProgressEvent {
    game_id: String,
    #[serde(flatten)]
    progress: install::Progress,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GameExitedEvent {
    game_id: String,
    session_seconds: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TrophyUnlockedEvent {
    game_id: String,
    key: String,
    title: String,
    description: String,
}

/// Troféu na interface, venha ele de condições do RA (arquivo local) ou do
/// próprio jogo (provedor).
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TrophyView {
    key: String,
    title: String,
    description: String,
    points: u32,
    category: String,
    /// Contadores: meta e progresso (ex.: 12/60). Meta 0 = troféu simples.
    goal: u32,
    progress: u32,
    unlocked: bool,
    /// Só quando o launcher viu o desbloqueio acontecer.
    unlocked_at: Option<chrono::DateTime<chrono::Local>>,
    /// Condição inválida (o troféu não é avaliado).
    error: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TrophiesView {
    /// Arquivo de definições ou de progresso (pode não existir ainda).
    path: PathBuf,
    trophies: Vec<TrophyView>,
    /// JSON inválido no arquivo.
    load_error: Option<String>,
    /// Ex.: "dusklight" quando os troféus vêm do próprio jogo (não editáveis).
    provider: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SettingsView {
    games_dir: PathBuf,
    default_games_dir: PathBuf,
    is_default_games_dir: bool,
}

#[tauri::command]
fn get_catalog(state: State<'_, AppState>) -> Catalog {
    state.catalog.read().unwrap().clone()
}

/// Baixa o catálogo publicado. Em caso de erro o catálogo atual continua valendo.
#[tauri::command]
async fn refresh_catalog(state: State<'_, AppState>) -> Result<Catalog> {
    let remote = catalog::fetch_remote(&state.http, &state.catalog_cache_path).await?;
    let mut current = state.catalog.write().unwrap();
    // Nunca troca por um catálogo mais antigo (ex.: o embutido numa versão
    // nova do launcher é mais recente que o publicado em cache).
    if remote.generated_at >= current.generated_at {
        *current = remote;
    }
    Ok(current.clone())
}

#[tauri::command]
fn current_platform(state: State<'_, AppState>) -> String {
    state.platform.clone()
}

#[tauri::command]
fn get_library(state: State<'_, AppState>) -> Library {
    state.library.lock().unwrap().clone()
}

#[tauri::command]
fn running_games(state: State<'_, AppState>) -> Vec<String> {
    state.running.lock().unwrap().iter().cloned().collect()
}

#[tauri::command]
fn add_to_library(state: State<'_, AppState>, game_id: String) -> Result<Library> {
    state.game(&game_id)?;
    state.update_library(|library| {
        library.entry(game_id).or_insert_with(LibraryEntry::new);
        library.clone()
    })
}

#[tauri::command]
fn remove_from_library(state: State<'_, AppState>, game_id: String) -> Result<Library> {
    if state.installed_game(&game_id).is_ok() {
        return Err(Error::StillInstalled);
    }
    state.update_library(|library| {
        library.remove(&game_id);
        library.clone()
    })
}

#[tauri::command]
async fn latest_release(state: State<'_, AppState>, game_id: String) -> Result<Release> {
    let game = state.game(&game_id)?;
    state.github.latest_release(&game.repo).await
}

#[tauri::command]
async fn install_game(
    app: AppHandle,
    state: State<'_, AppState>,
    game_id: String,
) -> Result<Library> {
    let _guard = BusyGuard::acquire(&state, &game_id)?;
    let game = state.game(&game_id)?;
    let platform = state.platform_asset(&game)?;
    let release = state.github.latest_release(&game.repo).await?;
    // Atualização acontece onde o jogo já está (inclusive se foi importado).
    let target = match state.installed_game(&game_id) {
        Ok(current) => current.install_path,
        Err(_) => state.games_dir().join(&game_id),
    };

    let installed = install::install(&state.http, &target, platform, &release, |progress| {
        let _ = app.emit(
            "install-progress",
            InstallProgressEvent {
                game_id: game_id.clone(),
                progress,
            },
        );
    })
    .await?;

    state.update_library(|library| {
        library
            .entry(game_id)
            .or_insert_with(LibraryEntry::new)
            .installed = Some(installed);
        library.clone()
    })
}

#[tauri::command]
fn import_game(state: State<'_, AppState>, game_id: String, path: PathBuf) -> Result<Library> {
    let _guard = BusyGuard::acquire(&state, &game_id)?;
    let game = state.game(&game_id)?;
    let installed = install::import(&path, state.platform_asset(&game)?)?;
    state.update_library(|library| {
        library
            .entry(game_id)
            .or_insert_with(LibraryEntry::new)
            .installed = Some(installed);
        library.clone()
    })
}

#[tauri::command]
fn uninstall_game(state: State<'_, AppState>, game_id: String) -> Result<Library> {
    let _guard = BusyGuard::acquire(&state, &game_id)?;
    let installed = state.installed_game(&game_id)?;
    install::uninstall(&game_id, &installed)?;
    state.update_library(|library| {
        if let Some(entry) = library.get_mut(&game_id) {
            entry.installed = None;
        }
        library.clone()
    })
}

#[tauri::command]
fn launch_game(app: AppHandle, state: State<'_, AppState>, game_id: String) -> Result<()> {
    if state.busy.lock().unwrap().contains(&game_id) {
        return Err(Error::Busy(game_id));
    }
    let installed = state.installed_game(&game_id)?;
    let executable = installed.install_path.join(&installed.executable);
    if !executable.exists() {
        return Err(Error::ExecutableNotFound(executable.display().to_string()));
    }

    let mut running = state.running.lock().unwrap();
    if running.contains(&game_id) {
        return Err(Error::Running);
    }
    let child = launch::spawn(&executable)?;
    running.insert(game_id.clone());
    let _ = app.emit("game-started", &game_id);

    let alive = Arc::new(AtomicBool::new(true));
    start_trophy_watch(&app, &state, &game_id, child.id(), alive.clone());
    std::thread::spawn(move || wait_for_exit(app, game_id, child, alive));
    Ok(())
}

/// Se o jogo tem troféus definidos, acompanha a memória dele em outra thread.
fn start_trophy_watch(
    app: &AppHandle,
    state: &AppState,
    game_id: &str,
    pid: u32,
    alive: Arc<AtomicBool>,
) {
    let unlock = {
        let app = app.clone();
        let game_id = game_id.to_string();
        move |key: String, title: &str, description: &str| {
            let state = app.state::<AppState>();
            let _ = state.update_library(|library| {
                if let Some(entry) = library.get_mut(&game_id) {
                    entry
                        .trophies
                        .entry(key.clone())
                        .or_insert_with(chrono::Local::now);
                }
            });
            // Notificação do sistema: aparece por cima do jogo em tela cheia,
            // onde a janela do launcher não é vista.
            let _ = app
                .notification()
                .builder()
                .title(format!("🏆 Troféu desbloqueado: {title}"))
                .body(description)
                .show();
            let _ = app.emit(
                "trophy-unlocked",
                TrophyUnlockedEvent {
                    game_id: game_id.clone(),
                    key,
                    title: title.to_string(),
                    description: description.to_string(),
                },
            );
        }
    };

    // Conquistas do próprio jogo: basta acompanhar o arquivo de progresso.
    if let Some(provider) = state.game(game_id).ok().and_then(|g| g.trophy_provider) {
        let Some(path) = trophies::provider_progress_path(&provider) else {
            return;
        };
        std::thread::spawn(move || {
            trophies::watch_provider(path, alive, provider.trophies, |t| {
                unlock(t.key.clone(), &t.title, &t.description)
            });
        });
        return;
    }

    // Troféus escritos pelo usuário (condições do RA): lê a RDRAM do processo.
    let path = trophies::definitions_path(&state.data_dir, game_id);
    let Ok(Some(set)) = trophies::load(&path) else {
        return;
    };
    let already: HashSet<u32> = state
        .library
        .lock()
        .unwrap()
        .get(game_id)
        .map(|e| e.trophies.keys().filter_map(|k| k.parse().ok()).collect())
        .unwrap_or_default();
    std::thread::spawn(move || {
        trophies::watch(pid, alive, set, already, |t| {
            unlock(t.id.to_string(), &t.title, &t.description)
        });
    });
}

fn wait_for_exit(
    app: AppHandle,
    game_id: String,
    mut child: std::process::Child,
    alive: Arc<AtomicBool>,
) {
    let started = Instant::now();
    let _ = child.wait();
    alive.store(false, Ordering::Relaxed);
    let session_seconds = started.elapsed().as_secs();

    let state = app.state::<AppState>();
    state.running.lock().unwrap().remove(&game_id);
    let _ = state.update_library(|library| {
        if let Some(entry) = library.get_mut(&game_id) {
            entry.play_time_seconds += session_seconds;
            entry.last_played_at = Some(chrono::Local::now());
        }
    });
    let _ = app.emit(
        "game-exited",
        GameExitedEvent {
            game_id,
            session_seconds,
        },
    );
}

#[tauri::command]
fn open_install_dir(app: AppHandle, state: State<'_, AppState>, game_id: String) -> Result<()> {
    let installed = state.installed_game(&game_id)?;
    app.opener()
        .open_path(installed.install_path.to_string_lossy(), None::<&str>)
        .map_err(|e| Error::Io(std::io::Error::other(e.to_string())))
}

#[tauri::command]
fn get_trophies(state: State<'_, AppState>, game_id: String) -> TrophiesView {
    let seen = state
        .library
        .lock()
        .unwrap()
        .get(&game_id)
        .map(|e| e.trophies.clone())
        .unwrap_or_default();

    if let Some(provider) = state.game(&game_id).ok().and_then(|g| g.trophy_provider) {
        let path = trophies::provider_progress_path(&provider).unwrap_or_default();
        let (progress, load_error) = match trophies::read_provider_progress(&path) {
            Ok(p) => (p, None),
            Err(e) => (Default::default(), Some(e.to_string())),
        };
        let trophies = provider
            .trophies
            .iter()
            .map(|t| {
                let entry = progress.get(&t.key).cloned().unwrap_or_default();
                TrophyView {
                    key: t.key.clone(),
                    title: t.title.clone(),
                    description: t.description.clone(),
                    points: 0,
                    category: t.category.clone(),
                    goal: t.goal,
                    progress: entry.progress.clamp(0, u32::MAX as i64) as u32,
                    unlocked: entry.unlocked,
                    unlocked_at: seen.get(&t.key).copied(),
                    error: None,
                }
            })
            .collect();
        return TrophiesView {
            path,
            trophies,
            load_error,
            provider: Some(provider.kind),
        };
    }

    let path = trophies::definitions_path(&state.data_dir, &game_id);
    let (set, load_error) = match trophies::load(&path) {
        Ok(set) => (set, None),
        Err(e) => (None, Some(e.to_string())),
    };
    let mut errors = set.as_ref().map(trophies::validate).unwrap_or_default();
    let trophies = set
        .map(|s| s.trophies)
        .unwrap_or_default()
        .into_iter()
        .map(|t| {
            let key = t.id.to_string();
            let unlocked_at = seen.get(&key).copied();
            TrophyView {
                error: errors.remove(&t.id),
                key,
                title: t.title,
                description: t.description,
                points: t.points,
                category: String::new(),
                goal: 0,
                progress: unlocked_at.is_some() as u32,
                unlocked: unlocked_at.is_some(),
                unlocked_at,
            }
        })
        .collect();
    TrophiesView {
        path,
        trophies,
        load_error,
        provider: None,
    }
}

/// Registra jogos do catálogo que já estão na pasta de jogos (ex.: copiados
/// à mão), pela pasta com o id do jogo e o executável esperado.
#[tauri::command]
fn scan_games_dir(state: State<'_, AppState>) -> Result<Library> {
    let dir = state.games_dir();
    let folders: Vec<PathBuf> = match std::fs::read_dir(&dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
            .map(|e| e.path())
            .collect(),
        Err(_) => return Ok(state.library.lock().unwrap().clone()),
    };
    let games = state.catalog.read().unwrap().games.clone();
    let mut found = Vec::new();
    for game in games {
        if state.installed_game(&game.id).is_ok() || state.busy.lock().unwrap().contains(&game.id) {
            continue;
        }
        let Some(platform) = game.platforms.get(&state.platform) else {
            continue;
        };
        let folder = folders.iter().find(|f| {
            f.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.eq_ignore_ascii_case(&game.id))
        });
        if let Some(installed) = folder.and_then(|f| install::import(f, platform).ok()) {
            found.push((game.id, installed));
        }
    }
    state.update_library(|library| {
        for (id, installed) in found {
            library
                .entry(id)
                .or_insert_with(LibraryEntry::new)
                .installed = Some(installed);
        }
        library.clone()
    })
}

/// Abre o arquivo de troféus do jogo no editor padrão (cria um modelo se não existir).
#[tauri::command]
fn edit_trophies(app: AppHandle, state: State<'_, AppState>, game_id: String) -> Result<()> {
    state.game(&game_id)?;
    let path = trophies::definitions_path(&state.data_dir, &game_id);
    if !path.exists() {
        trophies::write_template(&path, &game_id)?;
    }
    app.opener()
        .open_path(path.to_string_lossy(), None::<&str>)
        .map_err(|e| Error::Io(std::io::Error::other(e.to_string())))
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> SettingsView {
    state.settings_view()
}

/// `None` volta para a pasta padrão. Jogos já instalados ficam onde estão.
#[tauri::command]
fn set_games_dir(state: State<'_, AppState>, path: Option<PathBuf>) -> Result<SettingsView> {
    if let Some(dir) = &path {
        settings::validate_games_dir(dir)?;
    }
    {
        let mut current = state.settings.lock().unwrap();
        current.games_dir = path;
        settings::save(&state.settings_path, &current)?;
    }
    Ok(state.settings_view())
}

/// `C:\RecompHub\Games` no Windows (ao lado do launcher, fácil de achar);
/// nos outros sistemas, dentro da pasta de dados do app.
fn default_games_dir(data_dir: &std::path::Path) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(r"C:\RecompHub\Games")
    } else {
        data_dir.join("games")
    }
}

fn init_state(app: &AppHandle) -> Result<AppState> {
    let paths = app.path();
    let data_dir = paths.app_local_data_dir()?;
    let cache_dir = paths.app_cache_dir()?;
    let library_path = data_dir.join("library.json");
    let settings_path = data_dir.join("settings.json");
    let catalog_cache_path = cache_dir.join("catalog.json");
    let http = reqwest::Client::builder()
        .user_agent(concat!("RecompHub/", env!("CARGO_PKG_VERSION")))
        .build()?;

    Ok(AppState {
        catalog: RwLock::new(catalog::load(&catalog_cache_path)?),
        catalog_cache_path,
        platform: catalog::current_platform(),
        github: GitHub::new(http.clone(), cache_dir.join("github")),
        http,
        default_games_dir: default_games_dir(&data_dir),
        data_dir: data_dir.clone(),
        settings: Mutex::new(settings::load(&settings_path)?),
        settings_path,
        library: Mutex::new(state::load(&library_path)?),
        library_path,
        busy: Mutex::default(),
        running: Mutex::default(),
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let state = init_state(app.handle())?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_catalog,
            refresh_catalog,
            current_platform,
            get_library,
            running_games,
            add_to_library,
            remove_from_library,
            latest_release,
            install_game,
            uninstall_game,
            import_game,
            launch_game,
            open_install_dir,
            get_settings,
            get_trophies,
            scan_games_dir,
            edit_trophies,
            set_games_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
