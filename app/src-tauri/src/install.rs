//! Download, extração e remoção dos jogos.
//!
//! Cada jogo fica em `<games_dir>/<id>/` (ou onde já estava instalado). A
//! release é extraída primeiro em `<pasta>.staging/` e só depois aplicada,
//! então um download ou extração com erro não quebra a versão instalada.
//!
//! O launcher registra em [`MANIFEST`] os arquivos que ele mesmo colocou na
//! pasta. Atualizar e desinstalar removem **só esses**: ROM, saves, mods e
//! qualquer outro arquivo do usuário na pasta do jogo nunca são apagados.

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
    /// Instalador do próprio projeto rodando (ver `setup.rs`).
    Setup,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub stage: Stage,
    pub downloaded: u64,
    pub total: u64,
    /// Etapa descrita pelo instalador do projeto, quando ele informa.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Arquivo, dentro da pasta do jogo, com a lista do que o launcher instalou.
pub const MANIFEST: &str = ".recomphub-files.json";

#[derive(Debug, Serialize, serde::Deserialize)]
struct Manifest {
    /// Caminhos relativos à pasta do jogo, com `/`.
    files: Vec<String>,
}

/// Instala (ou atualiza) a release em `target`.
pub async fn install(
    http: &reqwest::Client,
    target: &Path,
    platform: &PlatformAsset,
    release: &Release,
    on_progress: impl Fn(Progress),
) -> Result<InstalledGame> {
    let asset = release.find_asset(&platform.asset_pattern)?;
    let parent = target
        .parent()
        .ok_or_else(|| Error::InvalidGamesDir(target.display().to_string()))?;
    tokio::fs::create_dir_all(parent).await?;

    let download_path = sibling(target, "download");
    let result = download(http, asset, &download_path, &on_progress).await;
    if let Err(e) = result {
        let _ = tokio::fs::remove_file(&download_path).await;
        return Err(e);
    }

    on_progress(Progress {
        stage: Stage::Extract,
        downloaded: asset.size,
        total: asset.size,
        message: None,
    });

    let target = target.to_path_buf();
    let asset_name = asset.name.clone();
    let executable = platform.executable.clone();
    let tag = release.tag_name.clone();
    tokio::task::spawn_blocking(move || {
        let result = unpack(&target, &download_path, &asset_name, &executable, tag);
        let _ = std::fs::remove_file(&download_path);
        result
    })
    .await
    .expect("tarefa de extração entrou em pânico")
}

pub(crate) async fn download(
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
                message: None,
            });
        }
    }
    file.flush().await?;
    Ok(())
}

/// `<pasta>.<sufixo>` ao lado da pasta do jogo (mesmo disco, para o rename).
fn sibling(target: &Path, suffix: &str) -> PathBuf {
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    target.with_file_name(format!("{name}.{suffix}"))
}

fn unpack(
    target: &Path,
    archive: &Path,
    asset_name: &str,
    executable_pattern: &str,
    tag: String,
) -> Result<InstalledGame> {
    let staging = sibling(target, "staging");
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

    // Tira os arquivos da versão anterior que o launcher instalou e coloca os
    // novos por cima; o resto da pasta (do usuário) fica como está.
    remove_managed_files(target)?;
    let files = list_files(&staging)?;
    for file in &files {
        let dest = target.join(file);
        if let Some(dir) = dest.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::rename(staging.join(file), &dest)?;
    }
    std::fs::remove_dir_all(&staging)?;
    let manifest = Manifest {
        files: files
            .iter()
            .map(|f| f.to_string_lossy().replace('\\', "/"))
            .collect(),
    };
    std::fs::write(target.join(MANIFEST), serde_json::to_vec_pretty(&manifest)?)?;

    Ok(InstalledGame {
        installed_tag: tag,
        installed_at: chrono::Local::now(),
        install_path: target.to_path_buf(),
        executable: relative,
        executable_hash,
        imported: false,
        external: false,
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

/// Arquivos (não pastas) sob `root`, relativos a ele.
fn list_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                dirs.push(path);
            } else {
                files.push(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
            }
        }
    }
    files.sort();
    Ok(files)
}

/// Apaga os arquivos listados no manifesto (e as pastas que ficarem vazias).
/// Sem manifesto, não apaga nada.
fn remove_managed_files(target: &Path) -> Result<()> {
    let Ok(bytes) = std::fs::read(target.join(MANIFEST)) else {
        return Ok(());
    };
    let manifest: Manifest = serde_json::from_slice(&bytes)?;
    for file in &manifest.files {
        // Nunca sai da pasta do jogo, mesmo com um manifesto adulterado.
        let relative = Path::new(file);
        let escapes = relative.is_absolute()
            || relative.components().any(|c| {
                matches!(
                    c,
                    std::path::Component::ParentDir | std::path::Component::Prefix(_)
                )
            });
        if escapes {
            continue;
        }
        let path = target.join(relative);
        if path.is_file() {
            std::fs::remove_file(&path)?;
        }
        let mut dir = path.parent();
        while let Some(d) = dir {
            // Para ao chegar na raiz do jogo ou numa pasta que não ficou vazia.
            if d == target || std::fs::remove_dir(d).is_err() {
                break;
            }
            dir = d.parent();
        }
    }
    let _ = std::fs::remove_file(target.join(MANIFEST));
    Ok(())
}

/// Desinstala: remove só os arquivos que o launcher instalou. Jogos
/// importados são apenas esquecidos.
pub fn uninstall(game_id: &str, installed: &InstalledGame) -> Result<()> {
    if installed.imported || installed.external {
        return Ok(());
    }
    let path = &installed.install_path;
    if !path.exists() {
        return Ok(());
    }
    if path.join(MANIFEST).is_file() {
        remove_managed_files(path)?;
        let _ = std::fs::remove_dir(path); // só se ficou vazia
        return Ok(());
    }
    // Instalações de versões antigas do launcher (sem manifesto) ficavam numa
    // pasta só delas, com o nome do jogo.
    // Os jogos podem estar em qualquer pasta escolhida pelo usuário, então a
    // proteção é verificar que é mesmo uma pasta criada pelo launcher.
    let looks_like_ours = path.file_name().is_some_and(|n| n == game_id)
        && path.join(&installed.executable).is_file();
    if !looks_like_ours {
        return Err(Error::Io(std::io::Error::other(format!(
            "{} não parece uma instalação do RecompHub; remova manualmente",
            path.display()
        ))));
    }
    std::fs::remove_dir_all(path)?;
    Ok(())
}

/// Registra uma recomp instalada manualmente em `dir`. A versão é
/// desconhecida, então o launcher vai oferecer a atualização.
pub fn import(dir: &Path, platform: &PlatformAsset) -> Result<InstalledGame> {
    let found = find_executable(dir, &platform.executable)?
        .ok_or_else(|| Error::ExecutableNotFound(platform.executable.clone()))?;
    Ok(InstalledGame {
        installed_tag: UNKNOWN_TAG.to_string(),
        installed_at: chrono::Local::now(),
        install_path: dir.to_path_buf(),
        executable: found.strip_prefix(dir).unwrap_or(&found).to_path_buf(),
        executable_hash: sha256_file(&found)?,
        imported: true,
        external: false,
    })
}

pub const UNKNOWN_TAG: &str = "desconhecida";

/// Procura o executável pelo nome (aceita glob), preferindo o mais raso:
/// alguns zips têm uma pasta raiz, outros não. Empate na mesma profundidade
/// fica com o modificado por último (ex.: `versions/<v>/Jogo.exe` de
/// instaladores que guardam a versão anterior).
pub(crate) fn find_executable(root: &Path, pattern: &str) -> Result<Option<PathBuf>> {
    let glob = GlobBuilder::new(pattern)
        .case_insensitive(true)
        .literal_separator(true)
        .build()?
        .compile_matcher();
    let mut dirs = vec![root.to_path_buf()];
    while !dirs.is_empty() {
        let mut next = Vec::new();
        let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
        for dir in dirs {
            let mut entries: Vec<_> = std::fs::read_dir(&dir)?.collect::<std::io::Result<_>>()?;
            entries.sort_by_key(|e| e.file_name());
            for entry in entries {
                let path = entry.path();
                if entry.file_type()?.is_dir() {
                    next.push(path);
                } else if glob.is_match(entry.file_name()) {
                    let modified = entry.metadata()?.modified().unwrap_or(std::time::UNIX_EPOCH);
                    if best.as_ref().is_none_or(|(t, _)| modified > *t) {
                        best = Some((modified, path));
                    }
                }
            }
        }
        if let Some((_, path)) = best {
            return Ok(Some(path));
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

    #[test]
    fn imported_install_is_registered_and_never_deleted() {
        let dir = temp_dir("import");
        std::fs::create_dir_all(dir.join("bin")).unwrap();
        std::fs::write(dir.join("bin/MyRecomp.exe"), b"exe").unwrap();
        let platform = PlatformAsset {
            asset_pattern: "*.zip".into(),
            executable: "myrecomp.exe".into(),
            installer: None,
        };

        let installed = import(&dir, &platform).unwrap();
        assert!(installed.imported);
        assert_eq!(installed.installed_tag, UNKNOWN_TAG);
        assert_eq!(installed.executable, PathBuf::from("bin/MyRecomp.exe"));

        uninstall("qualquer-id", &installed).unwrap();
        assert!(
            dir.join("bin/MyRecomp.exe").is_file(),
            "arquivos do usuário apagados"
        );
        std::fs::remove_dir_all(&dir).unwrap();
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

        let dir = root.join("game");
        let installed = unpack(&dir, &zip_path, "Game-Linux-X64.zip", "Game", "v1".into()).unwrap();
        assert_eq!(installed.executable, PathBuf::from("Game"));
        assert!(dir.join("Game").is_file());
        assert!(dir.join("assets/a.txt").is_file());
        assert!(!dir.join("Game.tar.gz").exists());

        std::fs::remove_dir_all(&root).unwrap();
    }

    fn zip_with(path: &Path, files: &[(&str, &[u8])]) {
        let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for (name, data) in files {
            zip.start_file(*name, options).unwrap();
            std::io::Write::write_all(&mut zip, data).unwrap();
        }
        zip.finish().unwrap();
    }

    /// O cenário do Dusklight: o usuário guardou a ROM na pasta do jogo.
    #[test]
    fn update_and_uninstall_never_touch_user_files() {
        let root = temp_dir("manifest");
        let dir = root.join("Jogo");
        std::fs::create_dir_all(dir.join("ROM")).unwrap();
        std::fs::write(dir.join("ROM/jogo.iso"), b"rom").unwrap();

        // v1 por cima da pasta que já tinha a ROM
        let v1 = root.join("v1.zip");
        zip_with(
            &v1,
            &[("Game.exe", b"v1"), ("old.dll", b"x"), ("libs/a.dll", b"x")],
        );
        unpack(&dir, &v1, "Game-Windows.zip", "game.exe", "v1".into()).unwrap();
        assert!(dir.join("old.dll").is_file());
        assert!(dir.join("ROM/jogo.iso").is_file());

        // v2 remove o que a v1 instalou e não existe mais
        let v2 = root.join("v2.zip");
        zip_with(&v2, &[("Game.exe", b"v2"), ("new.dll", b"x")]);
        let installed = unpack(&dir, &v2, "Game-Windows.zip", "game.exe", "v2".into()).unwrap();
        assert!(!dir.join("old.dll").exists());
        assert!(
            !dir.join("libs").exists(),
            "pasta vazia da v1 deveria sumir"
        );
        assert_eq!(std::fs::read(dir.join("Game.exe")).unwrap(), b"v2");
        assert!(dir.join("ROM/jogo.iso").is_file());

        // desinstalar deixa só o que é do usuário
        uninstall("jogo", &installed).unwrap();
        assert!(!dir.join("Game.exe").exists());
        assert!(!dir.join(MANIFEST).exists());
        assert!(dir.join("ROM/jogo.iso").is_file());

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn tampered_manifest_cannot_delete_outside_the_game_folder() {
        let root = temp_dir("tampered");
        let dir = root.join("jogo");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(root.join("fora.txt"), b"importante").unwrap();
        std::fs::write(dir.join(MANIFEST), br#"{"files":["../fora.txt"]}"#).unwrap();
        remove_managed_files(&dir).unwrap();
        assert!(root.join("fora.txt").is_file());
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
