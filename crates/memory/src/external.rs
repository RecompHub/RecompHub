//! Fase 1: lê a RDRAM de fora do processo da recomp (Windows).
//!
//! O runtime reserva 4 GiB e deixa só os primeiros 512 MiB legíveis
//! (`librecomp/src/recomp.cpp`); essa assinatura identifica a RDRAM sem
//! depender da versão do jogo.

use std::ffi::c_void;
use std::mem::{MaybeUninit, size_of};

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::System::Diagnostics::Debug::ReadProcessMemory;
use windows_sys::Win32::System::Memory::{
    MEM_COMMIT, MEMORY_BASIC_INFORMATION, PAGE_NOACCESS, PAGE_READWRITE, VirtualQueryEx,
};
use windows_sys::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
};

use crate::MemorySource;
use crate::n64::RDRAM_SIZE;

/// Tamanhos de `librecomp/include/librecomp/addresses.hpp`.
pub const RECOMP_MEM_SIZE: usize = 512 * 1024 * 1024;
pub const RECOMP_ALLOCATION_SIZE: usize = 4096 * 1024 * 1024;

#[derive(Debug)]
pub enum Error {
    /// `OpenProcess` falhou (processo fechou ou sem permissão).
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

/// Leitura da RDRAM de outro processo (ou do próprio, nos testes).
pub struct ExternalMemory {
    process: HANDLE,
    owned: bool,
    rdram: usize,
}

// O HANDLE só é usado por quem possui a ExternalMemory.
unsafe impl Send for ExternalMemory {}

impl ExternalMemory {
    /// Abre o processo da recomp e localiza a RDRAM.
    pub fn attach(pid: u32) -> Result<Self, Error> {
        let process = unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, pid) };
        if process.is_null() {
            return Err(Error::OpenProcess(std::io::Error::last_os_error()));
        }
        let mut memory = Self { process, owned: true, rdram: 0 };
        memory.rdram = find_rdram(process, RECOMP_MEM_SIZE, RECOMP_ALLOCATION_SIZE)
            .ok_or(Error::RdramNotFound)?;
        Ok(memory)
    }

    /// Endereço da RDRAM no processo (muda a cada execução por causa do ASLR).
    pub fn rdram_address(&self) -> usize {
        self.rdram
    }
}

impl Drop for ExternalMemory {
    fn drop(&mut self) {
        if self.owned {
            unsafe { CloseHandle(self.process) };
        }
    }
}

impl MemorySource for ExternalMemory {
    fn read(&mut self, address: u32, buf: &mut [u8]) -> usize {
        let end = address as u64 + buf.len() as u64;
        if end > RDRAM_SIZE as u64 {
            return 0;
        }
        let mut read = 0usize;
        let ok = unsafe {
            ReadProcessMemory(
                self.process,
                (self.rdram + address as usize) as *const c_void,
                buf.as_mut_ptr().cast(),
                buf.len(),
                &mut read,
            )
        };
        if ok == 0 { 0 } else { read }
    }
}

fn query(process: HANDLE, address: usize) -> Option<MEMORY_BASIC_INFORMATION> {
    let mut info = MaybeUninit::<MEMORY_BASIC_INFORMATION>::zeroed();
    let len = unsafe {
        VirtualQueryEx(
            process,
            address as *const c_void,
            info.as_mut_ptr(),
            size_of::<MEMORY_BASIC_INFORMATION>(),
        )
    };
    (len != 0).then(|| unsafe { info.assume_init() })
}

/// Procura uma alocação de `total` bytes cujos primeiros `readable` bytes são
/// leitura/escrita e o resto é sem acesso.
pub fn find_rdram(process: HANDLE, readable: usize, total: usize) -> Option<usize> {
    let mut address = 0usize;
    while let Some(region) = query(process, address) {
        let base = region.BaseAddress as usize;
        let is_candidate = region.State == MEM_COMMIT
            && region.Protect == PAGE_READWRITE
            && region.AllocationBase == region.BaseAddress
            && region.RegionSize == readable;
        if is_candidate {
            if let Some(guard) = query(process, base + readable) {
                if guard.AllocationBase as usize == base
                    && guard.Protect == PAGE_NOACCESS
                    && guard.RegionSize == total - readable
                {
                    return Some(base);
                }
            }
        }
        address = base.checked_add(region.RegionSize)?;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::n64;
    use windows_sys::Win32::System::Memory::{
        MEM_RELEASE, MEM_RESERVE, VirtualAlloc, VirtualFree, VirtualProtect,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    /// Reproduz a alocação do runtime, em tamanho menor, no próprio processo.
    #[test]
    fn finds_rdram_shaped_allocation_and_reads_console_values() {
        const READABLE: usize = 16 * 1024 * 1024;
        const TOTAL: usize = 64 * 1024 * 1024;
        unsafe {
            let rdram = VirtualAlloc(std::ptr::null(), TOTAL, MEM_COMMIT | MEM_RESERVE, PAGE_NOACCESS);
            assert!(!rdram.is_null());
            let mut old = 0;
            assert_ne!(VirtualProtect(rdram, READABLE, PAGE_READWRITE, &mut old), 0);

            // `sw 0xDEADBEEF` em 0x80000318, como a recomp gravaria.
            let word = rdram.cast::<u8>().add(0x318).cast::<u32>();
            word.write(0xDEAD_BEEF);

            let process = GetCurrentProcess();
            let found = find_rdram(process, READABLE, TOTAL);
            assert_eq!(found, Some(rdram as usize));

            let mut mem = ExternalMemory { process, owned: false, rdram: rdram as usize };
            assert_eq!(n64::read_u32(&mut mem, 0x8000_0318), Some(0xDEAD_BEEF));
            assert_eq!(n64::read_u8(&mut mem, 0x8000_0318), Some(0xDE));
            assert_eq!(mem.read(n64::RDRAM_SIZE, &mut [0u8; 1]), 0);

            VirtualFree(rdram, 0, MEM_RELEASE);
        }
    }
}
