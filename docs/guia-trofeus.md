# Guia: criar troféus (Fase 1, experimental)

Enquanto não existe servidor, os troféus de cada jogo ficam num arquivo local,
`%LOCALAPPDATA%\com.recomphub.launcher\trophies\<id do jogo>.json`, e são avaliados
pelo launcher enquanto o jogo roda.

## 1. Criar o arquivo

Na **Biblioteca**, abra o jogo e clique em **Criar troféus**. O launcher cria um modelo e
abre no editor padrão:

```json
{
  "gameId": "zelda64-mm",
  "trophies": [
    {
      "id": 1,
      "title": "Exemplo",
      "description": "Troca as condições por um endereço achado com o rdram_probe",
      "points": 5,
      "conditions": "0xH000000=1"
    }
  ]
}
```

- `id`: número único dentro do jogo (é o que fica salvo como desbloqueado; não reaproveite).
- `conditions`: condições no **formato do RetroAchievements**
  ([documentação](https://docs.retroachievements.org/developer-docs/condition-syntax.html)).
  Ex.: `0xH01EF6A0=99` (8 bits), `0x 01EF6A0>=100` (16 bits), `0xX01EF6A0=1` (32 bits),
  `_` separa condições que precisam ser verdadeiras ao mesmo tempo.

Depois de salvar, clique em **Recarregar**. Condições com erro aparecem em vermelho.

## 2. Achar os endereços

Com o jogo aberto (pelo launcher ou direto), pegue o PID no Gerenciador de Tarefas
(aba Detalhes) e rode na pasta do projeto:

```sh
cargo run -p recomphub-memory --example rdram_probe -- <pid> 0x8001EF6A0
```

O `rdram_probe` mostra valores **como o N64 os vê** (endereços de documentação/mapas de RAM,
`0x80xxxxxx`). Os troféus usam o **endereço cru do RetroAchievements**. A conversão
depende do tamanho:

| Valor no N64 (endereço `A`) | Condição do troféu |
|-----------------------------|--------------------|
| 32 bits | `0xX` + `A` (mesmo endereço) |
| 16 bits | `0x ` + `A ^ 2` |
| 8 bits  | `0xH` + `A ^ 3` |

Exemplo: um contador de 8 bits em `0x801EF6A0` (N64) vira `0xH1EF6A3` (`0x1EF6A0 ^ 3`).

> Ainda não confirmamos que o RetroAchievements usa esse mesmo endereçamento para o N64.
> Ver `docs/research/rdram-n64recomp.md`.

## 3. Testar sem jogar

Com o jogo aberto pelo launcher, dá para forçar um valor e ver o troféu desbloquear:

```sh
cargo run -p recomphub-memory --example rdram_poke -- <pid> <endereço da condição> <valor>
# ex.: condição 0xH000318=5  →  rdram_poke <pid> 0x318 5
```

A rcheevos só desbloqueia depois de ver a condição **falsa** pelo menos uma vez (para não
desbloquear ao carregar um save). Por isso o troféu precisa começar falso.

## Limitações conhecidas

- Avaliação por amostragem (~60×/s), não sincronizada com os frames do jogo: eventos que
  duram um único frame podem ser perdidos.
- Desbloqueios ficam só no seu PC (`library.json`) até existir servidor.
