//! Dados do jogo original no IGDB (https://api-docs.igdb.com).
//! Credenciais: um app em https://dev.twitch.tv/console (client id + secret).

use anyhow::{Context, Result};
use chrono::DateTime;
use recomphub_catalog::GameInfo;
use serde::Deserialize;

const API: &str = "https://api.igdb.com/v4";
const FIELDS: &str = "fields name,first_release_date,summary,total_rating,url,genres.name,\
    screenshots.image_id,involved_companies.developer,involved_companies.publisher,\
    involved_companies.company.name;";
const MAX_SCREENSHOTS: usize = 6;

pub struct Igdb {
    http: reqwest::Client,
    client_id: String,
    token: String,
}

#[derive(Deserialize)]
struct Token {
    access_token: String,
}

#[derive(Deserialize)]
struct Game {
    id: u64,
    name: String,
    first_release_date: Option<i64>,
    summary: Option<String>,
    total_rating: Option<f64>,
    url: Option<String>,
    #[serde(default)]
    genres: Vec<Named>,
    #[serde(default)]
    screenshots: Vec<Screenshot>,
    #[serde(default)]
    involved_companies: Vec<InvolvedCompany>,
}

#[derive(Deserialize)]
struct Named {
    name: String,
}

#[derive(Deserialize)]
struct Screenshot {
    image_id: String,
}

#[derive(Deserialize)]
struct InvolvedCompany {
    company: Named,
    developer: bool,
    publisher: bool,
}

/// IDs de plataforma do IGDB, para a busca não pegar remakes de outro console.
fn platform_id(platform: &str) -> Option<u32> {
    Some(match platform.to_ascii_uppercase().as_str() {
        "N64" => 4,
        "GC" | "GAMECUBE" => 21,
        "WII" => 5,
        "PS1" | "PSX" => 7,
        "PS2" => 8,
        "SNES" => 19,
        "NES" => 18,
        "GBA" => 24,
        "NDS" | "DS" => 20,
        "XBOX" => 11,
        _ => return None,
    })
}

impl Igdb {
    pub async fn connect(http: reqwest::Client, client_id: String, secret: &str) -> Result<Self> {
        let token: Token = http
            .post("https://id.twitch.tv/oauth2/token")
            .query(&[
                ("client_id", client_id.as_str()),
                ("client_secret", secret),
                ("grant_type", "client_credentials"),
            ])
            .send()
            .await?
            .error_for_status()
            .context("autenticação na Twitch")?
            .json()
            .await?;
        Ok(Self { http, client_id, token: token.access_token })
    }

    async fn query(&self, body: String) -> Result<Vec<Game>> {
        Ok(self
            .http
            .post(format!("{API}/games"))
            .header("Client-ID", &self.client_id)
            .bearer_auth(&self.token)
            .body(body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    /// Por id quando fixado no `sources.json`; senão busca pelo título.
    /// Retorna `(id, dados)`.
    pub async fn game(
        &self,
        id: Option<u64>,
        title: &str,
        platform: &str,
    ) -> Result<Option<(u64, GameInfo)>> {
        let filter = match (id, platform_id(platform)) {
            (Some(id), _) => format!("where id = {id};"),
            (None, Some(p)) => format!("search \"{}\"; where platforms = ({p});", escape(title)),
            (None, None) => format!("search \"{}\";", escape(title)),
        };
        let games = self.query(format!("{FIELDS} {filter} limit 1;")).await?;
        Ok(games.into_iter().next().map(|g| (g.id, convert(g))))
    }
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn convert(g: Game) -> GameInfo {
    let companies = |pick: fn(&InvolvedCompany) -> bool| -> Vec<String> {
        g.involved_companies
            .iter()
            .filter(|c| pick(c))
            .map(|c| c.company.name.clone())
            .collect()
    };
    GameInfo {
        developers: companies(|c| c.developer),
        publishers: companies(|c| c.publisher),
        title: g.name,
        release_date: g
            .first_release_date
            .and_then(|t| DateTime::from_timestamp(t, 0))
            .map(|d| d.date_naive()),
        genres: g.genres.into_iter().map(|n| n.name).collect(),
        summary: g.summary,
        rating: g.total_rating.map(|r| (r * 10.0).round() / 10.0),
        url: g.url,
        screenshots: g
            .screenshots
            .into_iter()
            .take(MAX_SCREENSHOTS)
            .map(|s| format!("https://images.igdb.com/igdb/image/upload/t_screenshot_big/{}.jpg", s.image_id))
            .collect(),
    }
}
