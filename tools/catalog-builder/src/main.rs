//! Gera `catalog/games.json` a partir de `catalog/sources.json`.
//!
//! ```sh
//! cargo run -p catalog-builder [-- <pasta do catálogo>]
//! ```
//!
//! Variáveis de ambiente (todas opcionais; sem uma delas, os dados dessa
//! fonte são mantidos do `games.json` anterior):
//! - `GITHUB_TOKEN`
//! - `SGDB_API_KEY`
//! - `IGDB_CLIENT_ID` + `IGDB_CLIENT_SECRET`
//! - `CATALOG_BASE_URL` (padrão: [`recomphub_catalog::DEFAULT_BASE_URL`])

mod github;
mod igdb;
mod sgdb;

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::Utc;
use recomphub_catalog::{Catalog, CatalogGame, Images, SCHEMA_VERSION, SourceGame, Sources};

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

#[tokio::main]
async fn main() -> Result<()> {
    let dir = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "catalog".into()));
    let sources: Sources = serde_json::from_slice(
        &std::fs::read(dir.join("sources.json")).context("lendo sources.json")?,
    )
    .context("sources.json inválido")?;
    let previous: Option<Catalog> = std::fs::read(dir.join("games.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok());
    let previous_games: HashMap<&str, &CatalogGame> = previous
        .iter()
        .flat_map(|c| &c.games)
        .map(|g| (g.id.as_str(), g))
        .collect();
    let base_url = env("CATALOG_BASE_URL").unwrap_or_else(|| recomphub_catalog::DEFAULT_BASE_URL.into());

    let http = reqwest::Client::builder()
        .user_agent("RecompHub-catalog-builder")
        .build()?;
    let gh = github::GitHub::new(http.clone(), env("GITHUB_TOKEN"));
    let sgdb = env("SGDB_API_KEY").map(|key| sgdb::SteamGridDb::new(http.clone(), key));
    let igdb = match (env("IGDB_CLIENT_ID"), env("IGDB_CLIENT_SECRET")) {
        (Some(id), Some(secret)) => Some(igdb::Igdb::connect(http.clone(), id, &secret).await?),
        _ => None,
    };
    if sgdb.is_none() {
        eprintln!("aviso: SGDB_API_KEY ausente; imagens mantidas da versão anterior");
    }
    if igdb.is_none() {
        eprintln!("aviso: IGDB_CLIENT_ID/IGDB_CLIENT_SECRET ausentes; dados do jogo mantidos da versão anterior");
    }

    let mut games = Vec::new();
    let mut failures = 0;
    for source in sources.games {
        let prev = previous_games.get(source.id.as_str()).copied();
        println!("• {}", source.name);
        let ctx = Ctx { gh: &gh, sgdb: sgdb.as_ref(), igdb: igdb.as_ref(), base_url: &base_url };
        games.push(build_game(&ctx, source, prev, &mut failures).await);
    }

    // `generatedAt` só muda quando o conteúdo muda, para o Action não
    // commitar todo dia sem necessidade.
    let generated_at = match &previous {
        Some(p) if serde_json::to_value(&p.games)? == serde_json::to_value(&games)? => p.generated_at,
        _ => Utc::now(),
    };
    let catalog = Catalog { schema_version: SCHEMA_VERSION, generated_at, games };
    let mut json = serde_json::to_string_pretty(&catalog)?;
    json.push('\n');
    std::fs::write(dir.join("games.json"), json)?;
    println!("games.json gravado ({} jogos, {failures} falhas)", catalog.games.len());
    if failures > 0 {
        std::process::exit(1);
    }
    Ok(())
}

struct Ctx<'a> {
    gh: &'a github::GitHub,
    sgdb: Option<&'a sgdb::SteamGridDb>,
    igdb: Option<&'a igdb::Igdb>,
    base_url: &'a str,
}

/// Cada fonte que falhar mantém os dados anteriores daquele jogo; a falha é
/// contada para o processo sair com erro no fim.
async fn build_game(
    ctx: &Ctx<'_>,
    source: SourceGame,
    prev: Option<&CatalogGame>,
    failures: &mut u32,
) -> CatalogGame {
    let mut fail = |what: &str, e: anyhow::Error| {
        eprintln!("  erro em {what}: {e:#}");
        *failures += 1;
    };

    let recomp = match ctx.gh.recomp_info(&source.repo).await {
        Ok(info) => Some(info),
        Err(e) => {
            fail("GitHub", e);
            prev.and_then(|p| p.recomp.clone())
        }
    };

    let mut sgdb_id = source.sgdb_id.or(prev.and_then(|p| p.sgdb_id));
    let mut fetched_images = prev.map(|p| p.images.clone()).unwrap_or_default();
    if let Some(sgdb) = ctx.sgdb {
        let result = async {
            if sgdb_id.is_none() {
                if let Some((id, name)) = sgdb.search(&source.original_title).await? {
                    println!("  SGDB: \"{name}\" (sgdbId {id}) — fixe no sources.json se estiver certo");
                    sgdb_id = Some(id);
                }
            }
            match sgdb_id {
                Some(id) => sgdb.images(id).await.map(Some),
                None => Ok(None),
            }
        }
        .await;
        match result {
            Ok(Some(images)) => fetched_images = images,
            Ok(None) => eprintln!("  SGDB: nenhum resultado para \"{}\"", source.original_title),
            Err(e) => fail("SteamGridDB", e),
        }
    }
    let images = resolve_local(source.images.clone(), ctx.base_url).or(fetched_images);

    let mut igdb_id = source.igdb_id.or(prev.and_then(|p| p.igdb_id));
    let mut game = prev.and_then(|p| p.game.clone());
    if let Some(igdb) = ctx.igdb {
        match igdb.game(igdb_id, &source.original_title, &source.original_platform).await {
            Ok(Some((id, info))) => {
                if igdb_id.is_none() {
                    println!("  IGDB: \"{}\" (igdbId {id}) — fixe no sources.json se estiver certo", info.title);
                }
                igdb_id = Some(id);
                game = Some(info);
            }
            Ok(None) => eprintln!("  IGDB: nenhum resultado para \"{}\"", source.original_title),
            Err(e) => fail("IGDB", e),
        }
    }

    CatalogGame {
        id: source.id,
        name: source.name,
        repo: source.repo,
        original_title: source.original_title,
        original_platform: source.original_platform,
        platforms: source.platforms,
        required_rom: source.required_rom,
        achievements: source.achievements,
        sgdb_id,
        igdb_id,
        images,
        recomp,
        game,
    }
}

/// Caminhos relativos (ex.: `images/zelda-hero.png`) viram URL do catálogo publicado.
fn resolve_local(images: Images, base_url: &str) -> Images {
    let resolve = |v: Option<String>| {
        v.map(|s| {
            if s.starts_with("http://") || s.starts_with("https://") {
                s
            } else {
                format!("{}/{}", base_url.trim_end_matches('/'), s.trim_start_matches('/'))
            }
        })
    };
    Images {
        grid: resolve(images.grid),
        hero: resolve(images.hero),
        logo: resolve(images.logo),
        icon: resolve(images.icon),
    }
}
