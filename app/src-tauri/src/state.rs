//! Biblioteca local do usuário (`library.json` na pasta de dados do app).
//!
//! Um jogo pode estar na biblioteca sem estar instalado. O launcher confia
//! neste arquivo para saber a versão instalada, não em metadados do executável.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryEntry {
    pub added_at: DateTime<Local>,
    #[serde(default)]
    pub installed: Option<InstalledGame>,
    /// Fica na entrada (não na instalação) para sobreviver a reinstalações.
    #[serde(default)]
    pub play_time_seconds: u64,
    #[serde(default)]
    pub last_played_at: Option<DateTime<Local>>,
    /// Troféus desbloqueados enquanto o launcher acompanhava: chave → quando.
    /// (Os do RA usam o id numérico como chave.)
    #[serde(default)]
    pub trophies: BTreeMap<String, DateTime<Local>>,
}

impl LibraryEntry {
    pub fn new() -> Self {
        Self {
            added_at: Local::now(),
            installed: None,
            play_time_seconds: 0,
            last_played_at: None,
            trophies: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledGame {
    pub installed_tag: String,
    pub installed_at: DateTime<Local>,
    pub install_path: PathBuf,
    /// Caminho do executável relativo a `install_path`.
    pub executable: PathBuf,
    /// SHA-256 do executável: detecta alteração manual e será a chave dos
    /// offsets de memória na fase 1 das conquistas.
    pub executable_hash: String,
    /// Instalação feita fora do launcher e só registrada nele: desinstalar
    /// apenas esquece o jogo, sem apagar os arquivos do usuário.
    #[serde(default)]
    pub imported: bool,
    /// Instalado pelo instalador do próprio projeto (ver `setup.rs`): os
    /// arquivos são dele, então remover do launcher não apaga nada.
    #[serde(default)]
    pub external: bool,
}

pub type Library = HashMap<String, LibraryEntry>;

pub fn load(path: &Path) -> Result<Library> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Library::new()),
        Err(e) => Err(e.into()),
    }
}

/// Grava num arquivo temporário e renomeia, para não corromper a biblioteca
/// se o launcher fechar no meio da escrita.
pub fn save(path: &Path, library: &Library) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(library)?)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}
