//! Leitura externa no Linux: `/proc/<pid>/maps` + `process_vm_readv`.
//!
//! Com `ptrace_scope = 1` (padrão em várias distros e no SteamOS) só é
//! permitido ler processos descendentes. O launcher inicia o jogo como
//! processo filho, então isso basta.

use super::{Error, RECOMP_ALLOCATION_SIZE, RECOMP_MEM_SIZE, find_rdram_in_maps};
use crate::MemorySource;
use crate::n64::RDRAM_SIZE;

pub struct ExternalMemory {
    pid: libc::pid_t,
    rdram: usize,
}

impl ExternalMemory {
    /// Localiza a RDRAM no processo da recomp.
    pub fn attach(pid: u32) -> Result<Self, Error> {
        let rdram = find_rdram(pid, RECOMP_MEM_SIZE, RECOMP_ALLOCATION_SIZE)?.ok_or(Error::RdramNotFound)?;
        Ok(Self { pid: pid as libc::pid_t, rdram })
    }

    /// Endereço da RDRAM no processo (muda a cada execução por causa do ASLR).
    pub fn rdram_address(&self) -> usize {
        self.rdram
    }
}

impl MemorySource for ExternalMemory {
    fn read(&mut self, address: u32, buf: &mut [u8]) -> usize {
        let end = address as u64 + buf.len() as u64;
        if end > RDRAM_SIZE as u64 {
            return 0;
        }
        let local = libc::iovec { iov_base: buf.as_mut_ptr().cast(), iov_len: buf.len() };
        let remote = libc::iovec {
            iov_base: (self.rdram + address as usize) as *mut libc::c_void,
            iov_len: buf.len(),
        };
        let read = unsafe { libc::process_vm_readv(self.pid, &local, 1, &remote, 1, 0) };
        if read < 0 { 0 } else { read as usize }
    }
}

pub fn find_rdram(pid: u32, readable: usize, total: usize) -> Result<Option<usize>, Error> {
    let maps = std::fs::read_to_string(format!("/proc/{pid}/maps")).map_err(Error::OpenProcess)?;
    Ok(find_rdram_in_maps(&maps, readable, total))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::n64;

    /// Reproduz a alocação do runtime, em tamanho menor, no próprio processo.
    #[test]
    fn finds_rdram_shaped_mapping_and_reads_console_values() {
        const READABLE: usize = 16 * 1024 * 1024;
        const TOTAL: usize = 64 * 1024 * 1024;
        unsafe {
            let rdram = libc::mmap(
                std::ptr::null_mut(),
                TOTAL,
                libc::PROT_NONE,
                libc::MAP_ANONYMOUS | libc::MAP_PRIVATE,
                -1,
                0,
            );
            assert_ne!(rdram, libc::MAP_FAILED);
            assert_eq!(libc::mprotect(rdram, READABLE, libc::PROT_READ | libc::PROT_WRITE), 0);
            rdram.cast::<u8>().add(0x318).cast::<u32>().write(0xDEAD_BEEF);

            let pid = std::process::id();
            assert_eq!(find_rdram(pid, READABLE, TOTAL).unwrap(), Some(rdram as usize));

            let mut mem = ExternalMemory { pid: pid as libc::pid_t, rdram: rdram as usize };
            assert_eq!(n64::read_u32(&mut mem, 0x8000_0318), Some(0xDEAD_BEEF));
            assert_eq!(n64::read_u8(&mut mem, 0x8000_0318), Some(0xDE));

            libc::munmap(rdram, TOTAL);
        }
    }
}
