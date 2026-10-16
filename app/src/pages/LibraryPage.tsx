import { useState } from "react";
import { useStore, useGameStatus } from "../store";
import { GameArt } from "../components/GameArt";
import { GameActions } from "../components/GameActions";
import { InfoPanel, ReleaseNotes } from "../components/GameDetails";
import { formatDate, formatPlayTime, formatRelative } from "../format";
import type { Game } from "../types";

export function LibraryPage({ gameId }: { gameId?: string }) {
  const { catalog, library, navigate } = useStore();
  const [query, setQuery] = useState("");

  const games = (catalog?.games ?? [])
    .filter((g) => library[g.id])
    .filter((g) => g.name.toLocaleLowerCase("pt-BR").includes(query.trim().toLocaleLowerCase("pt-BR")))
    .sort((a, b) => a.name.localeCompare(b.name, "pt-BR"));
  const installed = games.filter((g) => library[g.id].installed);
  const notInstalled = games.filter((g) => !library[g.id].installed);
  const selected = games.find((g) => g.id === gameId) ?? installed[0] ?? notInstalled[0];

  return (
    <div className="split">
      <aside className="sidebar library-list">
        <input
          className="search"
          type="search"
          placeholder="Buscar na biblioteca…"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <LibraryGroup title="Instalados" games={installed} selectedId={selected?.id} />
        <LibraryGroup title="Não instalados" games={notInstalled} selectedId={selected?.id} />
      </aside>

      <section className="library-detail">
        {selected ? (
          <LibraryDetail game={selected} />
        ) : (
          <div className="empty">
            <h2>Sua biblioteca está vazia</h2>
            <p className="muted">Adicione jogos pela aba Explorar.</p>
            <button className="btn btn-big btn-install" onClick={() => navigate({ page: "explore" })}>
              Explorar recomps
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
        <span className="library-name">{game.name}</span>
        {s.running && <span className="dot dot-running" title="Em execução" />}
        {s.progress && <span className="dot dot-busy" title="Instalando" />}
        {!s.running && !s.progress && s.updateAvailable && <span className="dot dot-update" title="Atualização disponível" />}
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
          <Stat label="Tempo de jogo" value={formatPlayTime(entry.playTimeSeconds)} />
          <Stat label="Última vez" value={entry.lastPlayedAt ? formatRelative(entry.lastPlayedAt) : "—"} />
          <Stat
            label="Versão instalada"
            value={s.installed ? s.installed.installedTag : "—"}
            hint={s.updateAvailable ? `${game.recomp?.latestRelease?.tag} disponível` : undefined}
          />
        </div>
      </div>

      <div className="detail-grid">
        <div className="detail-main">
          <div className="library-tools">
            {s.installed && (
              <>
                <button className="btn btn-ghost" onClick={() => store.openInstallDir(game.id)}>
                  Abrir pasta
                </button>
                <button
                  className="btn btn-ghost"
                  disabled={s.running || !!s.progress}
                  onClick={() =>
                    confirm(
                      s.installed?.imported
                        ? `Remover ${game.name} do launcher? Os arquivos em ${s.installed.installPath} não serão apagados.`
                        : `Desinstalar ${game.name}? A pasta de instalação será apagada.`,
                    ) && store.uninstall(game.id)
                  }
                >
                  {s.installed.imported ? "Remover do launcher" : "Desinstalar"}
                </button>
              </>
            )}
            {!s.installed && (
              <button className="btn btn-ghost" onClick={() => store.removeFromLibrary(game.id)}>
                Remover da biblioteca
              </button>
            )}
            <button className="btn btn-ghost" onClick={() => store.navigate({ page: "game", gameId: game.id })}>
              Página do jogo
            </button>
          </div>
          {s.installed && (
            <p className="muted small">
              {s.installed.imported ? "Importado" : "Instalado"} em {formatDate(s.installed.installedAt)} · {s.installed.installPath}
            </p>
          )}
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
