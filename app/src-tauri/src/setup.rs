//! Jogos que vêm com o instalador do próprio projeto (ex.: um Setup que
//! compila o jogo a partir do disco do usuário, como WiiCompiled e Nebula).
//!
//! O launcher baixa o instalador da release, executa e, quando ele termina,
//! procura o executável na pasta onde o instalador coloca o jogo. Os arquivos
//! são do instalador: remover do launcher não apaga nada (o próprio projeto
//! tem desinstalador).

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::Deserialize;

use crate::catalog::PlatformAsset;
use crate::error::{Error, Result};
use crate::github::Release;
use crate::install::{self, Progress, Stage};
use crate::state::InstalledGame;
use recomphub_catalog::Installer;

const ROM_PLACEHOLDER: &str = "{rom}";

/// O instalador precisa que o usuário escolha a imagem do disco.
pub fn needs_rom(installer: &Installer) -> bool {
    installer.args.iter().any(|a| a.contains(ROM_PLACEHOLDER))
}

/// Baixa e executa o instalador; devolve o jogo instalado.
pub async fn install(
    http: &reqwest::Client,
    cache_dir: &Path,
    platform: &PlatformAsset,
    installer: &Installer,
    release: &Release,
    rom: Option<PathBuf>,
    on_progress: impl Fn(Progress) + Send + Sync + 'static,
) -> Result<InstalledGame> {
    let args = build_args(installer, rom.as_deref())?;
    let asset = release.find_asset(&platform.asset_pattern)?;
    let dir = cache_dir.join("installers");
    tokio::fs::create_dir_all(&dir).await?;
    let setup = dir.join(&asset.name);

    if let Err(e) = install::download(http, asset, &setup, &on_progress).await {
        let _ = tokio::fs::remove_file(&setup).await;
        return Err(e);
    }

    let log = dir.join(format!("{}.log", asset.name));
    let installer = installer.clone();
    let platform = platform.clone();
    let tag = release.tag_name.clone();
    tokio::task::spawn_blocking(move || {
        let result = run(&setup, &args, &log, installer.progress_json, &on_progress);
        let _ = std::fs::remove_file(&setup);
        result?;
        let mut installed = detect(&installer, &platform)?
            .ok_or_else(|| Error::InstallerNoGame(expand_path(&installer.install_dir).display().to_string()))?;
        installed.installed_tag = tag;
        Ok(installed)
    })
    .await
    .expect("tarefa do instalador entrou em pânico")
}

/// Procura o jogo onde o instalador o coloca (também serve para registrar um
/// jogo que o usuário instalou por fora). A versão fica desconhecida.
pub fn detect(installer: &Installer, platform: &PlatformAsset) -> Result<Option<InstalledGame>> {
    let dir = expand_path(&installer.install_dir);
    if !dir.is_dir() {
        return Ok(None);
    }
    let Some(found) = install::find_executable(&dir, &platform.executable)? else {
        return Ok(None);
    };
    Ok(Some(InstalledGame {
        installed_tag: install::UNKNOWN_TAG.to_string(),
        installed_at: chrono::Local::now(),
        executable: found.strip_prefix(&dir).unwrap_or(&found).to_path_buf(),
        executable_hash: install::sha256_file(&found)?,
        install_path: dir,
        imported: false,
        external: true,
    }))
}

fn build_args(installer: &Installer, rom: Option<&Path>) -> Result<Vec<String>> {
    installer
        .args
        .iter()
        .map(|arg| {
            if !arg.contains(ROM_PLACEHOLDER) {
                return Ok(arg.clone());
            }
            let rom = rom.ok_or(Error::RomRequired)?;
            Ok(arg.replace(ROM_PLACEHOLDER, &rom.to_string_lossy()))
        })
        .collect()
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum Event {
    Progress {
        #[serde(default)]
        percent: u64,
        #[serde(default)]
        message: Option<String>,
    },
    Result {
        success: bool,
        #[serde(default)]
        error: Option<String>,
    },
}

/// Executa o instalador e espera ele terminar. Com `progress_json`, roda sem
/// janela e repassa o progresso; sem, abre a janela do próprio instalador.
fn run(setup: &Path, args: &[String], log: &Path, progress_json: bool, on_progress: &impl Fn(Progress)) -> Result<()> {
    let mut command = Command::new(setup);
    command.args(args);
    if let Some(dir) = setup.parent() {
        command.current_dir(dir);
    }
    on_progress(Progress {
        stage: Stage::Setup,
        downloaded: 0,
        total: if progress_json { 100 } else { 0 },
        message: None,
    });

    if !progress_json {
        let status = command.status()?;
        return if status.success() {
            Ok(())
        } else {
            Err(Error::InstallerFailed(format!("código {}", status.code().unwrap_or(-1))))
        };
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    // O stderr (diagnóstico) vai para um log, para ajudar a reportar erros.
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(std::fs::File::create(log)?);
    let mut child = command.spawn()?;
    let stdout = child.stdout.take().expect("stdout configurado como pipe");

    let mut outcome: Option<std::result::Result<(), String>> = None;
    for line in BufReader::new(stdout).lines() {
        let Ok(event) = serde_json::from_str::<Event>(&line?) else {
            continue;
        };
        match event {
            Event::Progress { percent, message } => on_progress(Progress {
                stage: Stage::Setup,
                downloaded: percent.min(100),
                total: 100,
                message,
            }),
            Event::Result { success, error } => {
                outcome = Some(if success { Ok(()) } else { Err(error.unwrap_or_default()) });
            }
        }
    }
    let status = child.wait()?;
    match outcome {
        Some(Ok(())) if status.success() => Ok(()),
        Some(Err(message)) if !message.is_empty() => Err(Error::InstallerFailed(message)),
        _ => Err(Error::InstallerFailed(format!(
            "código {} (detalhes em {})",
            status.code().unwrap_or(-1),
            log.display()
        ))),
    }
}

/// `%VARIAVEL%` (Windows) e `~` (pasta do usuário).
pub fn expand_path(raw: &str) -> PathBuf {
    let mut out = String::new();
    let mut rest = raw;
    while let Some(start) = rest.find('%') {
        let Some(len) = rest[start + 1..].find('%') else {
            break;
        };
        let name = &rest[start + 1..start + 1 + len];
        out.push_str(&rest[..start]);
        match std::env::var(name) {
            Ok(value) => out.push_str(&value),
            Err(_) => out.push_str(&rest[start..start + len + 2]),
        }
        rest = &rest[start + len + 2..];
    }
    out.push_str(rest);
    if let Some(tail) = out.strip_prefix('~') {
        let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).unwrap_or_default();
        out = format!("{home}{tail}");
    }
    PathBuf::from(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn installer(args: &[&str]) -> Installer {
        Installer {
            install_dir: "%RECOMPHUB_TEST_DIR%/Programs/Jogo".into(),
            args: args.iter().map(|a| a.to_string()).collect(),
            progress_json: false,
        }
    }

    #[test]
    fn rom_placeholder_is_required_and_replaced() {
        let i = installer(&["--silent", "--game", "{rom}"]);
        assert!(needs_rom(&i));
        assert!(matches!(build_args(&i, None), Err(Error::RomRequired)));
        let args = build_args(&i, Some(Path::new("C:/isos/mkwii.rvz"))).unwrap();
        assert_eq!(args, ["--silent", "--game", "C:/isos/mkwii.rvz"]);
        assert!(!needs_rom(&installer(&[])));
    }

    #[test]
    fn expands_environment_variables() {
        // SAFETY: teste de thread única para esta variável.
        unsafe { std::env::set_var("RECOMPHUB_TEST_DIR", "C:/Users/x/AppData/Local") };
        assert_eq!(
            expand_path("%RECOMPHUB_TEST_DIR%/Programs/Nebula"),
            PathBuf::from("C:/Users/x/AppData/Local/Programs/Nebula")
        );
        assert_eq!(expand_path("%NAO_EXISTE_XYZ%/a"), PathBuf::from("%NAO_EXISTE_XYZ%/a"));
    }

    /// Um "instalador" de mentira (PowerShell) falando o protocolo NDJSON.
    #[cfg(windows)]
    #[test]
    fn reads_ndjson_progress_and_result() {
        let powershell = Path::new(r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe");
        let log = std::env::temp_dir().join(format!("recomphub-setup-log-{}", std::process::id()));
        let script = |result: &str| {
            vec![
                "-NoProfile".to_string(),
                "-Command".to_string(),
                format!(
                    "Write-Output '{{\"type\":\"progress\",\"stage\":\"x\",\"message\":\"Compilando\",\"percent\":42}}'; Write-Output 'texto solto'; Write-Output '{result}'"
                ),
            ]
        };
        let seen = std::sync::Mutex::new(Vec::new());
        let ok = run(powershell, &script(r#"{"type":"result","success":true,"installDir":"C:/x"}"#), &log, true, &|p: Progress| {
            seen.lock().unwrap().push((p.downloaded, p.message))
        });
        assert!(ok.is_ok(), "{ok:?}");
        let seen = seen.into_inner().unwrap();
        assert!(seen.contains(&(42, Some("Compilando".to_string()))), "{seen:?}");

        let failed = run(powershell, &script(r#"{"type":"result","success":false,"error":"disco errado"}"#), &log, true, &|_| {});
        assert!(matches!(failed, Err(Error::InstallerFailed(m)) if m == "disco errado"));
        let _ = std::fs::remove_file(&log);
    }

    #[test]
    fn detects_newest_version_folder() {
        let root = std::env::temp_dir().join(format!("recomphub-setup-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for v in ["1.0.0", "1.0.1"] {
            std::fs::create_dir_all(root.join("versions").join(v)).unwrap();
            std::fs::write(root.join("versions").join(v).join("Nebula.exe"), v).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let installer = Installer {
            install_dir: root.to_string_lossy().into_owned(),
            args: Vec::new(),
            progress_json: false,
        };
        let platform = PlatformAsset {
            asset_pattern: "Nebula-Setup.exe".into(),
            executable: "Nebula.exe".into(),
            installer: Some(installer.clone()),
        };
        let found = detect(&installer, &platform).unwrap().unwrap();
        assert!(found.external);
        assert_eq!(found.executable, PathBuf::from("versions/1.0.1/Nebula.exe"));
        std::fs::remove_dir_all(&root).unwrap();
    }
}
