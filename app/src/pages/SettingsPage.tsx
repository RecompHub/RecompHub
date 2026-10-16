import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { open } from "@tauri-apps/plugin-dialog";
import { check } from "@tauri-apps/plugin-updater";
import { api } from "../api";
import type { SettingsView } from "../types";

export function SettingsPage() {
  const [settings, setSettings] = useState<SettingsView | null>(null);
  const [version, setVersion] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [updateStatus, setUpdateStatus] = useState<string | null>(null);

  useEffect(() => {
    api.getSettings().then(setSettings).catch((e) => setError(String(e)));
    getVersion().then(setVersion);
  }, []);

  const changeGamesDir = async (path: string | null) => {
    setError(null);
    try {
      setSettings(await api.setGamesDir(path));
    } catch (e) {
      setError(String(e));
    }
  };

  const pickFolder = async () => {
    const chosen = await open({ directory: true, defaultPath: settings?.gamesDir, title: "Pasta dos jogos" });
    if (typeof chosen === "string") await changeGamesDir(chosen);
  };

  const checkUpdates = async () => {
    setUpdateStatus("Procurando…");
    try {
      const update = await check();
      // Se houver, a faixa no topo do app oferece a instalação.
      setUpdateStatus(update ? `Versão ${update.version} disponível — reabra o launcher para instalar.` : "Você está na versão mais recente.");
    } catch (e) {
      setUpdateStatus(`Não foi possível verificar: ${e}`);
    }
  };

  return (
    <div className="page settings">
      <h1 className="settings-title">Configurações</h1>

      <section className="settings-section">
        <h2 className="section-title">Jogos</h2>
        <div className="setting">
          <div className="setting-text">
            <strong>Pasta de instalação</strong>
            <span className="muted small">
              Onde novos jogos são instalados. Jogos já instalados continuam na pasta atual.
            </span>
            <code className="path">{settings?.gamesDir ?? "…"}</code>
          </div>
          <div className="setting-actions">
            <button className="btn btn-ghost" onClick={pickFolder}>
              Alterar…
            </button>
            {settings && !settings.isDefaultGamesDir && (
              <button className="btn btn-ghost" onClick={() => changeGamesDir(null)}>
                Usar padrão
              </button>
            )}
          </div>
        </div>
        {error && <p className="settings-error">{error}</p>}
      </section>

      <section className="settings-section">
        <h2 className="section-title">Sobre</h2>
        <div className="setting">
          <div className="setting-text">
            <strong>RecompHub {version}</strong>
            <span className="muted small">Atualizações do launcher vêm assinadas do GitHub Releases.</span>
            {updateStatus && <span className="small">{updateStatus}</span>}
          </div>
          <div className="setting-actions">
            <button className="btn btn-ghost" onClick={checkUpdates}>
              Procurar atualizações
            </button>
          </div>
        </div>
      </section>
    </div>
  );
}
