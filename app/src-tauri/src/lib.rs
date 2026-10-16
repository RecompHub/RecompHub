mod achievements;
mod catalog;
mod error;
mod github;
mod install;
mod launch;
mod settings;
mod state;

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Mutex, RwLock};
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
            games_dir: custom.clone().unwrap_or_else(|| self.default_games_dir.clone()),
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
    let catalog = catalog::fetch_remote(&state.http, &state.catalog_cache_path).await?;
    *state.catalog.write().unwrap() = catalog.clone();
    Ok(catalog)
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

    let installed = install::install(
        &state.http,
        &state.games_dir(),
        &game_id,
        platform,
        &release,
        |progress| {
            let _ = app.emit(
                "install-progress",
                InstallProgressEvent {
                    game_id: game_id.clone(),
                    progress,
                },
            );
        },
    )
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
    std::thread::spawn(move || wait_for_exit(app, game_id, child));
    Ok(())
}

fn wait_for_exit(app: AppHandle, game_id: String, mut child: std::process::Child) {
    let started = Instant::now();
    let _ = child.wait();
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
        default_games_dir: data_dir.join("games"),
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
            set_games_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
