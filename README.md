# RecompHub

Launcher para jogos recomp (ports nativos de PC, ex.: projetos baseados em N64Recomp),
com sistema de conquistas compatível com o formato do RetroAchievements.

> Você usa a sua própria ROM. O RecompHub nunca distribui ROMs.

Contexto e planejamento: [CLAUDE.md](CLAUDE.md) · Decisões: [docs/decisions/](docs/decisions/)

## Estrutura

```
catalog/                sources.json (editado à mão) e games.json (gerado)
crates/catalog/         tipos do catálogo, compartilhados
tools/catalog-builder/  gera games.json (GitHub + SteamGridDB + IGDB)
app/                    launcher (Tauri 2: Rust em src-tauri/, React + TS em src/)
server/                 (futuro) API de contas e conquistas
docs/                   pesquisas, offsets por versão, decisões
```

## Desenvolvimento

Requisitos (só para quem desenvolve): Rust stable, Node.js LTS, Visual Studio Build Tools com C++.

```sh
cd app
npm install
npm run tauri dev      # abre o launcher em modo desenvolvimento
npm run tauri build    # gera o instalador
```

Depois de clonar: `git submodule update --init` (baixa a rcheevos).

Testes do Rust (na raiz): `cargo test --workspace`.

### Catálogo

Para adicionar um jogo, edite `catalog/sources.json` e gere o `games.json`:

```sh
# chaves opcionais; sem elas, imagens e dados do jogo ficam como estavam
export SGDB_API_KEY=...          # https://www.steamgriddb.com/profile/preferences/api
export IGDB_CLIENT_ID=...        # app em https://dev.twitch.tv/console
export IGDB_CLIENT_SECRET=...
cargo run -p catalog-builder
```

No GitHub, o workflow `catalog.yml` faz isso diariamente com os mesmos nomes como *secrets*.
Detalhes: [docs/decisions/0002-catalogo-e-interface.md](docs/decisions/0002-catalogo-e-interface.md).

## Distribuição

`npm run tauri build` gera:

- Windows: `target/release/bundle/nsis/RecompHub_<versão>_x64-setup.exe`
- Linux: `.AppImage` e `.deb` em `target/release/bundle/`

O instalador do Windows não exige admin nem programas pré-instalados (ver
[docs/decisions/0001-stack.md](docs/decisions/0001-stack.md)). Pastas padrão no Windows:

| O quê | Onde |
|-------|------|
| Launcher | `C:\RecompHub` (template NSIS próprio em `app/src-tauri/nsis/installer.nsi`) |
| Jogos | `C:\RecompHub\Games\<id>` (configurável) |
| Biblioteca, cache, troféus locais | `%LOCALAPPDATA%\com.recomphub.launcher` |

O desinstalador remove só os arquivos do launcher; `C:\RecompHub\Games` e a biblioteca
sobrevivem. Em cada pasta de jogo, o launcher registra em `.recomphub-files.json` os
arquivos que instalou, e atualizar ou desinstalar o jogo mexe **só** nesses: ROM, saves e
mods do usuário na mesma pasta ficam intactos. Pastas de jogo copiadas à mão para
`C:\RecompHub\Games\<id>` são detectadas e importadas ao abrir o launcher.

Build local com artefatos do updater (precisa da chave privada):

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content -Raw "$env:USERPROFILE\.tauri\recomphub.key"
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ""
cd app; npm run tauri build
```

### Publicar uma versão

1. Altere `version` em `app/src-tauri/Cargo.toml` (é a fonte única da versão) e commite.
2. `git tag v0.2.0 && git push origin v0.2.0`
3. O workflow `release.yml` gera o instalador Windows e o AppImage/.deb, assina e cria
   um **rascunho** em Releases com o `latest.json` do updater.
4. Revise e publique. A partir daí o launcher instalado mostra "Atualizar e reiniciar".

Secrets necessários no GitHub: `TAURI_SIGNING_PRIVATE_KEY` (conteúdo de
`~/.tauri/recomphub.key`) e, se a chave tiver senha, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.

> **Guarde um backup da chave privada.** Sem ela não é possível publicar atualizações
> que os launchers já instalados aceitem.
