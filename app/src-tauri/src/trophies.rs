//! Troféus durante o jogo (Fase 1): lê a RDRAM do processo da recomp de fora
//! e avalia as condições com a rcheevos (~60 vezes por segundo).
//!
//! As definições ficam em `<pasta de dados>/trophies/<id do jogo>.json`,
//! editáveis pelo usuário enquanto não existe servidor.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub use recomphub_catalog::{Trophy, TrophySet};

use crate::error::Result;

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
        .filter_map(|t| runtime.activate(t.id, &t.conditions).err().map(|e| (t.id, e.to_string())))
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
