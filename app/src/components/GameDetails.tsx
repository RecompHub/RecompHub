import { useEffect, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api } from "../api";
import { formatCount, formatDate } from "../format";
import type { Game, Release } from "../types";
import { lang, t } from "../i18n";

/** Ficha lateral com dados do recomp (GitHub) e do jogo original (IGDB). */
export function InfoPanel({ game }: { game: Game }) {
  const r = game.recomp;
  const g = game.game;
  const repoUrl = `https://github.com/${game.repo}`;
  return (
    <aside className="info-panel">
      <h3>{t("Recomp")}</h3>
      <dl>
        <dt>{t("Versão")}</dt>
        <dd>{r?.latestRelease?.tag ?? "—"}</dd>
        <dt>{t("Lançamento")}</dt>
        <dd>{formatDate(r?.firstReleaseAt)}</dd>
        <dt>{t("Última atualização")}</dt>
        <dd>{formatDate(r?.latestRelease?.publishedAt)}</dd>
        <dt>{t("Estrelas")}</dt>
        <dd>★ {formatCount(r?.stars)}</dd>
        <dt>{t("Downloads")}</dt>
        <dd>{formatCount(r?.totalDownloads)}</dd>
        <dt>{t("Licença")}</dt>
        <dd>{r?.license ?? "—"}</dd>
      </dl>
      <button className="link" onClick={() => openUrl(repoUrl)}>
        {game.repo} ↗
      </button>

      <h3>{t("Jogo original")}</h3>
      <dl>
        <dt>{t("Plataforma")}</dt>
        <dd>{game.originalPlatform}</dd>
        <dt>{t("Lançamento")}</dt>
        <dd>{formatDate(g?.releaseDate)}</dd>
        <dt>{t("Desenvolvedora")}</dt>
        <dd>{g?.developers.join(", ") || "—"}</dd>
        <dt>{t("Publicadora")}</dt>
        <dd>{g?.publishers.join(", ") || "—"}</dd>
        <dt>{t("Gêneros")}</dt>
        <dd>{g?.genres.join(", ") || "—"}</dd>
        {g?.rating != null && (
          <>
            <dt>{t("Nota (IGDB)")}</dt>
            <dd>{Math.round(g.rating)}/100</dd>
          </>
        )}
      </dl>

      <div className="rom-notice">
        <strong>{t("Requer sua própria ROM")}</strong>
        <span>{lang() === "en" ? (game.requiredRom.descriptionEn ?? game.requiredRom.description) : game.requiredRom.description}</span>
      </div>
    </aside>
  );
}

/** Projeto recomp/port por trás do jogo (o título do jogo fica no topo da página). */
export function ProjectSection({ game }: { game: Game }) {
  const owner = game.repo.split("/")[0];
  const repoUrl = `https://github.com/${game.repo}`;
  return (
    <section>
      <h2 className="section-title">{t("Projeto")}</h2>
      <div className="project-card">
        <div className="project-head">
          <strong className="project-name">{game.name}</strong>
          <span className="muted small">{t("por {owner}", { owner })}</span>
        </div>
        {game.recomp?.description && <p className="muted small project-desc">{game.recomp.description}</p>}
        {Object.values(game.platforms).some((p) => p.installer) && (
          <div className="rom-notice">
            <strong>{t("Instalador do projeto")}</strong>
            <span>
              {t(
                "Este jogo é instalado pelo instalador do próprio projeto, que compila o jogo no seu PC a partir do seu disco. Pode demorar bastante e ocupar vários GB durante a instalação.",
              )}
            </span>
          </div>
        )}
        {game.aiAssisted && (
          <div className="ai-notice">
            <strong>{t("Feito com IA")}</strong>
            <span>{t("Este projeto foi marcado como gerado em grande parte por IA. Pode ter mais bugs e menos manutenção que os outros.")}</span>
          </div>
        )}
        <button className="link" onClick={() => openUrl(repoUrl)}>
          {game.repo} ↗
        </button>
      </div>
    </section>
  );
}

export function Screenshots({ game }: { game: Game }) {
  const shots = game.game?.screenshots ?? [];
  if (shots.length === 0) return null;
  return (
    <section>
      <h2 className="section-title">{t("Imagens")}</h2>
      <div className="screenshots">
        {shots.map((url) => (
          <img key={url} src={url} alt="" loading="lazy" />
        ))}
      </div>
    </section>
  );
}

/** Notas da última release, buscadas sob demanda (cache de 10 min no backend). */
export function ReleaseNotes({ game }: { game: Game }) {
  const [release, setRelease] = useState<Release | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setRelease(null);
    setError(null);
    api.latestRelease(game.id).then(setRelease).catch((e) => setError(String(e)));
  }, [game.id]);

  return (
    <section>
      <h2 className="section-title">{t("Notas da versão")}</h2>
      {error && <p className="muted">{t("Não foi possível carregar: {error}", { error })}</p>}
      {!release && !error && <p className="muted">{t("Carregando…")}</p>}
      {release && (
        <div className="release-notes">
          <div className="release-notes-head">
            <strong>{release.name || release.tagName}</strong>
            <span className="muted">{formatDate(release.publishedAt)}</span>
          </div>
          <p>{release.body?.trim() || t("Sem notas.")}</p>
          <button className="link" onClick={() => openUrl(release.htmlUrl)}>
            {t("Ver no GitHub")} ↗
          </button>
        </div>
      )}
    </section>
  );
}
