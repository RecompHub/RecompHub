import { open } from "@tauri-apps/plugin-dialog";
import { useStore, useGameStatus } from "../store";
import { formatBytes, gameTitle } from "../format";
import type { Game } from "../types";
import { t } from "../i18n";

/** Botão principal (Jogar / Instalar / Atualizar / progresso) de um jogo. */
export function GameActions({ game, showLibraryButton = false }: { game: Game; showLibraryButton?: boolean }) {
  const store = useStore();
  const s = useGameStatus(game);

  const installer = game.platforms[store.platform]?.installer;
  const needsRom = installer?.args?.some((a) => a.includes("{rom}")) ?? false;

  // Jogos de instalador próprio que compilam a partir do disco pedem a imagem antes.
  const install = async () => {
    let rom: string | undefined;
    if (needsRom) {
      const file = await open({
        title: t("Imagem do disco de {game}", { game: gameTitle(game) }),
        filters: [{ name: t("Imagem de disco"), extensions: ["iso", "wbfs", "rvz", "wia", "gcz", "ciso", "gcm"] }],
      });
      if (typeof file !== "string") return;
      rom = file;
    }
    await store.install(game.id, rom);
  };

  if (s.progress) {
    const p = s.progress;
    const pct = p.total ? Math.round((p.downloaded / p.total) * 100) : 0;
    const label =
      p.stage === "extract"
        ? t("Extraindo…")
        : p.stage === "setup"
          ? p.total
            ? [t("Instalando {pct}%", { pct }), p.message].filter(Boolean).join(" · ")
            : t("Siga as instruções na janela do instalador…")
          : t("Baixando {pct}% · {done} de {total}", {
              pct,
              done: formatBytes(p.downloaded),
              total: formatBytes(p.total),
            });
    const full = p.stage === "extract" || (p.stage === "setup" && !p.total);
    return (
      <div className="actions">
        <div className="progress" role="progressbar" aria-valuenow={pct}>
          <div className="progress-bar" style={{ width: `${full ? 100 : pct}%` }} />
          <span>{label}</span>
        </div>
        {p.stage === "setup" && p.total > 0 && (
          <span className="muted small">{t("Pode demorar bastante: o jogo está sendo compilado no seu PC.")}</span>
        )}
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
            <button className="btn btn-big btn-update" onClick={install}>
              {t("Atualizar para {tag}", { tag: game.recomp?.latestRelease?.tag ?? "" })}
            </button>
          )}
        </>
      ) : (
        <>
          <button className="btn btn-big btn-install" onClick={install}>
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
