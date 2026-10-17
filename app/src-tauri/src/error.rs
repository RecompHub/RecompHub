use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("jogo desconhecido: {0}")]
    UnknownGame(String),
    #[error("jogo não instalado: {0}")]
    NotInstalled(String),
    #[error("{0} não tem build para esta plataforma ({1})")]
    UnsupportedPlatform(String, String),
    #[error("nenhum arquivo da release corresponde a {0}")]
    AssetNotFound(String),
    #[error("executável {0} não encontrado no pacote")]
    ExecutableNotFound(String),
    #[error("já existe uma operação em andamento para {0}")]
    Busy(String),
    #[error("o jogo está em execução")]
    Running,
    #[error("desinstale o jogo antes de removê-lo da biblioteca")]
    StillInstalled,
    #[error("não é possível instalar jogos em {0}")]
    InvalidGamesDir(String),
    #[error("o instalador do projeto falhou: {0}")]
    InstallerFailed(String),
    #[error("o instalador terminou, mas o jogo não foi encontrado em {0}")]
    InstallerNoGame(String),
    #[error("escolha a imagem do disco do jogo para instalar")]
    RomRequired,
    #[error("GitHub respondeu {0}")]
    GitHubStatus(u16),
    #[error("catálogo remoto indisponível (HTTP {0})")]
    CatalogStatus(u16),
    #[error("catálogo remoto usa um formato mais novo (v{0}); atualize o launcher")]
    CatalogSchema(u32),
    #[error("falha de rede: {0}")]
    Http(#[from] reqwest::Error),
    #[error("arquivo zip inválido: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("JSON inválido: {0}")]
    Json(#[from] serde_json::Error),
    #[error("padrão inválido: {0}")]
    Glob(#[from] globset::Error),
    #[error("erro de E/S: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Tauri(#[from] tauri::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Idioma das mensagens enviadas ao frontend (o `Display` fica em português,
/// para os logs). Definido pelas configurações do usuário.
static ENGLISH: AtomicBool = AtomicBool::new(false);

pub fn set_english(on: bool) {
    ENGLISH.store(on, Ordering::Relaxed);
}

impl Error {
    fn english(&self) -> String {
        match self {
            Error::UnknownGame(id) => format!("unknown game: {id}"),
            Error::NotInstalled(id) => format!("game not installed: {id}"),
            Error::UnsupportedPlatform(id, p) => format!("{id} has no build for this platform ({p})"),
            Error::AssetNotFound(p) => format!("no release file matches {p}"),
            Error::ExecutableNotFound(p) => format!("executable {p} not found in the package"),
            Error::Busy(id) => format!("an operation is already in progress for {id}"),
            Error::Running => "the game is running".into(),
            Error::StillInstalled => "uninstall the game before removing it from the library".into(),
            Error::InvalidGamesDir(d) => format!("games cannot be installed in {d}"),
            Error::InstallerFailed(m) => format!("the project's installer failed: {}", m.replace("código", "code").replace("detalhes em", "details in")),
            Error::InstallerNoGame(d) => format!("the installer finished, but the game was not found in {d}"),
            Error::RomRequired => "choose the game's disc image to install".into(),
            Error::GitHubStatus(code) => format!("GitHub responded {code}"),
            Error::CatalogStatus(code) => format!("remote catalog unavailable (HTTP {code})"),
            Error::CatalogSchema(v) => format!("remote catalog uses a newer format (v{v}); update the launcher"),
            Error::Http(e) => format!("network error: {e}"),
            Error::Zip(e) => format!("invalid zip file: {e}"),
            Error::Json(e) => format!("invalid JSON: {e}"),
            Error::Glob(e) => format!("invalid pattern: {e}"),
            Error::Io(e) => format!("I/O error: {e}"),
            Error::Tauri(e) => e.to_string(),
        }
    }
}

// Os comandos do Tauri devolvem o erro ao frontend como texto.
impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        if ENGLISH.load(Ordering::Relaxed) {
            serializer.serialize_str(&self.english())
        } else {
            serializer.serialize_str(&self.to_string())
        }
    }
}
