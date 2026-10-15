//! Imagens do SteamGridDB (https://www.steamgriddb.com/api/v2).
//! Chave em https://www.steamgriddb.com/profile/preferences/api

use anyhow::{Context, Result};
use recomphub_catalog::Images;
use serde::Deserialize;

const API: &str = "https://www.steamgriddb.com/api/v2";

pub struct SteamGridDb {
    http: reqwest::Client,
    api_key: String,
}

#[derive(Deserialize)]
struct Response<T> {
    data: T,
}

#[derive(Deserialize)]
struct SearchResult {
    id: u64,
    name: String,
}

#[derive(Deserialize)]
struct Image {
    url: String,
}

impl SteamGridDb {
    pub fn new(http: reqwest::Client, api_key: String) -> Self {
        Self { http, api_key }
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, url: reqwest::Url) -> Result<T> {
        let response: Response<T> = self
            .http
            .get(url)
            .bearer_auth(&self.api_key)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(response.data)
    }

    /// Primeiro resultado da busca: `(id, nome no SGDB)`.
    pub async fn search(&self, title: &str) -> Result<Option<(u64, String)>> {
        let mut url = reqwest::Url::parse(&format!("{API}/search/autocomplete"))?;
        url.path_segments_mut()
            .map_err(|_| anyhow::anyhow!("URL base inválida"))?
            .push(title);
        let results: Vec<SearchResult> = self.get(url).await?;
        Ok(results.into_iter().next().map(|r| (r.id, r.name)))
    }

    /// A API já ordena por nota da comunidade; pega a melhor imagem estática
    /// de cada tipo, sem NSFW nem humor.
    pub async fn images(&self, game_id: u64) -> Result<Images> {
        Ok(Images {
            grid: self.best(game_id, "grids", "&dimensions=600x900").await?,
            hero: self.best(game_id, "heroes", "").await?,
            logo: self.best(game_id, "logos", "").await?,
            icon: self.best(game_id, "icons", "").await?,
        })
    }

    async fn best(&self, game_id: u64, kind: &str, extra: &str) -> Result<Option<String>> {
        let url = format!("{API}/{kind}/game/{game_id}?types=static&nsfw=false&humor=false{extra}");
        let images: Vec<Image> = self
            .get(reqwest::Url::parse(&url)?)
            .await
            .with_context(|| format!("SGDB {kind} do jogo {game_id}"))?;
        Ok(images.into_iter().next().map(|i| i.url))
    }
}
