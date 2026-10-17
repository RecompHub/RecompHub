import { useEffect, useState } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { useStore } from "../store";
import { formatBytes } from "../format";
import { t } from "../i18n";

/**
 * Atualização do próprio launcher (GitHub Releases, assinada).
 * No Windows o instalador roda em modo "passive" e reabre o app sozinho.
 */
export function UpdateBanner() {
  const { running, progress } = useStore();
  const [update, setUpdate] = useState<Update | null>(null);
  const [downloaded, setDownloaded] = useState<{ done: number; total?: number } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [dismissed, setDismissed] = useState(false);

  useEffect(() => {
    // Sem rede ou sem release publicada, simplesmente não mostra nada.
    check().then(setUpdate).catch(() => {});
  }, []);

  if (!update || dismissed) return null;

  // Atualizar fecha o launcher: o jogo seguiria aberto, mas o tempo de jogo
  // e uma instalação em andamento se perderiam.
  const blocked = running.size > 0 || Object.keys(progress).length > 0;

  const install = async () => {
    setError(null);
    setDownloaded({ done: 0 });
    try {
      let done = 0;
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") setDownloaded({ done: 0, total: event.data.contentLength });
        if (event.event === "Progress") {
          done += event.data.chunkLength;
          setDownloaded((d) => ({ done, total: d?.total }));
        }
      });
      await relaunch();
    } catch (e) {
      setDownloaded(null);
      setError(String(e));
    }
  };

  return (
    <div className="update-banner" role="status">
      <span>
        <strong>RecompHub {update.version}</strong> {t("disponível")}
        {update.currentVersion && <span className="muted"> {t("(você tem {version})", { version: update.currentVersion })}</span>}
      </span>
      {error && <span className="update-error">{t("Falhou: {error}", { error })}</span>}
      {downloaded ? (
        <span className="muted">
          {downloaded.total
            ? t("Baixando {done} de {total}…", { done: formatBytes(downloaded.done), total: formatBytes(downloaded.total) })
            : t("Baixando {done}…", { done: formatBytes(downloaded.done) })}
        </span>
      ) : (
        <>
          <button
            className="btn btn-update"
            onClick={install}
            disabled={blocked}
            title={blocked ? t("Feche o jogo e aguarde as instalações terminarem") : undefined}
          >
            {t("Atualizar e reiniciar")}
          </button>
          <button className="btn btn-ghost" onClick={() => setDismissed(true)}>
            {t("Depois")}
          </button>
        </>
      )}
    </div>
  );
}
