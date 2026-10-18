import { useState } from "react";
import { confirm } from "@tauri-apps/plugin-dialog";
import { useStore, useGameStatus } from "../store";
import { GameArt } from "../components/GameArt";
import { GameActions } from "../components/GameActions";
import { InfoPanel, ReleaseNotes } from "../components/GameDetails";
import { TrophiesSection } from "../components/Trophies";
import { formatDate, formatPlayTime, formatRelative, gameTitle } from "../format";
import type { Game } from "../types";
import { locale, t } from "../i18n";

export function LibraryPage({ gameId }: { gameId?: string }) {
  const { catalog, library, navigate } = useStore();
  const [query, setQuery] = useState("");

  const games = (catalog?.games ?? [])
    .filter((g) => library[g.id])
    .filter((g) =>
      [gameTitle(g), g.name].some((n) => n.toLocaleLowerCase(locale()).includes(query.trim().toLocaleLowerCase(locale()))),
    )
    .sort((a, b) => gameTitle(a).localeCompare(gameTitle(b), locale()));
  const installed = games.filter((g) => library[g.id].installed);
  const notInstalled = games.filter((g) => !library[g.id].installed);
  const selected = games.find((g) => g.id === gameId) ?? installed[0] ?? notInstalled[0];

  return (
    <div className="split">
      <aside className="sidebar library-list">
        <input
          className="search"
          type="search"
          placeholder={t("Buscar na biblioteca…")}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <LibraryGroup title={t("Instalados")} games={installed} selectedId={selected?.id} />
        <LibraryGroup title={t("Não instalados")} games={notInstalled} selectedId={selected?.id} />
      </aside>

      <section className="library-detail">
        {selected ? (
          <LibraryDetail game={selected} />
        ) : (
          <div className="empty">
            <h2>{t("Sua biblioteca está vazia")}</h2>
            <p className="muted">{t("Adicione jogos pela aba Explorar.")}</p>
            <button className="btn btn-big btn-install" onClick={() => navigate({ page: "explore" })}>
              {t("Explorar recomps")}
            </button>
          </div>
        )}
      </section>
    </div>
  );
}

function LibraryGroup({ title, games, selectedId }: { title: string; games: Game[]; selectedId?: string }) {
  if (games.length === 0) return null;
  return (
    <>
      <h4>
        {title} ({games.length})
      </h4>
      <ul>
        {games.map((g) => (
          <LibraryItem key={g.id} game={g} selected={g.id === selectedId} />
        ))}
      </ul>
    </>
  );
}

function LibraryItem({ game, selected }: { game: Game; selected: boolean }) {
  const { navigate } = useStore();
  const s = useGameStatus(game);
  return (
    <li>
      <button
        className={`library-item ${selected ? "library-item-active" : ""} ${s.installed ? "" : "library-item-dim"}`}
        onClick={() => navigate({ page: "library", gameId: game.id })}
      >
        <GameArt game={game} kind="icon" className="library-icon" />
        <span className="library-name">{gameTitle(game)}</span>
        {s.running && <span className="dot dot-running" title={t("Em execução")} />}
        {s.progress && <span className="dot dot-busy" title={t("Instalando")} />}
        {!s.running && !s.progress && s.updateAvailable && <span className="dot dot-update" title={t("Atualização disponível")} />}
      </button>
    </li>
  );
}

function LibraryDetail({ game }: { game: Game }) {
  const store = useStore();
  const s = useGameStatus(game);
  const entry = s.entry!;

  return (
    <div className="page library-page">
      <div className="hero">
        <GameArt game={game} kind="hero" className="hero-img" />
        <div className="hero-overlay">
          <GameArt game={game} kind="logo" className="hero-logo" />
        </div>
      </div>

      <div className="action-bar">
        <GameActions game={game} />
        <div className="stats">
          <Stat label={t("Tempo de jogo")} value={formatPlayTime(entry.playTimeSeconds)} />
          <Stat label={t("Última vez")} value={entry.lastPlayedAt ? formatRelative(entry.lastPlayedAt) : "—"} />
          <Stat
            label={t("Versão instalada")}
            value={s.installed ? s.installed.installedTag : "—"}
            hint={s.updateAvailable ? t("{tag} disponível", { tag: game.recomp?.latestRelease?.tag ?? "" }) : undefined}
          />
        </div>
      </div>

      <div className="detail-grid">
        <div className="detail-main">
          <div className="library-tools">
            {s.installed && (
              <>
                <button className="btn btn-ghost" onClick={() => store.openInstallDir(game.id)}>
                  {t("Abrir pasta")}
                </button>
                <button
                  className="btn btn-ghost"
                  disabled={s.running || !!s.progress}
                  onClick={async () => {
                    // O confirm do navegador vira assíncrono no Tauri (plugin de diálogo):
                    // é preciso esperar a resposta antes de desinstalar.
                    const forget = s.installed?.imported || s.installed?.external;
                    const ok = await confirm(
                      s.installed?.external
                        ? t(
                            "Remover {game} do launcher? O jogo continua instalado; para apagá-lo, use o desinstalador do projeto em Configurações do Windows → Aplicativos.",
                            { game: gameTitle(game) },
                          )
                        : s.installed?.imported
                          ? t("Remover {game} do launcher? Os arquivos em {path} não serão apagados.", {
                              game: gameTitle(game),
                              path: s.installed.installPath,
                            })
                          : t(
                              "Desinstalar {game}? Os arquivos instalados pelo launcher serão apagados; ROM, saves e outros arquivos seus na pasta continuam lá.",
                              { game: gameTitle(game) },
                            ),
                      {
                        title: gameTitle(game),
                        kind: "warning",
                        okLabel: t(forget ? "Remover do launcher" : "Desinstalar"),
                        cancelLabel: t("Cancelar"),
                      },
                    );
                    if (ok) await store.uninstall(game.id);
                  }}
                >
                  {t(s.installed.imported || s.installed.external ? "Remover do launcher" : "Desinstalar")}
                </button>
              </>
            )}
            {!s.installed && (
              <button className="btn btn-ghost" onClick={() => store.removeFromLibrary(game.id)}>
                {t("Remover da biblioteca")}
              </button>
            )}
            <button className="btn btn-ghost" onClick={() => store.navigate({ page: "game", gameId: game.id })}>
              {t("Página do jogo")}
            </button>
          </div>
          {s.installed && (
            <p className="muted small">
              {t(s.installed.imported ? "Importado em {date}" : "Instalado em {date}", { date: formatDate(s.installed.installedAt) })} · {s.installed.installPath}
            </p>
          )}
          <TrophiesSection game={game} />
          <ReleaseNotes game={game} />
        </div>
        <InfoPanel game={game} />
      </div>
    </div>
  );
}

function Stat({ label, value, hint }: { label: string; value: string; hint?: string }) {
  return (
    <div className="stat">
      <span className="stat-label">{label}</span>
      <span className="stat-value">{value}</span>
      {hint && <span className="stat-hint">{hint}</span>}
    </div>
  );
}
