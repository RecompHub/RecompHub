//! Execução do jogo como processo filho.
//!
//! O handle do processo filho é o que a fase 1 das conquistas vai usar para
//! ler a memória (`ReadProcessMemory` / `process_vm_readv`).

use std::path::Path;
use std::process::{Child, Command};

use crate::error::Result;

/// Inicia o executável com a pasta dele como diretório de trabalho: as
/// recomps procuram assets relativos a ela.
pub fn spawn(executable: &Path) -> Result<Child> {
    let mut command = Command::new(executable);
    if let Some(dir) = executable.parent() {
        command.current_dir(dir);
    }
    Ok(command.spawn()?)
}
