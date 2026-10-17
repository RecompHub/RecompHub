//! Dados do jogo original no IGDB (https://api-docs.igdb.com).
//! Credenciais: um app em https://dev.twitch.tv/console (client id + secret).

use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use reqwest::StatusCode;
use chrono::DateTime;
use recomphub_catalog::GameInfo;
use serde::Deserialize;

const API: &str = "https://api.igdb.com/v4";
const FIELDS: &str = "fields name,first_release_date,summary,total_rating,url,genres.name,\
    screenshots.image_id,involved_companies.developer,involved_companies.publisher,\
    involved_companies.company.name;";
const MAX_SCREENSHOTS: usize = 6;

/// 4 req/s no máximo, com folga.
const MIN_INTERVAL: Duration = Duration::from_millis(260);

pub struct Igdb {
    http: reqwest::Client,
    client_id: String,
    secret: String,
    token: tokio::sync::Mutex<String>,
    last_request: tokio::sync::Mutex<Option<Instant>>,
}

/// Token de app (client credentials). Dura ~60 dias; o builder roda e termina,
/// então basta um por execução (e renovar se vier 401).
async fn fetch_token(http: &reqwest::Client, client_id: &str, secret: &str) -> Result<String> {
    let token: Token = http
        .post("https://id.twitch.tv/oauth2/token")
        .query(&[
            ("client_id", client_id),
            ("client_secret", secret),
            ("grant_type", "client_credentials"),
        ])
        .send()
        .await?
        .error_for_status()
        .context("autenticação na Twitch")?
        .json()
        .await?;
    Ok(token.access_token)
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
    pub async fn connect(http: reqwest::Client, client_id: String, secret: String) -> Result<Self> {
        let token = fetch_token(&http, &client_id, &secret).await?;
        Ok(Self {
            http,
            client_id,
            secret,
            token: tokio::sync::Mutex::new(token),
            last_request: tokio::sync::Mutex::new(None),
        })
    }

    /// Respeita o limite do IGDB (4 req/s), tenta de novo com espera crescente
    /// em 429 e renova o token uma vez em 401. Nunca registra token nem segredo.
    async fn query(&self, body: String) -> Result<Vec<Game>> {
        let mut renewed = false;
        let mut backoff = Duration::from_millis(500);
        for _ in 0..6 {
            {
                let mut last = self.last_request.lock().await;
                if let Some(t) = *last {
                    let wait = MIN_INTERVAL.saturating_sub(t.elapsed());
                    tokio::time::sleep(wait).await;
                }
                *last = Some(Instant::now());
            }
            let token = self.token.lock().await.clone();
            let response = self
                .http
                .post(format!("{API}/games"))
                .header("Client-ID", &self.client_id)
                .bearer_auth(token)
                .body(body.clone())
                .send()
                .await?;
            match response.status() {
                StatusCode::TOO_MANY_REQUESTS => {
                    tokio::time::sleep(backoff).await;
                    backoff *= 2;
                }
                StatusCode::UNAUTHORIZED if !renewed => {
                    renewed = true;
                    *self.token.lock().await = fetch_token(&self.http, &self.client_id, &self.secret).await?;
                }
                _ => return Ok(response.error_for_status()?.json().await?),
            }
        }
        anyhow::bail!("IGDB continuou recusando a consulta (limite de requisições)")
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
