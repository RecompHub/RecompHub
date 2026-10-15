//! Catálogo de jogos gerado pelo `catalog-builder` (`catalog/games.json`).
//!
//! Ordem de preferência: catálogo remoto (cacheado em disco) → cópia embutida
//! no binário, que garante que o launcher funcione offline na primeira vez.

use std::path::Path;

use reqwest::{StatusCode, header};
use serde::{Deserialize, Serialize};

pub use recomphub_catalog::{Catalog, CatalogGame as Game, PlatformAsset, SCHEMA_VERSION};

use crate::error::{Error, Result};

const BUNDLED_CATALOG: &str = include_str!("../../../catalog/games.json");

pub fn bundled() -> Result<Catalog> {
    Ok(serde_json::from_str(BUNDLED_CATALOG)?)
}

#[derive(Serialize, Deserialize)]
struct Cached {
    etag: Option<String>,
    catalog: Catalog,
}

fn read_cache(path: &Path) -> Option<Cached> {
    let cached: Cached = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    (cached.catalog.schema_version == SCHEMA_VERSION).then_some(cached)
}

/// O mais recente entre o cache e o embutido (o embutido pode ser mais novo
/// logo depois de uma atualização do launcher).
pub fn load(cache_path: &Path) -> Result<Catalog> {
    let bundled = bundled()?;
    Ok(match read_cache(cache_path) {
        Some(c) if c.catalog.generated_at > bundled.generated_at => c.catalog,
        _ => bundled,
    })
}

/// Baixa o catálogo publicado; usa `If-None-Match` para não baixar de novo
/// um arquivo que não mudou.
pub async fn fetch_remote(http: &reqwest::Client, cache_path: &Path) -> Result<Catalog> {
    let cached = read_cache(cache_path);
    let url = format!("{}/games.json", recomphub_catalog::DEFAULT_BASE_URL);
    let mut request = http.get(url);
    if let Some(etag) = cached.as_ref().and_then(|c| c.etag.as_deref()) {
        request = request.header(header::IF_NONE_MATCH, etag);
    }
    let response = request.send().await?;

    if response.status() == StatusCode::NOT_MODIFIED {
        if let Some(c) = cached {
            return Ok(c.catalog);
        }
    }
    if !response.status().is_success() {
        return Err(Error::CatalogStatus(response.status().as_u16()));
    }
    let etag = response
        .headers()
        .get(header::ETAG)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let catalog: Catalog = serde_json::from_slice(&response.bytes().await?)?;
    if catalog.schema_version != SCHEMA_VERSION {
        return Err(Error::CatalogSchema(catalog.schema_version));
    }

    if let Some(dir) = cache_path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(
        cache_path,
        serde_json::to_vec(&Cached {
            etag,
            catalog: catalog.clone(),
        })?,
    )?;
    Ok(catalog)
}

/// Plataforma atual no formato usado pelas chaves de `Game::platforms`.
pub fn current_platform() -> String {
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        other => other,
    };
    format!("{}-{arch}", std::env::consts::OS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_catalog_parses() {
        let catalog = bundled().expect("catalog/games.json inválido");
        assert_eq!(catalog.schema_version, SCHEMA_VERSION);
        assert!(!catalog.games.is_empty());
    }
}
