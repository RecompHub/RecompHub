//! Localiza a RDRAM de uma recomp em execução e mostra alguns valores.
//!
//! ```sh
//! cargo run -p recomphub-memory --example rdram_probe -- <pid> [endereço...]
//! # ex.: ... -- 12345 0x80000318
//! ```

#[cfg(any(windows, target_os = "linux"))]
fn main() {
    use recomphub_memory::external::ExternalMemory;
    use recomphub_memory::{MemorySource, n64};

    let mut args = std::env::args().skip(1);
    let pid: u32 = args
        .next()
        .and_then(|a| a.parse().ok())
        .expect("uso: rdram_probe <pid> [endereço...]");

    let mut mem = match ExternalMemory::attach(pid) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    println!("RDRAM em 0x{:016X}", mem.rdram_address());

    // Quantos bytes da RDRAM já foram escritos pelo jogo (0 = ainda na tela da ROM).
    let mut chunk = vec![0u8; 64 * 1024];
    let mut nonzero = 0usize;
    for offset in (0..n64::RDRAM_SIZE).step_by(chunk.len()) {
        if mem.read(offset, &mut chunk) == chunk.len() {
            nonzero += chunk.iter().filter(|&&b| b != 0).count();
        }
    }
    println!("bytes não zerados nos 8 MiB: {nonzero}");

    for arg in args {
        let address = u32::from_str_radix(arg.trim_start_matches("0x"), 16).expect("endereço em hex");
        match n64::read_u32(&mut mem, address) {
            Some(v) => println!("0x{address:08X} = 0x{v:08X}"),
            None => println!("0x{address:08X} = (ilegível)"),
        }
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
fn main() {
    eprintln!("rdram_probe só funciona no Windows e no Linux");
}
