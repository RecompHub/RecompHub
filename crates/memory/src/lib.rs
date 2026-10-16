//! Fonte de memória trocável do motor de conquistas.
//!
//! | Fase | Implementação                                                      |
//! |------|--------------------------------------------------------------------|
//! | 1    | Leitura externa do processo ([`external`]: Windows e Linux)        |
//! | 2    | Mod na recomp expõe a RDRAM por memória compartilhada              |
//! | 3    | rcheevos roda dentro da recomp (o launcher só exibe)               |
//!
//! Layout da RDRAM nas recomps: `docs/research/rdram-n64recomp.md`.

pub mod n64;

pub mod external;

/// Acesso à RAM do console, em endereços físicos do próprio console
/// (`0x000000..0x800000` na RDRAM do N64).
///
/// Os bytes saem **como a recomp os guarda** (palavras de 32 bits em
/// little-endian); o módulo [`n64`] converte para valores do console.
pub trait MemorySource: Send {
    /// Lê `buf.len()` bytes a partir de `address`. Retorna quantos bytes foram
    /// lidos; 0 indica endereço inválido (é o que a rcheevos espera do callback).
    fn read(&mut self, address: u32, buf: &mut [u8]) -> usize;

    /// `true` quando a fonte sabe sinalizar fim de frame (fases 2 e 3).
    /// Na fase 1 a avaliação é feita por amostragem de tempo.
    fn is_frame_synced(&self) -> bool {
        false
    }
}

/// Fonte em memória, para testes e para validar conquistas offline.
pub struct BufferSource(pub Vec<u8>);

impl MemorySource for BufferSource {
    fn read(&mut self, address: u32, buf: &mut [u8]) -> usize {
        let start = address as usize;
        let Some(src) = self.0.get(start..start + buf.len()) else {
            return 0;
        };
        buf.copy_from_slice(src);
        buf.len()
    }
}
