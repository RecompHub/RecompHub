//! Download, extração e remoção dos jogos.
//!
//! Cada jogo fica em `<games_dir>/<id>/`. A release é extraída primeiro em
//! `<id>.staging/` e só substitui a pasta antiga quando tudo deu certo, então
//! uma atualização interrompida não quebra a versão instalada.

use std::io::Read;
use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use globset::GlobBuilder;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use crate::catalog::PlatformAsset;
use crate::error::{Error, Result};
use crate::github::{Asset, Release};
use crate::state::InstalledGame;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Stage {
    Download,
    Extract,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub stage: Stage,
    pub downloaded: u64,
    pub total: u64,
}

pub async fn install(
    http: &reqwest::Client,
    games_dir: &Path,
    game_id: &str,
    platform: &PlatformAsset,
    release: &Release,
    on_progress: impl Fn(Progress),
) -> Result<InstalledGame> {
    let asset = release.find_asset(&platform.asset_pattern)?;
    tokio::fs::create_dir_all(games_dir).await?;

    let download_path = games_dir.join(format!("{game_id}.download"));
    let result = download(http, asset, &download_path, &on_progress).await;
    if let Err(e) = result {
        let _ = tokio::fs::remove_file(&download_path).await;
        return Err(e);
    }

    on_progress(Progress {
        stage: Stage::Extract,
        downloaded: asset.size,
        total: asset.size,
    });

    let games_dir = games_dir.to_path_buf();
    let game_id = game_id.to_string();
    let asset_name = asset.name.clone();
    let executable = platform.executable.clone();
    let tag = release.tag_name.clone();
    tokio::task::spawn_blocking(move || {
        let result = unpack(
            &games_dir,
            &game_id,
            &download_path,
            &asset_name,
            &executable,
            tag,
        );
        let _ = std::fs::remove_file(&download_path);
        result
    })
    .await
    .expect("tarefa de extração entrou em pânico")
}

async fn download(
    http: &reqwest::Client,
    asset: &Asset,
    dest: &Path,
    on_progress: &impl Fn(Progress),
) -> Result<()> {
    let response = http
        .get(&asset.browser_download_url)
        .send()
        .await?
        .error_for_status()?;
    let total = response.content_length().unwrap_or(asset.size);
    let mut file = tokio::fs::File::create(dest).await?;
    let mut stream = response.bytes_stream();
    let mut downloaded = 0u64;
    let mut last_reported = 0u64;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        // Um evento a cada ~1% basta para a barra de progresso.
        if downloaded - last_reported >= total / 100 || downloaded == total {
            last_reported = downloaded;
            on_progress(Progress {
                stage: Stage::Download,
                downloaded,
                total,
            });
        }
    }
    file.flush().await?;
    Ok(())
}

fn unpack(
    games_dir: &Path,
    game_id: &str,
    archive: &Path,
    asset_name: &str,
    executable_pattern: &str,
    tag: String,
) -> Result<InstalledGame> {
    let staging = games_dir.join(format!("{game_id}.staging"));
    let target = games_dir.join(game_id);
    if staging.exists() {
        std::fs::remove_dir_all(&staging)?;
    }
    std::fs::create_dir_all(&staging)?;

    let lower = asset_name.to_ascii_lowercase();
    if lower.ends_with(".zip") {
        // `extract` sanitiza os caminhos (proteção contra zip slip).
        zip::ZipArchive::new(std::fs::File::open(archive)?)?.extract(&staging)?;
    } else if is_tarball(&lower) {
        extract_tar_gz(archive, &staging)?;
    } else {
        // Asset que já é o próprio executável (ex.: AppImage).
        std::fs::copy(archive, staging.join(asset_name))?;
    }
    // Os builds Linux das recomps vêm como um zip com um .tar.gz dentro (o
    // tar preserva a permissão de execução, que o zip do GitHub perde).
    unpack_nested_tarballs(&staging)?;

    let found = find_executable(&staging, executable_pattern)?
        .ok_or_else(|| Error::ExecutableNotFound(executable_pattern.to_string()))?;
    let relative = found.strip_prefix(&staging).unwrap_or(&found).to_path_buf();
    make_executable(&found)?;
    let executable_hash = sha256_file(&found)?;

    if target.exists() {
        std::fs::remove_dir_all(&target)?;
    }
    std::fs::rename(&staging, &target)?;

    Ok(InstalledGame {
        installed_tag: tag,
        installed_at: chrono::Local::now(),
        install_path: target,
        executable: relative,
        executable_hash,
    })
}

fn is_tarball(lower_name: &str) -> bool {
    lower_name.ends_with(".tar.gz") || lower_name.ends_with(".tgz")
}

/// `unpack` recusa entradas com `..` ou caminhos absolutos.
fn extract_tar_gz(archive: &Path, dest: &Path) -> Result<()> {
    let decoder = flate2::read::GzDecoder::new(std::fs::File::open(archive)?);
    let mut tar = tar::Archive::new(decoder);
    tar.set_preserve_permissions(true);
    tar.unpack(dest)?;
    Ok(())
}

fn unpack_nested_tarballs(dir: &Path) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let is_tar = path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| is_tarball(&n.to_ascii_lowercase()));
        if is_tar && path.is_file() {
            extract_tar_gz(&path, dir)?;
            std::fs::remove_file(&path)?;
        }
    }
    Ok(())
}

/// Remove a pasta do jogo. Recusa caminhos fora de `games_dir`, para que um
/// `installed.json` editado à mão não apague outra pasta.
pub fn uninstall(games_dir: &Path, installed: &InstalledGame) -> Result<()> {
    let path = &installed.install_path;
    if !path.starts_with(games_dir) {
        return Err(Error::Io(std::io::Error::other(format!(
            "{} está fora da pasta de jogos; remova manualmente",
            path.display()
        ))));
    }
    if path.exists() {
        std::fs::remove_dir_all(path)?;
    }
    Ok(())
}

/// Procura o executável pelo nome (aceita glob), preferindo o mais raso:
/// alguns zips têm uma pasta raiz, outros não.
fn find_executable(root: &Path, pattern: &str) -> Result<Option<PathBuf>> {
    let glob = GlobBuilder::new(pattern)
        .case_insensitive(true)
        .literal_separator(true)
        .build()?
        .compile_matcher();
    let mut dirs = vec![root.to_path_buf()];
    while !dirs.is_empty() {
        let mut next = Vec::new();
        for dir in dirs {
            let mut entries: Vec<_> = std::fs::read_dir(&dir)?.collect::<std::io::Result<_>>()?;
            entries.sort_by_key(|e| e.file_name());
            for entry in entries {
                let path = entry.path();
                if entry.file_type()?.is_dir() {
                    next.push(path);
                } else if glob.is_match(entry.file_name()) {
                    return Ok(Some(path));
                }
            }
        }
        dirs = next;
    }
    Ok(None)
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(perms.mode() | 0o755);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("recomphub-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Mesmo formato dos builds Linux do Zelda64Recomp e do Starfox64Recomp.
    #[test]
    fn unpack_handles_zip_with_tarball_inside() {
        let root = temp_dir("tarball");

        let mut tar_gz = Vec::new();
        {
            let encoder = flate2::write::GzEncoder::new(&mut tar_gz, flate2::Compression::fast());
            let mut builder = tar::Builder::new(encoder);
            for (name, data) in [("Game", &b"elf"[..]), ("assets/a.txt", &b"x"[..])] {
                let mut header = tar::Header::new_gnu();
                header.set_size(data.len() as u64);
                header.set_mode(0o755);
                header.set_cksum();
                builder.append_data(&mut header, name, data).unwrap();
            }
            builder.into_inner().unwrap().finish().unwrap();
        }

        let zip_path = root.join("download");
        {
            let mut zip = zip::ZipWriter::new(std::fs::File::create(&zip_path).unwrap());
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            zip.start_file("Game.tar.gz", options).unwrap();
            std::io::Write::write_all(&mut zip, &tar_gz).unwrap();
            zip.finish().unwrap();
        }

        let installed = unpack(&root, "game", &zip_path, "Game-Linux-X64.zip", "Game", "v1".into()).unwrap();
        let dir = root.join("game");
        assert_eq!(installed.executable, PathBuf::from("Game"));
        assert!(dir.join("Game").is_file());
        assert!(dir.join("assets/a.txt").is_file());
        assert!(!dir.join("Game.tar.gz").exists());

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn find_executable_prefers_shallowest_match() {
        let root = std::env::temp_dir().join(format!("recomphub-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("Game/sub")).unwrap();
        std::fs::write(root.join("Game/Game.exe"), b"a").unwrap();
        std::fs::write(root.join("Game/sub/Game.exe"), b"b").unwrap();

        let found = find_executable(&root, "game.exe").unwrap().unwrap();
        assert_eq!(found, root.join("Game/Game.exe"));
        assert!(find_executable(&root, "Other.exe").unwrap().is_none());

        std::fs::remove_dir_all(&root).unwrap();
    }
}
