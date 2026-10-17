#[cfg(feature = "auto-trophies")]
mod achievements;
#[cfg(feature = "auto-trophies")]
mod auto_trophies;
mod catalog;
mod error;
mod github;
mod install;
mod launch;
mod settings;
mod setup;
mod state;

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
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
    /// Usado pelas conquistas automáticas (troféus escritos pelo usuário).
    #[cfg_attr(not(feature = "auto-trophies"), allow(dead_code))]
    data_dir: PathBuf,
    default_games_dir: PathBuf,
    settings_path: PathBuf,
    settings: Mutex<Settings>,
    cache_dir: PathBuf,
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
        let settings = self.settings.lock().unwrap();
        let custom = settings.games_dir.clone();
        SettingsView {
            language: settings.language.clone(),
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
    language: Option<String>,
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
    rom: Option<PathBuf>,
) -> Result<Library> {
    let _guard = BusyGuard::acquire(&state, &game_id)?;
    let game = state.game(&game_id)?;
    let platform = state.platform_asset(&game)?;
    // Antes de baixar centenas de MB do instalador.
    if platform.installer.as_ref().is_some_and(setup::needs_rom) && rom.is_none() {
        return Err(Error::RomRequired);
    }
    let release = state.github.latest_release(&game.repo).await?;
    // Atualização acontece onde o jogo já está (inclusive se foi importado).
    let target = match state.installed_game(&game_id) {
        Ok(current) => current.install_path,
        Err(_) => state.games_dir().join(&game_id),
    };

    let emit = {
        let app = app.clone();
        let game_id = game_id.clone();
        move |progress| {
            let _ = app.emit(
                "install-progress",
                InstallProgressEvent {
                    game_id: game_id.clone(),
                    progress,
                },
            );
        }
    };
    let installed = match &platform.installer {
        // O instalador do projeto decide onde o jogo fica.
        Some(installer) => {
            setup::install(&state.http, &state.cache_dir, platform, installer, &release, rom, emit).await?
        }
        None => install::install(&state.http, &target, platform, &release, emit).await?,
    };

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
    let mut installed = state.installed_game(&game_id)?;
    // O instalador do projeto pode ter se atualizado sozinho (outra pasta de versão).
    if installed.external {
        let game = state.game(&game_id)?;
        let platform = state.platform_asset(&game)?;
        if let Some(installer) = &platform.installer
            && let Ok(Some(found)) = setup::detect(installer, platform)
            && found.executable != installed.executable
        {
            installed.executable = found.executable;
            installed.executable_hash = found.executable_hash;
            let updated = installed.clone();
            let id = game_id.clone();
            state.update_library(move |library| {
                if let Some(entry) = library.get_mut(&id) {
                    entry.installed = Some(updated);
                }
            })?;
        }
    }
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
    // Conquistas automáticas (leitura de memória / arquivo do jogo): só com a
    // feature `auto-trophies`, guardada para implantação futura.
    #[cfg(feature = "auto-trophies")]
    auto_trophies::start_watch(&app, &state, &game_id, child.id(), alive.clone());
    std::thread::spawn(move || wait_for_exit(app, game_id, child, alive));
    Ok(())
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

/// Lista de conquistas do jogo (as que o próprio jogo/port define, vindas do
/// catálogo). O usuário marca e desmarca à mão; as marcações ficam na
/// biblioteca (`trophies`: chave → quando foi marcada).
#[tauri::command]
fn get_trophies(state: State<'_, AppState>, game_id: String) -> TrophiesView {
    #[cfg(feature = "auto-trophies")]
    if let Some(view) = auto_trophies::file_trophies_view(&state, &game_id) {
        return view;
    }

    let marked = state
        .library
        .lock()
        .unwrap()
        .get(&game_id)
        .map(|e| e.trophies.clone())
        .unwrap_or_default();
    let Some(provider) = state.game(&game_id).ok().and_then(|g| g.trophy_provider) else {
        return TrophiesView {
            path: PathBuf::new(),
            trophies: Vec::new(),
            load_error: None,
            provider: None,
        };
    };
    let trophies = provider
        .trophies
        .iter()
        .map(|t| {
            let unlocked_at = marked.get(&t.key).copied();
            TrophyView {
                key: t.key.clone(),
                title: t.title.clone(),
                description: t.description.clone(),
                points: 0,
                category: t.category.clone(),
                goal: t.goal,
                progress: if unlocked_at.is_some() { t.goal } else { 0 },
                unlocked: unlocked_at.is_some(),
                unlocked_at,
                error: None,
            }
        })
        .collect();
    TrophiesView {
        path: PathBuf::new(),
        trophies,
        load_error: None,
        provider: Some(provider.kind),
    }
}

/// Marca (ou desmarca) uma conquista como feita.
#[tauri::command]
fn set_trophy(state: State<'_, AppState>, game_id: String, key: String, unlocked: bool) -> Result<Library> {
    state.game(&game_id)?;
    state.update_library(|library| {
        let entry = library.entry(game_id).or_insert_with(LibraryEntry::new);
        if unlocked {
            entry.trophies.entry(key).or_insert_with(chrono::Local::now);
        } else {
            entry.trophies.remove(&key);
        }
        library.clone()
    })
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
        // Jogo de instalador próprio já instalado por fora do launcher.
        if let Some(installer) = &platform.installer {
            if let Ok(Some(installed)) = setup::detect(installer, platform) {
                found.push((game.id, installed));
            }
            continue;
        }
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

/// Troféus com condições de memória escritos pelo usuário (automáticos).
#[tauri::command]
fn edit_trophies(app: AppHandle, state: State<'_, AppState>, game_id: String) -> Result<()> {
    #[cfg(feature = "auto-trophies")]
    return auto_trophies::edit(&app, &state, &game_id);
    #[cfg(not(feature = "auto-trophies"))]
    {
        let _ = (app, state, game_id);
        Err(Error::Io(std::io::Error::other("conquistas automáticas desativadas nesta versão")))
    }
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

/// `language` é a escolha salva (`None` = seguir o sistema); `effective` é o
/// idioma que a interface está usando, para as mensagens de erro baterem.
#[tauri::command]
fn set_language(state: State<'_, AppState>, language: Option<String>, effective: String) -> Result<SettingsView> {
    let valid = |l: &str| matches!(l, "pt" | "en");
    let language = language.filter(|l| valid(l));
    error::set_english(effective == "en");
    {
        let mut current = state.settings.lock().unwrap();
        if current.language != language {
            current.language = language;
            settings::save(&state.settings_path, &current)?;
        }
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
    let settings = settings::load(&settings_path)?;
    error::set_english(settings.language.as_deref() == Some("en"));
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
        settings: Mutex::new(settings),
        cache_dir: cache_dir.clone(),
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
        // Lembra se a janela estava maximizada (e tamanho/posição) entre aberturas.
        .plugin(tauri_plugin_window_state::Builder::default().build())
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
            set_trophy,
            scan_games_dir,
            edit_trophies,
            set_games_dir,
            set_language,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
