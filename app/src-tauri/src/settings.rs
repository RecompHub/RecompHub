//! Preferências do usuário (`settings.json` na pasta de dados do app).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// Pasta onde novos jogos são instalados. `None` = padrão do app.
    #[serde(default)]
    pub games_dir: Option<PathBuf>,
}

pub fn load(path: &Path) -> Result<Settings> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Settings::default()),
        Err(e) => Err(e.into()),
    }
}

pub fn save(path: &Path, settings: &Settings) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(settings)?)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Garante que a pasta existe e aceita escrita, antes de aceitá-la como
/// destino de instalações.
pub fn validate_games_dir(dir: &Path) -> Result<()> {
    if !dir.is_absolute() {
        return Err(Error::InvalidGamesDir(dir.display().to_string()));
    }
    std::fs::create_dir_all(dir)?;
    let probe = dir.join(".recomphub-write-test");
    std::fs::write(&probe, b"ok")
        .map_err(|_| Error::InvalidGamesDir(dir.display().to_string()))?;
    let _ = std::fs::remove_file(probe);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_dir_is_rejected() {
        assert!(validate_games_dir(Path::new("jogos")).is_err());
    }

    #[test]
    fn writable_dir_is_accepted() {
        let dir = std::env::temp_dir().join(format!("recomphub-settings-{}", std::process::id()));
        validate_games_dir(&dir).unwrap();
        assert!(dir.is_dir());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
