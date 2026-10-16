# RDRAM nas recomps do N64Recomp

- **Data:** 15/10/2026
- **Fontes:** `N64Recomp/N64Recomp` (`include/recomp.h`) e `N64Recomp/N64ModernRuntime`
  (`librecomp/src/recomp.cpp`, `librecomp/include/librecomp/addresses.hpp`), branch `main`.

## Como a memória é alocada

Em `recomp::start` (`recomp.cpp`), **antes** da tela de seleção de ROM:

```cpp
// addresses.hpp
constexpr size_t mem_size        = 512ULL  * 1024 * 1024; // 512 MiB
constexpr size_t allocation_size = 4096ULL * 1024 * 1024; // 4 GiB

// Windows
rdram = VirtualAlloc(nullptr, allocation_size, MEM_COMMIT | MEM_RESERVE, PAGE_NOACCESS);
VirtualProtect(rdram, mem_size, PAGE_READWRITE, &old);
// Linux
rdram = mmap(NULL, allocation_size, PROT_NONE, MAP_ANON | MAP_PRIVATE, -1, 0);
mprotect(rdram, mem_size, PROT_READ | PROT_WRITE);
```

O endereço muda a cada execução (ASLR), mas **o formato da alocação é único e fixo**:

| Trecho | Tamanho | Proteção |
|--------|---------|----------|
| `rdram + 0` | 512 MiB | leitura/escrita |
| `rdram + 512 MiB` | 3,5 GiB | sem acesso |

A mesma alocação de 4 GiB, com os primeiros 512 MiB legíveis. Dá para localizar de fora:
- **Windows:** percorrer as regiões com `VirtualQueryEx` e procurar uma `AllocationBase` com
  essas duas regiões consecutivas.
- **Linux:** em `/proc/<pid>/maps`, um mapeamento anônimo `rw-p` de 512 MiB seguido de um
  `---p` de 3,5 GiB adjacente.

**Consequência:** a ideia inicial de achar ponteiros/offsets **por versão** de cada recomp
(seção 3.5 do CLAUDE.md) provavelmente não é necessária. O localizador vale para qualquer
recomp que use o N64ModernRuntime atual. Se o runtime mudar esses tamanhos, só o localizador muda.

## Endereçamento e ordem dos bytes

`rdram` corresponde ao endereço virtual `0x80000000` (KSEG0). Macros de `recomp.h`:

```c
#define MEM_W(off, reg)  (*(int32_t*)(rdram + (((reg) + (off))     - 0xFFFFFFFF80000000)))
#define MEM_H(off, reg)  (*(int16_t*)(rdram + ((((reg) + (off)) ^ 2) - 0xFFFFFFFF80000000)))
#define MEM_B(off, reg)  (*(int8_t*) (rdram + ((((reg) + (off)) ^ 3) - 0xFFFFFFFF80000000)))
```

Para um endereço físico `A` do N64 (0x000000–0x7FFFFF):

| Leitura | Onde no processo | Formato |
|---------|------------------|---------|
| 32 bits | `rdram + A` | little-endian (nativo do PC) |
| 16 bits | `rdram + (A ^ 2)` | little-endian |
| 8 bits | `rdram + (A ^ 3)` | — |

Ou seja: a RDRAM está guardada **em palavras de 32 bits com os bytes invertidos** em relação
ao console (big-endian). É o mesmo formato "word-swapped" que os emuladores de N64 (mupen64plus)
costumam usar internamente.

## Relação com o RetroAchievements

No `rcheevos` (`src/runtime/rc_consoleinfo.c`), o mapa do N64 é:

| Endereço RA | Endereço real | Região |
|-------------|---------------|--------|
| `0x000000–0x1FFFFF` | `0x80000000` | RDRAM 1 |
| `0x200000–0x3FFFFF` | `0x80200000` | RDRAM 2 |
| `0x400000–0x7FFFFF` | `0x80400000` | Expansion Pak |

Os endereços batem 1:1 com `rdram + A`. **Falta confirmar** se o RA espera os bytes no formato
"word-swapped" (como a recomp guarda) ou já convertidos. Se for o mesmo formato, a fonte de memória
pode entregar `rdram + A` direto para a rcheevos, sem conversão. O teste é ler um valor conhecido
(ex.: vida ou rupias no Majora's Mask) pelos dois caminhos.

## Limitação da recomp

`osGetMemSize_recomp` sempre responde 8 MiB, então o Expansion Pak está sempre "presente".
A região de 8 MiB a 512 MiB é usada pelo runtime e por mods (heap, `mod_rdram_start`), não
pelo jogo original. Para conquistas, interessa só `0x000000–0x7FFFFF`.

## Próximos passos

1. Localizador de RDRAM (Windows) + `MemorySource` externa, com a conversão de endereço acima.
2. Validar no processo real de uma recomp (a alocação existe já na tela de seleção de ROM).
3. Com uma ROM: ler valores conhecidos e confirmar a ordem dos bytes esperada pelo RA.
