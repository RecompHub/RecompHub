//! Escreve um byte na RDRAM de uma recomp aberta, para testar troféus sem jogar.
//!
//! O endereço é o **mesmo dos troféus** (formato do RetroAchievements: a
//! memória crua, como a recomp guarda). `rdram_poke <pid> 0x318 5` faz a
//! condição `0xH000318=5` ficar verdadeira. No N64 esse byte é o `0x31B`
//! (`A ^ 3`, ver docs/research/rdram-n64recomp.md).
//!
//! ```sh
//! cargo run -p recomphub-memory --example rdram_poke -- <pid> <endereço RA> <valor>
//! # ex.: ... -- 12345 0x318 5
//! ```

#[cfg(windows)]
fn main() {
    use std::ffi::c_void;

    use recomphub_memory::external::{RECOMP_ALLOCATION_SIZE, RECOMP_MEM_SIZE, find_rdram};
    use recomphub_memory::n64;
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Diagnostics::Debug::WriteProcessMemory;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_OPERATION, PROCESS_VM_WRITE,
    };

    let args: Vec<String> = std::env::args().skip(1).collect();
    let [pid, address, value] = args.as_slice() else {
        eprintln!("uso: rdram_poke <pid> <endereço RA em hex> <valor>");
        std::process::exit(2);
    };
    let pid: u32 = pid.parse().expect("pid");
    let address = u32::from_str_radix(address.trim_start_matches("0x"), 16).expect("endereço em hex");
    let value: u8 = value.parse().expect("valor 0-255");

    unsafe {
        let process = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_OPERATION | PROCESS_VM_WRITE, 0, pid);
        assert!(!process.is_null(), "não foi possível abrir o processo");
        let rdram = find_rdram(process, RECOMP_MEM_SIZE, RECOMP_ALLOCATION_SIZE).expect("RDRAM não encontrada");
        let target = rdram + n64::physical(address) as usize;
        let mut written = 0;
        let ok = WriteProcessMemory(process, target as *const c_void, (&value as *const u8).cast(), 1, &mut written);
        CloseHandle(process);
        assert!(ok != 0, "falha ao escrever");
    }
    println!("0x{address:08X} = {value}");
}

#[cfg(not(windows))]
fn main() {
    eprintln!("rdram_poke por enquanto só funciona no Windows");
}
