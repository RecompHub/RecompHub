import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { open } from "@tauri-apps/plugin-dialog";
import { check } from "@tauri-apps/plugin-updater";
import { api } from "../api";
import { t, useLang, type LangChoice } from "../i18n";
import type { SettingsView } from "../types";

const LANGUAGES: { value: LangChoice; label: string }[] = [
  { value: null, label: "Automático (idioma do sistema)" },
  { value: "pt", label: "Português (Brasil)" },
  { value: "en", label: "English" },
];

export function SettingsPage() {
  const { choice, setChoice } = useLang();
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
      await api.scanGamesDir();
    } catch (e) {
      setError(String(e));
    }
  };

  const pickFolder = async () => {
    const chosen = await open({ directory: true, defaultPath: settings?.gamesDir, title: t("Pasta dos jogos") });
    if (typeof chosen === "string") await changeGamesDir(chosen);
  };

  const checkUpdates = async () => {
    setUpdateStatus(t("Procurando…"));
    try {
      const update = await check();
      // Se houver, a faixa no topo do app oferece a instalação.
      setUpdateStatus(
        update
          ? t("Versão {version} disponível — reabra o launcher para instalar.", { version: update.version })
          : t("Você está na versão mais recente."),
      );
    } catch (e) {
      setUpdateStatus(t("Não foi possível verificar: {error}", { error: String(e) }));
    }
  };

  return (
    <div className="page settings">
      <h1 className="settings-title">{t("Configurações")}</h1>

      <section className="settings-section">
        <h2 className="section-title">{t("Geral")}</h2>
        <div className="setting">
          <div className="setting-text">
            <strong>{t("Idioma")}</strong>
            <span className="muted small">{t("Idioma da interface do launcher.")}</span>
          </div>
          <div className="setting-actions">
            <select
              value={choice ?? "auto"}
              onChange={(e) => setChoice(e.target.value === "auto" ? null : (e.target.value as LangChoice))}
              aria-label={t("Idioma")}
            >
              {LANGUAGES.map((l) => (
                <option key={l.value ?? "auto"} value={l.value ?? "auto"}>
                  {l.value ? l.label : t(l.label)}
                </option>
              ))}
            </select>
          </div>
        </div>
      </section>

      <section className="settings-section">
        <h2 className="section-title">{t("Jogos")}</h2>
        <div className="setting">
          <div className="setting-text">
            <strong>{t("Pasta de instalação")}</strong>
            <span className="muted small">
              {t("Onde novos jogos são instalados. Jogos já instalados continuam na pasta atual.")}
            </span>
            <code className="path">{settings?.gamesDir ?? "…"}</code>
          </div>
          <div className="setting-actions">
            <button className="btn btn-ghost" onClick={pickFolder}>
              {t("Alterar…")}
            </button>
            {settings && !settings.isDefaultGamesDir && (
              <button className="btn btn-ghost" onClick={() => changeGamesDir(null)}>
                {t("Usar padrão")}
              </button>
            )}
          </div>
        </div>
        {error && <p className="settings-error">{error}</p>}
      </section>

      <section className="settings-section">
        <h2 className="section-title">{t("Sobre")}</h2>
        <div className="setting">
          <div className="setting-text">
            <strong>RecompHub {version}</strong>
            <span className="muted small">{t("Atualizações do launcher vêm assinadas do GitHub Releases.")}</span>
            {updateStatus && <span className="small">{updateStatus}</span>}
          </div>
          <div className="setting-actions">
            <button className="btn btn-ghost" onClick={checkUpdates}>
              {t("Procurar atualizações")}
            </button>
          </div>
        </div>
      </section>
    </div>
  );
}
