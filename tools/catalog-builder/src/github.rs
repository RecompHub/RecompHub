//! Dados do projeto recomp: repositório e releases.

use anyhow::Result;
use chrono::{DateTime, Utc};
use recomphub_catalog::{RecompInfo, ReleaseSummary};
use serde::Deserialize;

const API: &str = "https://api.github.com";

pub struct GitHub {
    http: reqwest::Client,
    /// `GITHUB_TOKEN` sobe o limite de 60 para 5000 req/hora.
    token: Option<String>,
}

#[derive(Deserialize)]
struct Repo {
    description: Option<String>,
    homepage: Option<String>,
    license: Option<License>,
    #[serde(default)]
    topics: Vec<String>,
    stargazers_count: u64,
}

#[derive(Deserialize)]
struct License {
    spdx_id: Option<String>,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    name: Option<String>,
    published_at: Option<DateTime<Utc>>,
    html_url: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    download_count: u64,
}

impl GitHub {
    pub fn new(http: reqwest::Client, token: Option<String>) -> Self {
        Self { http, token }
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        let mut request = self
            .http
            .get(format!("{API}{path}"))
            .header("Accept", "application/vnd.github+json");
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }
        Ok(request.send().await?.error_for_status()?.json().await?)
    }
}

/// Dados do recomp + nomes dos assets da release que o launcher instala.
pub struct RecompData {
    pub info: RecompInfo,
    pub latest_assets: Vec<String>,
}

impl GitHub {
    pub async fn recomp_info(&self, repo: &str) -> Result<RecompData> {
        let info: Repo = self.get(&format!("/repos/{repo}")).await?;

        let mut releases: Vec<Release> = Vec::new();
        for page in 1.. {
            let batch: Vec<Release> = self
                .get(&format!("/repos/{repo}/releases?per_page=100&page={page}"))
                .await?;
            let done = batch.len() < 100;
            releases.extend(batch.into_iter().filter(|r| !r.draft));
            if done {
                break;
            }
        }

        // A API devolve da mais nova para a mais antiga.
        let latest_release = releases.iter().find(|r| !r.prerelease).or(releases.first());
        let latest_assets = latest_release
            .map(|r| r.assets.iter().map(|a| a.name.clone()).collect())
            .unwrap_or_default();
        let latest = latest_release.map(|r| ReleaseSummary {
                tag: r.tag_name.clone(),
                name: r.name.clone(),
                published_at: r.published_at,
                html_url: r.html_url.clone(),
            });

        let info = RecompInfo {
            description: info.description,
            homepage: info.homepage.filter(|h| !h.is_empty()),
            license: info
                .license
                .and_then(|l| l.spdx_id)
                .filter(|id| id != "NOASSERTION"),
            topics: info.topics,
            stars: info.stargazers_count,
            first_release_at: releases.iter().filter_map(|r| r.published_at).min(),
            latest_release: latest,
            release_count: releases.len() as u32,
            total_downloads: releases
                .iter()
                .flat_map(|r| &r.assets)
                .map(|a| a.download_count)
                .sum(),
        };
        Ok(RecompData { info, latest_assets })
    }
}
