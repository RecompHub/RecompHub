//! Prova de conceito da Fase 1: lê a RDRAM de uma recomp aberta e avisa
//! quando uma conquista (condições no formato do RA) desbloqueia.
//!
//! ```sh
//! cargo run -p recomphub-achievements --example watch -- <pid> "<condições>" ["<condições>"...]
//! # ex.: ... -- 12345 "0xH1EF6A0=99"
//! ```
//!
//! A avaliação é por amostragem (~60 vezes por segundo), não sincronizada
//! com os frames do jogo: eventos de um único frame podem ser perdidos.

#[cfg(any(windows, target_os = "linux"))]
fn main() {
    use std::time::Duration;

    use recomphub_achievements::{Event, Runtime};
    use recomphub_memory::external::ExternalMemory;

    let mut args = std::env::args().skip(1);
    let pid: u32 = args
        .next()
        .and_then(|a| a.parse().ok())
        .expect("uso: watch <pid> \"<condições>\" ...");

    let mut runtime = Runtime::new();
    let mut count = 0;
    for (i, conditions) in args.enumerate() {
        let id = i as u32 + 1;
        match runtime.activate(id, &conditions) {
            Ok(()) => {
                println!("#{id}: {conditions}");
                count += 1;
            }
            Err(e) => eprintln!("#{id}: condição inválida: {e}"),
        }
    }
    if count == 0 {
        eprintln!("nenhuma conquista válida");
        std::process::exit(1);
    }

    let mut memory = match ExternalMemory::attach(pid) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    println!("RDRAM em 0x{:X}; observando…", memory.rdram_address());

    let mut unlocked = 0;
    while unlocked < count {
        for event in runtime.do_frame(&mut memory) {
            match event {
                Event::Triggered(id) => {
                    unlocked += 1;
                    println!("🏆 conquista #{id} desbloqueada");
                }
                Event::Activated(id) => println!("#{id} ativa (condições ficaram falsas)"),
                Event::Disabled(id) => println!("#{id} desativada: endereço inválido"),
                _ => {}
            }
        }
        std::thread::sleep(Duration::from_millis(16));
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
fn main() {
    eprintln!("watch só funciona no Windows e no Linux");
}
