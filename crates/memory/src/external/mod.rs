//! Fase 1: lê a RDRAM de fora do processo da recomp.
//!
//! O runtime reserva 4 GiB e deixa só os primeiros 512 MiB legíveis
//! (`librecomp/src/recomp.cpp`); essa assinatura identifica a RDRAM sem
//! depender da versão do jogo.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
pub use linux::{ExternalMemory, find_rdram};
#[cfg(windows)]
pub use windows::{ExternalMemory, find_rdram};

/// Tamanhos de `librecomp/include/librecomp/addresses.hpp`.
pub const RECOMP_MEM_SIZE: usize = 512 * 1024 * 1024;
pub const RECOMP_ALLOCATION_SIZE: usize = 4096 * 1024 * 1024;

#[derive(Debug)]
pub enum Error {
    /// O processo não pôde ser aberto (fechou ou sem permissão).
    OpenProcess(std::io::Error),
    /// Nenhuma alocação com o formato da RDRAM da recomp.
    RdramNotFound,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::OpenProcess(e) => write!(f, "não foi possível abrir o processo: {e}"),
            Error::RdramNotFound => f.write_str("RDRAM da recomp não encontrada no processo"),
        }
    }
}

impl std::error::Error for Error {}

/// Procura a RDRAM no conteúdo de `/proc/<pid>/maps`: um mapeamento anônimo
/// `rw-p` de `readable` bytes seguido, sem buraco, de um `---p` com o resto.
/// Fica fora do módulo `linux` para ser testável em qualquer sistema.
pub fn find_rdram_in_maps(maps: &str, readable: usize, total: usize) -> Option<usize> {
    struct Map<'a> {
        start: usize,
        end: usize,
        perms: &'a str,
        anonymous: bool,
    }
    fn parse(line: &str) -> Option<Map<'_>> {
        let mut fields = line.split_whitespace();
        let (start, end) = fields.next()?.split_once('-')?;
        let perms = fields.next()?;
        let inode = fields.nth(2)?;
        let path = fields.next();
        Some(Map {
            start: usize::from_str_radix(start, 16).ok()?,
            end: usize::from_str_radix(end, 16).ok()?,
            perms,
            anonymous: inode == "0" && path.is_none(),
        })
    }
    let maps: Vec<Map> = maps.lines().filter_map(parse).collect();
    maps.windows(2).find_map(|pair| {
        let (ram, guard) = (&pair[0], &pair[1]);
        let matches = ram.perms == "rw-p"
            && ram.anonymous
            && ram.end - ram.start == readable
            && guard.perms == "---p"
            && guard.start == ram.end
            && guard.end - guard.start == total - readable;
        matches.then_some(ram.start)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_rdram_in_proc_maps() {
        // Trecho no formato real; a RDRAM é o par 7f00..7f20 / 7f20..7f00+4GiB.
        let maps = "\
55d0c0000000-55d0c0200000 r-xp 00000000 08:01 1234   /opt/Zelda64Recompiled
7eff00000000-7eff00010000 rw-p 00000000 00:00 0
7f0000000000-7f0020000000 rw-p 00000000 00:00 0
7f0020000000-7f0100000000 ---p 00000000 00:00 0
7f0100000000-7f0100001000 rw-p 00000000 00:00 0      [heap]
";
        assert_eq!(
            find_rdram_in_maps(maps, RECOMP_MEM_SIZE, RECOMP_ALLOCATION_SIZE),
            Some(0x7f00_0000_0000)
        );
    }

    #[test]
    fn ignores_file_backed_and_non_adjacent_mappings() {
        let maps = "\
7f0000000000-7f0020000000 rw-p 00000000 08:01 99     /tmp/arquivo
7f0020000000-7f0100000000 ---p 00000000 00:00 0
";
        assert_eq!(find_rdram_in_maps(maps, RECOMP_MEM_SIZE, RECOMP_ALLOCATION_SIZE), None);
    }
}
