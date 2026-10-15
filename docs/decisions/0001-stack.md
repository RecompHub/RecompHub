# 0001 — Stack do projeto

- **Data:** 15/10/2026
- **Status:** aceita

## Requisito de distribuição

O usuário final instala o RecompHub com **um único instalador**, sem precisar instalar
nada antes (.NET, Visual C++ Redistributable, Node, Java etc.) e **sem precisar de admin**.

## Decisão por parte do projeto

| Parte | Stack | Por quê |
|-------|-------|---------|
| Launcher — backend | **Rust** (Tauri 2) | Binário nativo, sem runtime. Acesso direto a `ReadProcessMemory` / `process_vm_readv` (crate `windows` / `libc`). Compila a rcheevos (C) com o crate `cc`. |
| Launcher — UI | **React 19 + TypeScript + Vite** | Maior ecossistema de componentes; a UI roda no WebView2 (Windows) / WebKitGTK (Linux). |
| Catálogo | **JSON versionado** em `catalog/` | Embutido no binário (funciona offline) e baixado de `raw.githubusercontent.com`, que não sofre o limite de 60 req/h da API. |
| Integração GitHub | `reqwest` com **rustls** | Sem dependência de OpenSSL no Linux. Cache local das respostas da API. |
| Download/instalação dos jogos | crates `zip`, `sha2`, `globset` | Extração, hash do executável e seleção do asset por `assetPattern`. |
| Conquistas | **rcheevos** como submódulo git, compilada no `build.rs` | Mesmo motor nas fases 1–3. A fonte de memória é a trait `MemorySource`. |
| Servidor (futuro) | **Rust + axum + PostgreSQL (sqlx)** | Mesma linguagem do launcher: tipos compartilhados num crate `shared` dentro de um workspace Cargo. |
| Instalador Windows | **NSIS** (bundler do Tauri) | `.exe` único, instalação por usuário (`currentUser`), em PT-BR. |
| Pacotes Linux | **AppImage** (+ `.deb`) | AppImage roda no Steam Deck sem instalação. |
| Atualização do launcher | `tauri-plugin-updater` + GitHub Releases | Atualização assinada (chave do updater), sem servidor próprio. |
| CI/CD | GitHub Actions + `tauri-apps/tauri-action` | Gera os instaladores Windows/Linux a cada tag. |

## Como o "zero dependências externas" é garantido

| Dependência | Solução |
|-------------|---------|
| .NET | Não usado. |
| Visual C++ Redistributable | CRT linkado estaticamente (`.cargo/config.toml`, `+crt-static`). |
| WebView2 | Já vem no Windows 11 e no Windows 10 atualizado. Se faltar, o instalador embute o bootstrapper (`webviewInstallMode: embedBootstrapper`) e instala sozinho, sem admin. |
| Node.js | Só no desenvolvimento; a UI é empacotada como arquivos estáticos dentro do `.exe`. |
| Permissão de admin | `installMode: currentUser` instala em `%LOCALAPPDATA%`. |

## Alternativas descartadas

- **Electron:** instalador de ~100 MB e leitura de memória exigiria módulo nativo em C++.
- **.NET (WPF/Avalonia):** WPF só roda no Windows; o modo self-contained evita instalar o .NET,
  mas gera binários grandes e o AOT tem limitações com UI.

## Pendências

- [ ] Assinatura de código do instalador (SignPath para projetos open source, ou Azure Trusted Signing)
      — reduz alertas do SmartScreen e de antivírus na leitura de memória.
- [x] Gerar as chaves do updater (`~/.tauri/recomphub.key`, sem senha; a pública está no `tauri.conf.json`).
- [ ] Antes da primeira release pública, decidir se a chave terá senha (gerar de novo agora é
      grátis; depois que houver usuários, trocar a chave quebra a atualização automática deles).
