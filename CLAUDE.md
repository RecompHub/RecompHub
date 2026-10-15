# RecompHub — Contexto do Projeto

> Documento de planejamento. Etapa 0 concluída (stack escolhida, projeto criado); Etapa 1 a iniciar.
> Use este arquivo como referência inicial (e como contexto para assistentes de IA no VSCode).

---

## 0. Nome e identidade

### Decisão de nomes
| Uso | Nome |
|-----|------|
| Projeto, launcher, repositório | **RecompHub** |
| Site / marca pública (futuro) | **Akmon** |

- **RecompHub**: nome descritivo, entendido de imediato pela comunidade ("o hub das recomps") e fácil de achar em buscas. Nenhum projeto com esse nome encontrado em 15/10/2026.
  - Limitações conhecidas: prende o escopo às recomps (ports de decompilação ficariam "fora" do nome); "recomp" também é jargão fitness (*body recomposition*).
- **Akmon**: reservado como marca do site. Possível combinação futura: *"Akmon — the recomp hub"*.

### Akmon (ἄκμων)
Nome reservado para o site/marca pública.

- **Significado literal:** "bigorna" em grego antigo — onde o metal é forjado e ganha nova forma.
- **Significado associado:** "incansável" — uma etimologia tradicional liga a palavra a *a-* ("não") + *kámnō* ("cansar-se"): a bigorna que nunca se cansa de ser golpeada.
- **Leitura semântica do projeto:**
  - *Forja* → as recomps são jogos antigos **reforjados** pela comunidade.
  - *Incansável* → a vontade incansável dos jogadores e desenvolvedores em **reviver jogos antigos**.
- Na mitologia, Akmon também é o nome de um titã/daímon ligado à forja.

### Terminologia (provisória)
| Conceito | Nome atual | Observação |
|----------|-----------|------------|
| Projeto / launcher | **RecompHub** | Definido |
| Site / marca | **Akmon** | Definido (quando houver site) |
| Conquistas | **Troféus** | Provisório — candidatos discutidos: Ingot/Lingote, Plinthos (πλίνθος, "lingote" no grego antigo) |
| XP / nível | **XP** | Provisório — implementar só depois que os troféus funcionarem. Candidatos: Spark/Faísca, Spinther (σπινθήρ) |

Termos guardados para uso interno futuro (não precisam ser únicos): **Hoard** (coleção de troféus do usuário), **Foundry** (catálogo de recomps).

### Nomes descartados (e por quê)
- **Kiln** — jogo homônimo da Double Fine/Xbox (2026), marca da Microsoft; também app Kiln AI.
- **Anvil** — Anvil Organizer (gerenciador de mods Linux), jogo ANVIL, anvil.works.
- **Forge / Foundry** — saturados (Minecraft Forge, Foundry VTT etc.).
- **Hoard** — jogo HOARD (2010), mesmo tema de dragão; marca registrada.
- **Wyrm** — livre, mas trocado por Akmon.
- **Renew Retro** — criador "RetroRenew" (~32 mil seguidores) no mesmo nicho.
- **Renew Recomp** — redundante ("renovar a renovação"); "renew" soa como renovar assinatura.
- **Renewable Source / Dynamo** — domínios .com ocupados; nomes saturados (setor de energia, Dynamo Software, DynamoDB).
- **Bessemer / Flywheel** — sugeridos, não verificados.

### Posicionamento
"O launcher de recomps feito do jeito certo: com a sua ROM, direto dos repositórios oficiais." Evitar qualquer associação pública com launchers de pirataria (ex.: Hydra Launcher).

### Checagem do nome (15/10/2026)
- **Software/jogos:** nenhum app, launcher, jogo ou ferramenta relevante chamado "Akmon" encontrado. Resultados são da mitologia (Acmon/Akmon, um dos Dáctilos, "bigorna") e nomes parecidos sem relação (Akumon Summoner, Akmos).
- **akmon.com:** **registrado** desde 2006 pela registradora coreana Gabia, com bloqueio de transferência/exclusão. Vencimento constava como 25/09/2026 (pode estar renovado ou em período de carência). Não está anunciado à venda.
- **akmon.app / .gg / .dev:** sem registro encontrado no WHOIS, mas a consulta foi inconclusiva → confirmar direto em um registrador.

### Domínios (referência de preços — Cloudflare Registrar, out/2026, a preço de custo)
- .com ~US$ 10,46/ano · .dev ~US$ 12,20/ano na renovação · .app ~US$ 14,20/ano na renovação
- .gg provavelmente não vendido pela Cloudflare (mais caro em outros registradores)
- Cada extensão é um registro independente: registrar uma não protege as outras.
- Sugestão quando houver site: **akmon.app** ou **akmon.gg** + **akmon.com.br** (proteção).

### Pendências do nome
- [ ] Criar a organização/repositório "RecompHub" no GitHub.
- [ ] Confirmar disponibilidade de akmon.app / .gg / .dev / .com.br em um registrador (quando for fazer o site).
- [ ] (Opcional) Monitorar akmon.com caso expire, ou consultar o dono via serviço de corretagem.
- [ ] (Opcional) Reservar a organização "akmon" no GitHub para o futuro.
- [ ] Busca de marcas registradas (INPI no Brasil; USPTO/EUIPO se for internacional).

---

## 1. Visão geral

**RecompHub** é um **launcher para jogos recomp/ports nativos de PC** (ex.: projetos baseados em N64Recomp), semelhante ao N64RecompLauncher, com um diferencial: **sistema de conquistas próprio**, desenhado desde o início para ser compatível com o formato do **RetroAchievements (RA)**, permitindo uma possível integração futura.

### Objetivos
1. Listar os jogos recomp disponíveis (catálogo).
2. Baixar releases diretamente dos repositórios GitHub de cada projeto.
3. Identificar e gerenciar a versão instalada de cada jogo no PC.
4. Iniciar o executável do jogo a partir do launcher.
5. (Fase 1+) Detectar conquistas lendo a memória do jogo e registrá-las na plataforma própria.
6. (Futuro) Integrar com o RetroAchievements.

### Fora de escopo
- Distribuir ROMs (nunca). O usuário fornece a própria ROM, como as recomps já exigem.
- Copiar/republicar sets de conquistas do RA sem autorização.
- Modificar as recomps nesta primeira etapa.

---

## 2. Decisão arquitetural principal

**O modelo correto a longo prazo é o próprio jogo detectar as conquistas** (rcheevos rodando dentro da recomp, a cada frame). A leitura externa de memória pelo launcher é apenas um **protótipo** para validar a ideia sem depender dos mantenedores das recomps.

Por isso, a **fonte de memória deve ser uma interface trocável**:

| Fase | Fonte da memória | Precisão | Compatível com RA? |
|------|------------------|----------|--------------------|
| 1 | Launcher lê a memória do processo de fora (amostragem por tempo) | Baixa/média | Improvável |
| 2 | Mod/patch na recomp expõe a RAM + sinal de "fim de frame" ao launcher (memória compartilhada/socket) | Alta | Talvez |
| 3 | rcheevos roda dentro da recomp (patch no runtime do N64Recomp) | Total | Modelo que o RA usa |

O **motor de conquistas (rcheevos)** e o **servidor** devem ser os mesmos nas três fases.

---

## 3. Componentes

### 3.1 Catálogo de jogos (manifesto)
**Implementado em duas camadas** (ver [docs/decisions/0002-catalogo-e-interface.md](docs/decisions/0002-catalogo-e-interface.md)):
`catalog/sources.json` (editado à mão) → `tools/catalog-builder` (GitHub + SteamGridDB + IGDB, via Action diário) → `catalog/games.json` (gerado, baixado pelo launcher). Tipos em `crates/catalog`.

Formato original proposto (o atual acrescenta `originalTitle`, `originalPlatform`, `sgdbId`, `igdbId`, `images`, `recomp`, `game`):

```json
{
  "id": "zelda64-mm",
  "name": "Zelda 64: Recompiled (Majora's Mask)",
  "repo": "Zelda64Recomp/Zelda64Recomp",
  "platforms": {
    "windows-x64": { "assetPattern": "*Windows*.zip", "executable": "Zelda64Recompiled.exe" },
    "linux-x64":   { "assetPattern": "*Linux*.AppImage", "executable": "*.AppImage" }
  },
  "requiredRom": {
    "description": "Majora's Mask (USA)",
    "sha1": ["<hash>"]
  },
  "achievements": {
    "enabled": false,
    "supportedVersions": []
  }
}
```

### 3.2 Integração com GitHub
- API REST: `GET /repos/{owner}/{repo}/releases` e `/releases/latest`.
- Limite sem autenticação: ~60 req/hora por IP → **cache local obrigatório** (ou servidor próprio servindo o catálogo).
- Selecionar o asset pelo `assetPattern` da plataforma atual.
- Mostrar tag, data e notas da release.

### 3.3 Estado local (versões instaladas)
O launcher grava seu próprio registro — não confiar em metadados do executável.

```json
{
  "zelda64-mm": {
    "installedTag": "v1.2.0",
    "installedAt": "2026-10-15T14:00:00-03:00",
    "installPath": "C:/Games/Recomp/zelda64-mm",
    "executableHash": "<sha256>",
    "playTimeSeconds": 0
  }
}
```

- O hash do executável detecta alteração manual e serve de chave para os offsets de memória (fase 1).
- Comparar `installedTag` com a release mais recente → "atualização disponível".

### 3.4 Execução do jogo
- Iniciar o executável como **processo filho**; monitorar fechamento (tempo de jogo).
- Primeira execução: a recomp pede a ROM — responsabilidade do app da recomp, não do launcher.

### 3.5 Monitor de memória (Fase 1 — protótipo)
- Windows: `OpenProcess` + `ReadProcessMemory` (handle já disponível por ser processo filho).
- Linux: `process_vm_readv`.
- Desafios conhecidos:
  - **Localizar a RAM do N64** no processo (endereço muda a cada execução por ASLR) → varredura por assinatura ou ponteiro conhecido **por versão** de cada recomp.
  - **Ordem dos bytes**: o N64Recomp provavelmente guarda a RDRAM em layout diferente do console (a confirmar no código-fonte).
  - **Sem sincronia de frame**: amostragem perde eventos de 1 frame e quebra condições com contadores/"valor anterior".
  - Antivírus podem sinalizar leitura de memória de outros processos (assinar o executável ajuda).
  - Recomp Windows rodando via Proton no Linux complica a leitura → priorizar builds nativas.

### 3.6 Motor de conquistas
- **rcheevos** (licença MIT): https://github.com/RetroAchievements/rcheevos
- Lê memória via callback fornecido pelo integrador → funciona com qualquer fonte (fases 1–3).
- Não faz HTTP nem parse de JSON: o cliente busca os dados no servidor e repassa.
- Escrever conquistas no **formato de condições de memória do RA** (não hooks de função), para manter a portabilidade.
- Verificar se o `rc_client` permite configurar o host do servidor (servidor próprio agora, RA depois).

### 3.7 Servidor (etapa posterior)
- Contas, jogos (hash da ROM + versão da recomp), definições de conquistas, desbloqueios, rankings.
- API inspirada na Connect API do RA (login, iniciar sessão, desbloquear, ping) → migração = trocar endpoint.
- Antitrapaça desde o início: modo hardcore, registro de tempo de sessão, validação de desbloqueios impossíveis no servidor.

---

## 4. Stack — decidida (15/10/2026)

Detalhes e justificativas: [docs/decisions/0001-stack.md](docs/decisions/0001-stack.md).

| Parte | Stack |
|-------|-------|
| Launcher | **Tauri 2** — backend em Rust, UI em React 19 + TypeScript + Vite |
| Catálogo | JSON em `catalog/`, embutido no binário + baixado do `raw.githubusercontent.com` |
| Conquistas | rcheevos (submódulo git) compilada pelo `build.rs` do Rust |
| Servidor (futuro) | Rust + axum + PostgreSQL |
| Distribuição | Instalador NSIS (Windows, por usuário, sem admin), AppImage/.deb (Linux), updater via GitHub Releases |

**Requisito de distribuição:** o usuário instala com um único `.exe`, sem instalar .NET, VC++ Redistributable ou qualquer outro programa antes. CRT linkado estaticamente; WebView2 é nativo no Windows 10/11 e, se faltar, o instalador o instala.

---

## 5. Estrutura de pastas

```
RecompHub/                   ← workspace Cargo
├─ CLAUDE.md                 ← este arquivo
├─ catalog/
│  ├─ sources.json           ← editado à mão
│  └─ games.json             ← GERADO pelo catalog-builder (não editar)
├─ crates/catalog/           ← tipos do catálogo (compartilhados)
├─ tools/catalog-builder/    ← gera games.json (GitHub + SteamGridDB + IGDB)
├─ app/                      ← launcher Tauri
│  ├─ src/                   ← UI (React + TS): pages/, components/, store.tsx
│  └─ src-tauri/src/         ← backend Rust:
│     ├─ catalog.rs          ← catálogo remoto + cache + fallback embutido
│     ├─ github.rs           ← API de releases (cache + ETag)
│     ├─ install.rs          ← download, extração (staging), hash, remoção
│     ├─ launch.rs           ← execução do processo filho
│     ├─ state.rs            ← biblioteca local (library.json)
│     └─ achievements/
│        ├─ memory_source.rs ← trait trocável (externa / mod / interna)
│        └─ engine.rs        ← integração com rcheevos                [Etapa 2]
├─ .github/workflows/        ← catalog.yml (diário), release.yml (tag v*), ci.yml
├─ server/                   ← (futuro) API de contas e conquistas
└─ docs/decisions/           ← registros de decisão (ADR)
```

Dados do usuário ficam em `%LOCALAPPDATA%\com.recomphub.launcher\` (`library.json`, `games/<id>/`).
Executável conferido em uma instalação real: Starfox 64 (`Starfox64Recompiled.exe`). Zelda 64 ainda não.

---

## 6. Roadmap

**Etapa 0 — Preparação**
- [x] Escolher a stack
- [x] Criar repositório (git local) e estrutura
- [x] Escrever manifesto de 2 recomps (Zelda 64 MM, Starfox 64)
- [ ] Publicar o repositório no GitHub

**Etapa 1 — Launcher básico (sem conquistas)**
- [x] Ler catálogo e listar jogos (remoto com cache + fallback embutido)
- [x] Buscar releases no GitHub (com cache)
- [x] Baixar e extrair o asset da plataforma
- [x] Registrar versão instalada / detectar atualização
- [x] Iniciar o jogo e contar tempo de jogo
- [x] Interface estilo Steam: Início, Explorar (filtros/ordenação), Biblioteca, página do jogo
- [x] catalog-builder + Action diário (GitHub, SteamGridDB, IGDB)
- [ ] Configurar secrets `SGDB_API_KEY`, `IGDB_CLIENT_ID`, `IGDB_CLIENT_SECRET` e rodar o builder com eles
- [x] Workflow de release (instaladores Windows/Linux via tauri-action) + updater assinado + CI
- [x] Ícone do app (bigorna, fonte em `app/app-icon.svg`)
- [ ] Configurar o secret `TAURI_SIGNING_PRIVATE_KEY` e publicar a v0.1.0
- [ ] Assinatura de código do instalador (SmartScreen)
- [ ] Escolher a pasta de instalação dos jogos (hoje fixa em `%LOCALAPPDATA%`)

**Etapa 2 — Prova de conceito de conquistas (Fase 1)**
- [ ] Localizar a RDRAM em uma versão específica de uma recomp (ex.: Zelda64Recomp)
- [ ] Ler memória externamente e validar valores conhecidos (vida, rupias etc.)
- [ ] Integrar rcheevos com 2–3 conquistas próprias, sem servidor

**Etapa 3 — Plataforma**
- [ ] Servidor mínimo (login + desbloqueio)
- [ ] Launcher exibindo progresso
- [ ] Modo hardcore

**Etapa 4 — Integração real**
- [ ] Fase 2/3: patch no runtime do N64Recomp ou mod por jogo
- [ ] Contato formal com a equipe do RetroAchievements

---

## 7. Ambiente de desenvolvimento (VSCode)

Comum a qualquer stack:
- Git + conta no GitHub (token pessoal opcional, para fugir do limite de requisições)
- VSCode com extensões: GitLens, EditorConfig, extensão da linguagem escolhida
- Compilador C/C++ (para a rcheevos): MSVC Build Tools no Windows, `gcc`/`clang` no Linux
- Uma recomp instalada manualmente + ROM própria, para testes
- Ferramenta de inspeção de memória para pesquisa (ex.: Cheat Engine no Windows, `scanmem`/GameConqueror no Linux)

Se Tauri: Rust (`rustup`), Node.js LTS, extensões `rust-analyzer` e Tauri.
Se Electron: Node.js LTS.
Se .NET: SDK .NET 8+, extensão C# Dev Kit.

---

## 8. Regras e cuidados

- Nunca distribuir ROMs nem links para ROMs.
- Não copiar sets de conquistas do RA sem permissão — criar conquistas próprias.
- Avisar os usuários que a migração para o RA não é garantida.
- Respeitar as licenças de cada recomp ao redistribuir/baixar (preferir baixar direto do GitHub do projeto).
- Conversar com a equipe do RA cedo, antes de investir na integração.

---

## 9. Referências

- rcheevos: https://github.com/RetroAchievements/rcheevos
- RA — Standalone Games: https://docs.retroachievements.org/general/standalone-support.html
- RA — Hardcore Compliance Requirements: https://docs.retroachievements.org/general/hardcore-compliance-requirements.html
- N64Recomp: https://github.com/N64Recomp/N64Recomp
- GitHub REST API — Releases: https://docs.github.com/en/rest/releases/releases
