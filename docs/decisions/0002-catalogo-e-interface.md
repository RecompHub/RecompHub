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

Menu horizontal no topo: **Início · Explorar · Biblioteca**, com botão de voltar, "Atualizar lista" e engrenagem (Configurações).

Tema **preto e verde** (verde `#76b900`), cantos retos, inspirado no app da NVIDIA. Cores em variáveis CSS no topo de `app/src/App.css`.

| Tela | Conteúdo |
|------|----------|
| Início | Destaque do recomp atualizado mais recentemente; prateleiras "Atualizações recentes", "Novos recomps" e "Mais populares" |
| Explorar | Filtros à esquerda (busca, ordenação, biblioteca, disponível no meu sistema, com conquistas, plataforma original, gênero); ordena por última atualização, lançamento do recomp, lançamento do jogo, nome, estrelas, downloads |
| Biblioteca | Lista à esquerda (instalados / não instalados); à direita hero, Jogar/Instalar/Atualizar, tempo de jogo, última vez, versão, notas da versão |
| Configurações | Pasta de instalação dos jogos (só afeta novas instalações), versão do launcher, procurar atualizações |
| Página do jogo | Hero + logo, adicionar à biblioteca, resumo, screenshots, notas da versão, ficha (recomp + jogo original), aviso de ROM |

A biblioteca é **local** (`library.json`) por enquanto. Um jogo pode estar na biblioteca
sem estar instalado; o tempo de jogo fica na entrada da biblioteca e sobrevive a reinstalações.

## Troféus do próprio jogo (provedores)

Alguns ports já detectam conquistas por conta própria. O **Dusklight** (Twilight Princess,
port da decompilação, CC0) grava `%APPDATA%\TwilitRealm\Dusklight\achievements.json`
(`{ "<chave>": { "progress": n, "unlocked": bool } }`) no mesmo frame do desbloqueio, mas
não expõe as conquistas para mods ("Until we implement an AchievementService…").

Por isso o jogo ganha um `trophyProvider` no `sources.json`:

```json
"trophyProvider": {
  "kind": "dusklight",
  "progressFile": { "windows": "%APPDATA%/TwilitRealm/Dusklight/achievements.json", "linux": "~/.local/share/..." },
  "definitions": "trophies/dusklight.json"
}
```

- `catalog/trophies/dusklight.json` é gerado por `tools/extract-dusklight-achievements.mjs`
  a partir de `src/dusk/achievements.cpp` (nome, descrição, categoria, meta dos contadores).
  O builder copia as definições para o `games.json`. Rodar de novo quando sair versão nova.
- Enquanto o jogo roda pelo RecompHub, o launcher acompanha o arquivo de progresso e, a cada
  conquista nova, grava a data na biblioteca e mostra notificação **do sistema** (a janela do
  launcher fica atrás do jogo) e na própria interface.
- Não usamos a lista do RetroAchievements: o CLAUDE.md proíbe copiar sets do RA, e as
  condições do RA são para a memória do Dolphin, que não existe num port nativo.

Próximo passo possível: um mod nativo do Dusklight (SDK + `WebSocketService`) para eventos
que o arquivo não tem, como a área atual (rich presence).

### Atualização (16/10/2026): lista manual; automático guardado

Decisão do usuário: **por enquanto, conquistas são uma lista que o usuário marca à mão**.
A detecção automática fica para o futuro.

- Pesquisa nos 14 repositórios do catálogo: **só o Dusklight tem conquistas próprias**
  (as 13 recomps de N64 não têm). A lista do RetroAchievements **não** é usada.
- `get_trophies` devolve as conquistas do `trophyProvider` (catálogo); `set_trophy` marca e
  desmarca, gravando em `LibraryEntry::trophies` (chave → data).
- Todo o código automático (arquivo de progresso do Dusklight, leitura da RDRAM + rcheevos,
  troféus com condições escritos pelo usuário) foi para `app/src-tauri/src/auto_trophies.rs`,
  atrás da feature **`auto-trophies`** (desligada por padrão; `crates/memory` e
  `crates/achievements` viram dependências opcionais). O CI compila e testa os dois modos.
  Ligar a feature grava no mesmo lugar das marcações manuais, sem mudar a interface.

## Atualização (17/10/2026): jogos com instalador próprio

Alguns projetos não publicam o jogo pronto, e sim um instalador que compila o jogo no PC do usuário a partir do disco dele (ex.: WiiCompiled/Mario Kart Wii, Nebula/Super Mario Galaxy). Para eles, a plataforma no `sources.json` ganha `installer`:

```json
"windows-x64": {
  "assetPattern": "WiiCompiled-Setup.exe",
  "executable": "WiiCompiled.exe",
  "installer": {
    "installDir": "%LOCALAPPDATA%/Programs/WiiCompiled",
    "args": ["--silent", "--progress-json", "--game", "{rom}"],
    "progressJson": true
  }
}
```

- O launcher baixa o instalador, pede a imagem do disco quando os argumentos têm `{rom}` e executa. Com `progressJson`, roda sem janela e lê o progresso em NDJSON; sem, abre a janela do próprio instalador e espera ela fechar.
- Depois procura `executable` em `installDir`. Quando há várias pastas de versão, fica com o executável mais recente, e essa busca é refeita a cada vez que o jogo abre.
- Os arquivos são do instalador: "Remover do launcher" não apaga nada. Quem desinstala é o próprio projeto, em Configurações do Windows → Aplicativos.
- Jogos já instalados por fora aparecem sozinhos na biblioteca (`scan_games_dir`).
- Pacotes `.msi` (ex.: OpenGOAL Launcher, usado por Jak 1 e Jak II) rodam com `msiexec /i`; o código 3010 (pede reinício) conta como sucesso.
- Código: `app/src-tauri/src/setup.rs`.
