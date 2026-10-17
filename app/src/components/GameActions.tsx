import { open } from "@tauri-apps/plugin-dialog";
import { useStore, useGameStatus } from "../store";
import { formatBytes, gameTitle } from "../format";
import type { Game } from "../types";
import { t } from "../i18n";

/** Botão principal (Jogar / Instalar / Atualizar / progresso) de um jogo. */
export function GameActions({ game, showLibraryButton = false }: { game: Game; showLibraryButton?: boolean }) {
  const store = useStore();
  const s = useGameStatus(game);

  if (s.progress) {
    const pct = s.progress.total ? Math.round((s.progress.downloaded / s.progress.total) * 100) : 0;
    const label =
      s.progress.stage === "extract"
        ? t("Extraindo…")
        : t("Baixando {pct}% · {done} de {total}", {
            pct,
            done: formatBytes(s.progress.downloaded),
            total: formatBytes(s.progress.total),
          });
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
          {t("Indisponível para {platform}", { platform: store.platform })}
        </button>
      ) : s.running ? (
        <button className="btn btn-big btn-running" disabled>
          ● {t("Em execução")}
        </button>
      ) : s.installed ? (
        <>
          <button className="btn btn-big btn-play" onClick={() => store.launch(game.id)}>
            ▶ {t("Jogar")}
          </button>
          {s.updateAvailable && (
            <button className="btn btn-big btn-update" onClick={() => store.install(game.id)}>
              {t("Atualizar para {tag}", { tag: game.recomp?.latestRelease?.tag ?? "" })}
            </button>
          )}
        </>
      ) : (
        <>
          <button className="btn btn-big btn-install" onClick={() => store.install(game.id)}>
            ⬇ {t("Instalar")}
          </button>
          <button
            className="btn btn-big btn-secondary"
            title={t("Já baixou esta recomp por conta própria? Aponte a pasta dela.")}
            onClick={async () => {
              const dir = await open({ directory: true, title: t("Pasta onde {game} está instalado", { game: gameTitle(game) }) });
              if (typeof dir === "string") await store.importInstall(game.id, dir);
            }}
          >
            {t("Já tenho instalado…")}
          </button>
        </>
      )}

      {showLibraryButton && !s.inLibrary && (
        <button className="btn btn-big btn-secondary" onClick={() => store.addToLibrary(game.id)}>
          + {t("Adicionar à biblioteca")}
        </button>
      )}
      {showLibraryButton && s.inLibrary && (
        <button className="btn btn-big btn-secondary" onClick={() => store.navigate({ page: "library", gameId: game.id })}>
          ✓ {t("Na biblioteca")}
        </button>
      )}
    </div>
  );
}
