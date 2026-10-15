//! Fonte de memória trocável usada pelo motor de conquistas.
//!
//! | Fase | Implementação                                             |
//! |------|-----------------------------------------------------------|
//! | 1    | Leitura externa do processo (ReadProcessMemory / process_vm_readv) |
//! | 2    | Mod na recomp expõe a RDRAM por memória compartilhada     |
//! | 3    | rcheevos roda dentro da recomp (o launcher só exibe)      |

/// Acesso à RAM do console emulado, em endereços do próprio console
/// (ex.: `0x0000_0000..0x0080_0000` na RDRAM do N64), já com a ordem de bytes
/// corrigida pela implementação.
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
