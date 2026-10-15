//! Cliente da API de releases do GitHub, com cache em disco.
//!
//! Sem autenticação o limite é ~60 req/hora por IP. Para economizar:
//! - respostas recentes (< `FRESH_FOR`) saem direto do cache;
//! - depois disso, a revalidação usa `If-None-Match` (ETag); um 304 não conta
//!   no limite;
//! - se a rede falhar ou o limite estourar, usa o cache antigo.

use std::path::PathBuf;

use chrono::{DateTime, Duration, Utc};
use globset::GlobBuilder;
use reqwest::{StatusCode, header};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

const API: &str = "https://api.github.com";
const FRESH_FOR: Duration = Duration::minutes(10);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    #[serde(alias = "tag_name")]
    pub tag_name: String,
    pub name: Option<String>,
    #[serde(alias = "published_at")]
    pub published_at: Option<DateTime<Utc>>,
    pub body: Option<String>,
    #[serde(alias = "html_url")]
    pub html_url: String,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub name: String,
    #[serde(alias = "browser_download_url")]
    pub browser_download_url: String,
    pub size: u64,
}

impl Release {
    /// Primeiro asset cujo nome casa com o glob (sem diferenciar maiúsculas).
    pub fn find_asset(&self, pattern: &str) -> Result<&Asset> {
        let glob = GlobBuilder::new(pattern)
            .case_insensitive(true)
            .build()?
            .compile_matcher();
        self.assets
            .iter()
            .find(|a| glob.is_match(&a.name))
            .ok_or_else(|| Error::AssetNotFound(pattern.to_string()))
    }
}

#[derive(Serialize, Deserialize)]
struct CachedRelease {
    etag: Option<String>,
    fetched_at: DateTime<Utc>,
    release: Release,
}

pub struct GitHub {
    http: reqwest::Client,
    cache_dir: PathBuf,
}

impl GitHub {
    pub fn new(http: reqwest::Client, cache_dir: PathBuf) -> Self {
        Self { http, cache_dir }
    }

    pub async fn latest_release(&self, repo: &str) -> Result<Release> {
        let cache_path = self
            .cache_dir
            .join(format!("{}.json", repo.replace('/', "__")));
        let cached: Option<CachedRelease> = std::fs::read(&cache_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok());

        if let Some(c) = &cached {
            if Utc::now() - c.fetched_at < FRESH_FOR {
                return Ok(c.release.clone());
            }
        }

        let mut request = self
            .http
            .get(format!("{API}/repos/{repo}/releases/latest"))
            .header(header::ACCEPT, "application/vnd.github+json");
        if let Some(etag) = cached.as_ref().and_then(|c| c.etag.as_deref()) {
            request = request.header(header::IF_NONE_MATCH, etag);
        }

        let response = match request.send().await {
            Ok(r) => r,
            Err(e) => return cached.map(|c| c.release).ok_or(e.into()),
        };

        let status = response.status();
        if status == StatusCode::NOT_MODIFIED {
            if let Some(mut c) = cached {
                c.fetched_at = Utc::now();
                self.write_cache(&cache_path, &c);
                return Ok(c.release);
            }
        }
        if !status.is_success() {
            return cached
                .map(|c| c.release)
                .ok_or(Error::GitHubStatus(status.as_u16()));
        }

        let etag = response
            .headers()
            .get(header::ETAG)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        let release: Release = response.json().await?;
        self.write_cache(
            &cache_path,
            &CachedRelease {
                etag,
                fetched_at: Utc::now(),
                release: release.clone(),
            },
        );
        Ok(release)
    }

    // Falhar ao gravar o cache não deve impedir o uso da resposta.
    fn write_cache(&self, path: &std::path::Path, entry: &CachedRelease) {
        let _ = std::fs::create_dir_all(&self.cache_dir);
        if let Ok(bytes) = serde_json::to_vec(entry) {
            let _ = std::fs::write(path, bytes);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(names: &[&str]) -> Release {
        Release {
            tag_name: "v1".into(),
            name: None,
            published_at: None,
            body: None,
            html_url: String::new(),
            assets: names
                .iter()
                .map(|n| Asset {
                    name: n.to_string(),
                    browser_download_url: String::new(),
                    size: 0,
                })
                .collect(),
        }
    }

    #[test]
    fn find_asset_matches_platform_pattern() {
        let r = release(&[
            "Zelda64Recompiled-v1.2.2-Linux-X64.zip",
            "Zelda64Recompiled-v1.2.2-Windows.zip",
        ]);
        let asset = r.find_asset("*-windows.zip").unwrap();
        assert_eq!(asset.name, "Zelda64Recompiled-v1.2.2-Windows.zip");
        assert!(r.find_asset("*-macOS.zip").is_err());
    }

    #[test]
    fn release_reads_github_and_cache_formats() {
        let from_api = r#"{"tag_name":"v1","name":null,"published_at":null,"body":null,
            "html_url":"u","assets":[{"name":"a.zip","browser_download_url":"d","size":1}]}"#;
        let r: Release = serde_json::from_str(from_api).unwrap();
        let roundtrip: Release = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(roundtrip.assets[0].browser_download_url, "d");
    }
}
