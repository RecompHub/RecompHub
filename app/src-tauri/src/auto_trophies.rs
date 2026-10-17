//! **Conquistas automáticas — guardadas para implantação futura.**
//!
//! Só compila com a feature `auto-trophies` (`cargo build --features
//! auto-trophies`). Hoje o launcher usa a lista manual (o usuário marca e
//! desmarca); este módulo é o que, no futuro, marca sozinho enquanto o jogo
//! roda. Dois caminhos, ambos testados de ponta a ponta (ver PRs #8 e #9):
//!
//! - **Provedor do próprio jogo** (ex.: Dusklight): acompanha o arquivo de
//!   progresso que o jogo grava (`achievements.json`) e marca o que ele
//!   desbloquear, com notificação do Windows.
//! - **Condições do RetroAchievements**: lê a RDRAM do processo da recomp de
//!   fora e avalia com a rcheevos (~60×/s); definições em
//!   `<pasta de dados>/trophies/<id do jogo>.json`, escritas pelo usuário.
//!
//! As marcações vão para o mesmo lugar da lista manual
//! (`LibraryEntry::trophies`), então ligar a feature não muda a interface.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;

pub use recomphub_catalog::{Trophy, TrophySet};

use crate::error::{Error, Result};
use crate::{AppState, TrophiesView, TrophyView};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TrophyUnlockedEvent {
    game_id: String,
    key: String,
    title: String,
    description: String,
}

/// Começa a acompanhar o jogo recém-aberto (`pid`) até `alive` virar falso.
pub fn start_watch(app: &AppHandle, state: &AppState, game_id: &str, pid: u32, alive: Arc<AtomicBool>) {
    let unlock = {
        let app = app.clone();
        let game_id = game_id.to_string();
        move |key: String, title: &str, description: &str| {
            let state = app.state::<AppState>();
            let _ = state.update_library(|library| {
                if let Some(entry) = library.get_mut(&game_id) {
                    entry.trophies.entry(key.clone()).or_insert_with(chrono::Local::now);
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
        let Some(path) = provider_progress_path(&provider) else {
            return;
        };
        std::thread::spawn(move || {
            watch_provider(path, alive, provider.trophies, |t| {
                unlock(t.key.clone(), &t.title, &t.description)
            });
        });
        return;
    }

    // Troféus escritos pelo usuário (condições do RA): lê a RDRAM do processo.
    let path = definitions_path(&state.data_dir, game_id);
    let Ok(Some(set)) = load(&path) else {
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
        watch(pid, alive, set, already, |t| {
            unlock(t.id.to_string(), &t.title, &t.description)
        });
    });
}

/// Lista dos troféus com condições escritos pelo usuário, se o jogo tiver o
/// arquivo. `None` = usar a lista normal do catálogo.
pub fn file_trophies_view(state: &AppState, game_id: &str) -> Option<TrophiesView> {
    let path = definitions_path(&state.data_dir, game_id);
    let (set, load_error) = match load(&path) {
        Ok(Some(set)) => (set, None),
        Ok(None) => return None,
        Err(e) => (TrophySet { game_id: game_id.into(), trophies: Vec::new() }, Some(e.to_string())),
    };
    let seen = state
        .library
        .lock()
        .unwrap()
        .get(game_id)
        .map(|e| e.trophies.clone())
        .unwrap_or_default();
    let mut errors = validate(&set);
    let trophies = set
        .trophies
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
    Some(TrophiesView { path, trophies, load_error, provider: None })
}

/// Abre o arquivo de troféus do jogo no editor padrão (cria um modelo se não existir).
pub fn edit(app: &AppHandle, state: &AppState, game_id: &str) -> Result<()> {
    state.game(game_id)?;
    let path = definitions_path(&state.data_dir, game_id);
    if !path.exists() {
        write_template(&path, game_id)?;
    }
    app.opener()
        .open_path(path.to_string_lossy(), None::<&str>)
        .map_err(|e| Error::Io(std::io::Error::other(e.to_string())))
}

pub fn definitions_path(data_dir: &Path, game_id: &str) -> PathBuf {
    data_dir.join("trophies").join(format!("{game_id}.json"))
}

pub fn load(path: &Path) -> Result<Option<TrophySet>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Arquivo inicial para o usuário começar a escrever troféus.
pub fn write_template(path: &Path, game_id: &str) -> Result<()> {
    let template = TrophySet {
        game_id: game_id.to_string(),
        trophies: vec![Trophy {
            id: 1,
            title: "Exemplo".into(),
            description: "Troca as condições por um endereço achado com o rdram_probe".into(),
            points: 5,
            conditions: "0xH000000=1".into(),
        }],
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, serde_json::to_vec_pretty(&template)?)?;
    Ok(())
}

/// Erros de sintaxe por troféu, para mostrar na interface.
pub fn validate(set: &TrophySet) -> BTreeMap<u32, String> {
    let mut runtime = recomphub_achievements::Runtime::new();
    set.trophies
        .iter()
        .filter_map(|t| {
            runtime
                .activate(t.id, &t.conditions)
                .err()
                .map(|e| (t.id, e.to_string()))
        })
        .collect()
}

/// Acompanha o processo `pid` até `alive` virar falso. Chama `on_unlock` a
/// cada troféu desbloqueado (fora os de `already_unlocked`).
#[cfg(any(windows, target_os = "linux"))]
pub fn watch(
    pid: u32,
    alive: Arc<AtomicBool>,
    set: TrophySet,
    already_unlocked: HashSet<u32>,
    on_unlock: impl Fn(&Trophy),
) {
    use std::time::Duration;

    use recomphub_achievements::{Event, Runtime};
    use recomphub_memory::external::ExternalMemory;

    let pending: Vec<&Trophy> = set
        .trophies
        .iter()
        .filter(|t| !already_unlocked.contains(&t.id))
        .collect();
    if pending.is_empty() {
        return;
    }

    // A RDRAM é alocada logo que a recomp abre; tenta até conseguir.
    let mut memory = loop {
        if !alive.load(Ordering::Relaxed) {
            return;
        }
        match ExternalMemory::attach(pid) {
            Ok(m) => break m,
            Err(_) => std::thread::sleep(Duration::from_secs(1)),
        }
    };

    let mut runtime = Runtime::new();
    for trophy in &pending {
        // Condições inválidas são ignoradas aqui e mostradas pela `validate`.
        let _ = runtime.activate(trophy.id, &trophy.conditions);
    }

    while alive.load(Ordering::Relaxed) {
        for event in runtime.do_frame(&mut memory) {
            if let Event::Triggered(id) = event {
                if let Some(trophy) = set.trophies.iter().find(|t| t.id == id) {
                    on_unlock(trophy);
                }
            }
        }
        std::thread::sleep(Duration::from_millis(16));
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn watch(_: u32, _: Arc<AtomicBool>, _: TrophySet, _: HashSet<u32>, _: impl Fn(&Trophy)) {}

// ---------- Conquistas detectadas pelo próprio jogo (ex.: Dusklight) ----------

pub use recomphub_catalog::{ExternalTrophy, TrophyProvider};

/// Progresso de uma conquista, como o jogo grava.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct ProviderEntry {
    #[serde(default)]
    /// Contadores (ex.: 12/60); ainda não exibido.
    #[allow(dead_code)]
    pub progress: i64,
    #[serde(default)]
    pub unlocked: bool,
}

pub type ProviderProgress = std::collections::HashMap<String, ProviderEntry>;

/// Expande `%APPDATA%`, `%LOCALAPPDATA%` e `~` no caminho do catálogo.
pub fn expand_path(raw: &str) -> PathBuf {
    let mut s = raw.to_string();
    for var in ["APPDATA", "LOCALAPPDATA", "USERPROFILE"] {
        let token = format!("%{var}%");
        if s.contains(&token) {
            if let Ok(value) = std::env::var(var) {
                s = s.replace(&token, &value);
            }
        }
    }
    if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(s)
}

/// Arquivo de progresso para o sistema atual.
pub fn provider_progress_path(provider: &TrophyProvider) -> Option<PathBuf> {
    provider
        .progress_file
        .get(std::env::consts::OS)
        .map(|p| expand_path(p))
}

/// Arquivo ainda inexistente (jogo nunca aberto) = nenhum progresso.
pub fn read_provider_progress(path: &Path) -> Result<ProviderProgress> {
    match std::fs::read(path) {
        // Tolera BOM (editores e o PowerShell 5 gravam UTF-8 com BOM).
        Ok(bytes) => Ok(serde_json::from_slice(
            bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes),
        )?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(ProviderProgress::new()),
        Err(e) => Err(e.into()),
    }
}

/// Acompanha o arquivo de progresso até `alive` virar falso e chama
/// `on_unlock` para cada conquista que passar a desbloqueada nesse período.
/// O Dusklight grava o arquivo no mesmo frame do desbloqueio.
pub fn watch_provider(
    path: PathBuf,
    alive: Arc<AtomicBool>,
    trophies: Vec<ExternalTrophy>,
    on_unlock: impl Fn(&ExternalTrophy),
) {
    use std::time::{Duration, SystemTime};

    let unlocked_now = |progress: &ProviderProgress| -> HashSet<String> {
        progress
            .iter()
            .filter(|(_, e)| e.unlocked)
            .map(|(k, _)| k.clone())
            .collect()
    };
    let modified = |p: &Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();

    // O que já estava desbloqueado antes desta sessão não gera notificação.
    let mut seen = read_provider_progress(&path)
        .map(|p| unlocked_now(&p))
        .unwrap_or_default();
    let mut last_modified: Option<SystemTime> = modified(&path);

    while alive.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(500));
        let current = modified(&path);
        if current.is_none() || current == last_modified {
            continue;
        }
        last_modified = current;
        // O jogo pode estar no meio da escrita; tenta de novo na próxima volta.
        let Ok(progress) = read_provider_progress(&path) else {
            last_modified = None;
            continue;
        };
        for key in unlocked_now(&progress) {
            if seen.insert(key.clone()) {
                if let Some(trophy) = trophies.iter().find(|t| t.key == key) {
                    on_unlock(trophy);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_is_valid_and_loads_back() {
        let dir = std::env::temp_dir().join(format!("recomphub-trophies-{}", std::process::id()));
        let path = definitions_path(&dir, "jogo");
        write_template(&path, "jogo").unwrap();
        let set = load(&path).unwrap().unwrap();
        assert_eq!(set.game_id, "jogo");
        assert!(validate(&set).is_empty());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn expand_path_replaces_appdata() {
        // SAFETY: teste isolado; só esta thread lê a variável.
        unsafe { std::env::set_var("APPDATA", "C:/Users/x/AppData/Roaming") };
        assert_eq!(
            expand_path("%APPDATA%/TwilitRealm/Dusklight/achievements.json"),
            PathBuf::from("C:/Users/x/AppData/Roaming/TwilitRealm/Dusklight/achievements.json")
        );
    }

    /// Simula o Dusklight gravando o achievements.json durante a sessão.
    #[test]
    fn watch_provider_reports_only_new_unlocks() {
        let dir = std::env::temp_dir().join(format!("recomphub-provider-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("achievements.json");
        std::fs::write(
            &path,
            br#"{"antiga":{"progress":1,"unlocked":true},"nova":{"progress":0,"unlocked":false}}"#,
        )
        .unwrap();

        let def = |key: &str| ExternalTrophy {
            key: key.into(),
            title: key.into(),
            description: String::new(),
            category: String::new(),
            goal: 0,
        };
        let alive = Arc::new(AtomicBool::new(true));
        let unlocked = Arc::new(std::sync::Mutex::new(Vec::new()));
        let handle = {
            let (path, alive, unlocked) = (path.clone(), alive.clone(), unlocked.clone());
            std::thread::spawn(move || {
                watch_provider(path, alive, vec![def("antiga"), def("nova")], |t| {
                    unlocked.lock().unwrap().push(t.key.clone())
                })
            })
        };

        std::thread::sleep(std::time::Duration::from_millis(700));
        std::fs::write(
            &path,
            br#"{"antiga":{"progress":1,"unlocked":true},"nova":{"progress":1,"unlocked":true}}"#,
        )
        .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1500));
        alive.store(false, Ordering::Relaxed);
        handle.join().unwrap();

        assert_eq!(*unlocked.lock().unwrap(), vec!["nova".to_string()]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn validate_reports_bad_conditions() {
        let set = TrophySet {
            game_id: "x".into(),
            trophies: vec![Trophy {
                id: 7,
                title: "t".into(),
                description: "d".into(),
                points: 0,
                conditions: "nada".into(),
            }],
        };
        assert!(validate(&set).contains_key(&7));
    }
}
