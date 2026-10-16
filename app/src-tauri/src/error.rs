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

// Os comandos do Tauri devolvem o erro ao frontend como texto.
impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
