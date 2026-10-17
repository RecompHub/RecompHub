//! Tipos do catálogo, compartilhados entre o launcher e o `catalog-builder`.
//!
//! - [`Sources`] (`catalog/sources.json`): editado à mão; só o que não dá
//!   para descobrir automaticamente.
//! - [`Catalog`] (`catalog/games.json`): gerado pelo builder a partir das
//!   fontes + GitHub, SteamGridDB e IGDB. É o que o launcher baixa.

use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 2;

/// Onde o catálogo gerado é publicado (pasta `catalog/` do repositório).
/// Imagens locais em `catalog/images/` também são servidas daqui.
pub const DEFAULT_BASE_URL: &str =
    "https://raw.githubusercontent.com/RecompHub/RecompHub/main/catalog";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sources {
    pub games: Vec<SourceGame>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceGame {
    pub id: String,
    pub name: String,
    /// `owner/repo` no GitHub.
    pub repo: String,
    /// Título do jogo original, usado na busca no SteamGridDB e no IGDB.
    pub original_title: String,
    /// Console original, ex.: `N64`.
    pub original_platform: String,
    /// Chave no formato `{os}-{arch}`, ex.: `windows-x64`.
    pub platforms: BTreeMap<String, PlatformAsset>,
    pub required_rom: RequiredRom,
    #[serde(default)]
    pub achievements: AchievementsInfo,
    /// Fixa o jogo no SteamGridDB; sem ele o builder busca pelo nome.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sgdb_id: Option<u64>,
    /// Fixa o jogo no IGDB; sem ele o builder busca pelo nome.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub igdb_id: Option<u64>,
    /// Imagens que substituem as do SteamGridDB (URL ou caminho em `catalog/images/`).
    #[serde(default, skip_serializing_if = "Images::is_empty")]
    pub images: Images,
    /// Conquistas detectadas pelo próprio jogo (ex.: Dusklight).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trophy_provider: Option<TrophyProvider>,
    /// Projeto feito em grande parte com IA (marcado no awesome-decomp-games);
    /// o launcher mostra um aviso.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ai_assisted: bool,
}

/// Conquistas que o próprio jogo detecta e grava num arquivo de progresso; o
/// launcher só lê esse arquivo. Hoje: `kind = "dusklight"`, que grava
/// `{ "<chave>": { "progress": n, "unlocked": bool } }`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrophyProvider {
    pub kind: String,
    /// Arquivo de progresso por sistema (`windows`, `linux`, `macos`).
    /// `%APPDATA%`, `%LOCALAPPDATA%` e `~` são expandidos pelo launcher.
    pub progress_file: BTreeMap<String, String>,
    /// Só no `sources.json`: caminho (em `catalog/`) com as definições, que o
    /// builder copia para `trophies`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub definitions: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trophies: Vec<ExternalTrophy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalTrophy {
    /// Chave usada pelo jogo no arquivo de progresso.
    pub key: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub category: String,
    /// Meta de um contador (ex.: 60 Poe Souls); 0 = conquista simples.
    #[serde(default)]
    pub goal: u32,
}

/// Formato de `catalog/trophies/<id>.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalTrophyFile {
    #[serde(default)]
    pub source: String,
    pub trophies: Vec<ExternalTrophy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformAsset {
    /// Glob aplicado ao nome do asset da release.
    pub asset_pattern: String,
    /// Nome do executável dentro do pacote (aceita glob).
    pub executable: String,
    /// O asset é o instalador do próprio projeto (ex.: um Setup que compila o
    /// jogo a partir do disco do usuário), não um pacote para extrair.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installer: Option<Installer>,
}

/// Instalador próprio do projeto. O launcher baixa, executa e depois procura
/// `executable` em `install_dir`; os arquivos ficam a cargo do instalador.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Installer {
    /// Onde o instalador coloca o jogo. Aceita `%VARIAVEL%` e `~`.
    pub install_dir: String,
    /// Argumentos. `{rom}` vira a imagem de disco que o usuário escolher.
    /// Sem argumentos, o instalador abre a janela dele.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    /// O instalador escreve o progresso em NDJSON na saída padrão:
    /// `{"type":"progress","percent":n,"message":"..."}` e, no fim,
    /// `{"type":"result","success":bool,"error":"..."}`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub progress_json: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequiredRom {
    pub description: String,
    /// Mesma descrição em inglês, quando a original não serve para os dois idiomas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description_en: Option<String>,
    #[serde(default)]
    pub sha1: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AchievementsInfo {
    pub enabled: bool,
    pub supported_versions: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Images {
    /// Capa vertical 600x900.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid: Option<String>,
    /// Banner largo 1920x620.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hero: Option<String>,
    /// Logo com fundo transparente.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logo: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

impl Images {
    pub fn is_empty(&self) -> bool {
        self == &Images::default()
    }

    /// Campos de `self` têm prioridade; os vazios vêm de `fallback`.
    pub fn or(self, fallback: Images) -> Images {
        Images {
            grid: self.grid.or(fallback.grid),
            hero: self.hero.or(fallback.hero),
            logo: self.logo.or(fallback.logo),
            icon: self.icon.or(fallback.icon),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub schema_version: u32,
    pub generated_at: DateTime<Utc>,
    pub games: Vec<CatalogGame>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogGame {
    pub id: String,
    pub name: String,
    pub repo: String,
    pub original_title: String,
    pub original_platform: String,
    pub platforms: BTreeMap<String, PlatformAsset>,
    pub required_rom: RequiredRom,
    pub achievements: AchievementsInfo,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sgdb_id: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub igdb_id: Option<u64>,
    #[serde(default)]
    pub images: Images,
    /// Dados do projeto recomp (GitHub).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recomp: Option<RecompInfo>,
    /// Dados do jogo original (IGDB).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game: Option<GameInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trophy_provider: Option<TrophyProvider>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ai_assisted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecompInfo {
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub license: Option<String>,
    pub topics: Vec<String>,
    pub stars: u64,
    /// Data da primeira release: o "lançamento" do recomp.
    pub first_release_at: Option<DateTime<Utc>>,
    pub latest_release: Option<ReleaseSummary>,
    pub release_count: u32,
    /// Soma dos downloads de todos os assets de todas as releases.
    pub total_downloads: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseSummary {
    pub tag: String,
    pub name: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub html_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameInfo {
    pub title: String,
    pub release_date: Option<NaiveDate>,
    pub developers: Vec<String>,
    pub publishers: Vec<String>,
    pub genres: Vec<String>,
    pub summary: Option<String>,
    /// Nota agregada do IGDB, 0–100.
    pub rating: Option<f64>,
    pub url: Option<String>,
    pub screenshots: Vec<String>,
}

/// Troféus de um jogo (`<pasta de dados>/trophies/<id>.json` por enquanto;
/// no futuro, servidos junto com o catálogo).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrophySet {
    pub game_id: String,
    pub trophies: Vec<Trophy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Trophy {
    /// Único dentro do jogo; é a chave dos desbloqueios salvos.
    pub id: u32,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub points: u32,
    /// Condições no formato do RetroAchievements (ex.: `"0xH1EF6A0=99"`).
    pub conditions: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn images_override_keeps_fallback_for_missing_fields() {
        let manual = Images { hero: Some("manual".into()), ..Default::default() };
        let sgdb = Images {
            grid: Some("g".into()),
            hero: Some("h".into()),
            ..Default::default()
        };
        let merged = manual.or(sgdb);
        assert_eq!(merged.hero.as_deref(), Some("manual"));
        assert_eq!(merged.grid.as_deref(), Some("g"));
    }

    #[test]
    fn repo_catalog_files_parse() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../catalog");
        let sources = std::fs::read(format!("{dir}/sources.json")).unwrap();
        serde_json::from_slice::<Sources>(&sources).expect("sources.json inválido");
        let games = std::fs::read(format!("{dir}/games.json")).unwrap();
        serde_json::from_slice::<Catalog>(&games).expect("games.json inválido");
    }
}
