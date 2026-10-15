import { useStore, useGameStatus } from "../store";
import { formatBytes } from "../format";
import type { Game } from "../types";

/** Botão principal (Jogar / Instalar / Atualizar / progresso) de um jogo. */
export function GameActions({ game, showLibraryButton = false }: { game: Game; showLibraryButton?: boolean }) {
  const store = useStore();
  const s = useGameStatus(game);

  if (s.progress) {
    const pct = s.progress.total ? Math.round((s.progress.downloaded / s.progress.total) * 100) : 0;
    const label =
      s.progress.stage === "extract"
        ? "Extraindo…"
        : `Baixando ${pct}% · ${formatBytes(s.progress.downloaded)} de ${formatBytes(s.progress.total)}`;
    return (
      <div className="actions">
        <div className="progress" role="progressbar" aria-valuenow={pct}>
          <div className="progress-bar" style={{ width: `${s.progress.stage === "extract" ? 100 : pct}%` }} />
          <span>{label}</span>
        </div>
      </div>
    );
  }

  return (
    <div className="actions">
      {!s.supported ? (
        <button className="btn btn-big" disabled>
          Indisponível para {store.platform}
        </button>
      ) : s.running ? (
        <button className="btn btn-big btn-running" disabled>
          ● Em execução
        </button>
      ) : s.installed ? (
        <>
          <button className="btn btn-big btn-play" onClick={() => store.launch(game.id)}>
            ▶ Jogar
          </button>
          {s.updateAvailable && (
            <button className="btn btn-big btn-update" onClick={() => store.install(game.id)}>
              Atualizar para {game.recomp?.latestRelease?.tag}
            </button>
          )}
        </>
      ) : (
        <button className="btn btn-big btn-install" onClick={() => store.install(game.id)}>
          ⬇ Instalar
        </button>
      )}

      {showLibraryButton && !s.inLibrary && (
        <button className="btn btn-big btn-secondary" onClick={() => store.addToLibrary(game.id)}>
          + Adicionar à biblioteca
        </button>
      )}
      {showLibraryButton && s.inLibrary && (
        <button className="btn btn-big btn-secondary" onClick={() => store.navigate({ page: "library", gameId: game.id })}>
          ✓ Na biblioteca
        </button>
      )}
    </div>
  );
}
