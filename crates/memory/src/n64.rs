//! Leitura de valores do N64 a partir da RDRAM no formato da recomp.
//!
//! A recomp guarda cada palavra de 32 bits em little-endian (ver `MEM_W`,
//! `MEM_H` e `MEM_B` em `N64Recomp/include/recomp.h`):
//! - 32 bits em `A`
//! - 16 bits em `A ^ 2`
//! - 8 bits em `A ^ 3`

use crate::MemorySource;

/// RDRAM + Expansion Pak, como no mapa de memória do RetroAchievements.
pub const RDRAM_SIZE: u32 = 0x80_0000;

/// Converte um endereço virtual KSEG0/KSEG1 (`0x80xxxxxx`/`0xA0xxxxxx`) ou
/// físico para o endereço físico usado pela [`MemorySource`].
pub fn physical(address: u32) -> u32 {
    address & 0x1FFF_FFFF
}

pub fn read_u8(mem: &mut dyn MemorySource, address: u32) -> Option<u8> {
    let mut b = [0u8; 1];
    (mem.read(physical(address) ^ 3, &mut b) == 1).then_some(b[0])
}

/// `address` precisa estar alinhado a 2, como no console.
pub fn read_u16(mem: &mut dyn MemorySource, address: u32) -> Option<u16> {
    let mut b = [0u8; 2];
    (mem.read(physical(address) ^ 2, &mut b) == 2).then(|| u16::from_le_bytes(b))
}

/// `address` precisa estar alinhado a 4, como no console.
pub fn read_u32(mem: &mut dyn MemorySource, address: u32) -> Option<u32> {
    let mut b = [0u8; 4];
    (mem.read(physical(address), &mut b) == 4).then(|| u32::from_le_bytes(b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BufferSource;

    /// Simula o que a recomp faz ao executar `sw 0x11223344` em 0x80000100:
    /// a palavra fica em little-endian no buffer.
    fn memory_with_word_at_0x100() -> BufferSource {
        let mut ram = vec![0u8; 0x200];
        ram[0x100..0x104].copy_from_slice(&0x1122_3344u32.to_le_bytes());
        BufferSource(ram)
    }

    #[test]
    fn reads_match_console_big_endian_view() {
        let mut mem = memory_with_word_at_0x100();
        // No console (big-endian) os bytes em 0x100..0x104 são 11 22 33 44.
        assert_eq!(read_u32(&mut mem, 0x8000_0100), Some(0x1122_3344));
        assert_eq!(read_u16(&mut mem, 0x8000_0100), Some(0x1122));
        assert_eq!(read_u16(&mut mem, 0x8000_0102), Some(0x3344));
        assert_eq!(read_u8(&mut mem, 0x8000_0100), Some(0x11));
        assert_eq!(read_u8(&mut mem, 0x8000_0103), Some(0x44));
    }

    #[test]
    fn kseg1_and_physical_addresses_are_equivalent() {
        let mut mem = memory_with_word_at_0x100();
        assert_eq!(read_u32(&mut mem, 0xA000_0100), read_u32(&mut mem, 0x100));
    }

    #[test]
    fn out_of_range_returns_none() {
        let mut mem = memory_with_word_at_0x100();
        assert_eq!(read_u32(&mut mem, 0x8000_1000), None);
    }
}
