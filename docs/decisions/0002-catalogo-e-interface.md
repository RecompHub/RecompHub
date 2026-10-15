# 0002 — Pipeline do catálogo e interface do launcher

- **Data:** 15/10/2026
- **Status:** aceita

## Catálogo: gerado fora do launcher

```
catalog/sources.json        ← editado à mão (repo, título original, plataformas, ids opcionais)
        │
        ▼  tools/catalog-builder   (GitHub Action diário: .github/workflows/catalog.yml)
           ├─ GitHub      → estrelas, licença, 1ª release, última release, nº de releases, downloads
           ├─ SteamGridDB → capa 600x900, hero, logo, ícone
           └─ IGDB        → data de lançamento, desenvolvedora, publicadora, gêneros, resumo, nota, screenshots
        ▼
catalog/games.json          ← gerado; publicado via raw.githubusercontent.com
        │
        ▼
launcher → cache local (ETag) → fallback: cópia embutida no binário
```

**Por quê:** as chaves do SteamGridDB e do IGDB ficam só nos *secrets* do repositório;
o launcher não faz chamadas por jogo (o limite de 60 req/h do GitHub sem autenticação
acabaria com poucos jogos); a lista abre instantaneamente.

Regras do builder:
- Fonte que falha ou cuja chave falta **mantém os dados anteriores** daquele jogo.
- Sem `sgdbId`/`igdbId` no `sources.json`, busca pelo `originalTitle` (IGDB filtra pela
  plataforma original) e imprime o id encontrado, para ser fixado se estiver certo.
- `images` no `sources.json` substitui as do SGDB. Aceita URL ou caminho relativo em
  `catalog/images/`, que vira URL do catálogo publicado.
- `generatedAt` só muda quando o conteúdo muda (o Action não commita à toa).

Os tipos ficam em `crates/catalog`, compartilhados entre o builder e o launcher.

### Configuração dos secrets (GitHub → Settings → Secrets → Actions)

| Secret | Onde obter |
|--------|-----------|
| `SGDB_API_KEY` | https://www.steamgriddb.com/profile/preferences/api |
| `IGDB_CLIENT_ID`, `IGDB_CLIENT_SECRET` | Criar um app em https://dev.twitch.tv/console (categoria "Application Integration") |

`GITHUB_TOKEN` já é fornecido pelo Actions.

## Interface: inspirada na Steam

Menu horizontal no topo: **Início · Explorar · Biblioteca**, com botão de voltar e "Atualizar lista".

| Tela | Conteúdo |
|------|----------|
| Início | Destaque do recomp atualizado mais recentemente; prateleiras "Atualizações recentes", "Novos recomps" e "Mais populares" |
| Explorar | Filtros à esquerda (busca, ordenação, biblioteca, disponível no meu sistema, com conquistas, plataforma original, gênero); ordena por última atualização, lançamento do recomp, lançamento do jogo, nome, estrelas, downloads |
| Biblioteca | Lista à esquerda (instalados / não instalados); à direita hero, Jogar/Instalar/Atualizar, tempo de jogo, última vez, versão, notas da versão |
| Página do jogo | Hero + logo, adicionar à biblioteca, resumo, screenshots, notas da versão, ficha (recomp + jogo original), aviso de ROM |

A biblioteca é **local** (`library.json`) por enquanto. Um jogo pode estar na biblioteca
sem estar instalado; o tempo de jogo fica na entrada da biblioteca e sobrevive a reinstalações.
