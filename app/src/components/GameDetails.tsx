import { useEffect, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api } from "../api";
import { formatCount, formatDate } from "../format";
import type { Game, Release } from "../types";

/** Ficha lateral com dados do recomp (GitHub) e do jogo original (IGDB). */
export function InfoPanel({ game }: { game: Game }) {
  const r = game.recomp;
  const g = game.game;
  const repoUrl = `https://github.com/${game.repo}`;
  return (
    <aside className="info-panel">
      <h3>Recomp</h3>
      <dl>
        <dt>Versão</dt>
        <dd>{r?.latestRelease?.tag ?? "—"}</dd>
        <dt>Lançamento</dt>
        <dd>{formatDate(r?.firstReleaseAt)}</dd>
        <dt>Última atualização</dt>
        <dd>{formatDate(r?.latestRelease?.publishedAt)}</dd>
        <dt>Estrelas</dt>
        <dd>★ {formatCount(r?.stars)}</dd>
        <dt>Downloads</dt>
        <dd>{formatCount(r?.totalDownloads)}</dd>
        <dt>Licença</dt>
        <dd>{r?.license ?? "—"}</dd>
      </dl>
      <button className="link" onClick={() => openUrl(repoUrl)}>
        {game.repo} ↗
      </button>

      <h3>Jogo original</h3>
      <dl>
        <dt>Plataforma</dt>
        <dd>{game.originalPlatform}</dd>
        <dt>Lançamento</dt>
        <dd>{formatDate(g?.releaseDate)}</dd>
        <dt>Desenvolvedora</dt>
        <dd>{g?.developers.join(", ") || "—"}</dd>
        <dt>Publicadora</dt>
        <dd>{g?.publishers.join(", ") || "—"}</dd>
        <dt>Gêneros</dt>
        <dd>{g?.genres.join(", ") || "—"}</dd>
        {g?.rating != null && (
          <>
            <dt>Nota (IGDB)</dt>
            <dd>{Math.round(g.rating)}/100</dd>
          </>
        )}
      </dl>

      <div className="rom-notice">
        <strong>Requer sua própria ROM</strong>
        <span>{game.requiredRom.description}</span>
      </div>
    </aside>
  );
}

export function Screenshots({ game }: { game: Game }) {
  const shots = game.game?.screenshots ?? [];
  if (shots.length === 0) return null;
  return (
    <section>
      <h2 className="section-title">Imagens</h2>
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
      <h2 className="section-title">Notas da versão</h2>
      {error && <p className="muted">Não foi possível carregar: {error}</p>}
      {!release && !error && <p className="muted">Carregando…</p>}
      {release && (
        <div className="release-notes">
          <div className="release-notes-head">
            <strong>{release.name || release.tagName}</strong>
            <span className="muted">{formatDate(release.publishedAt)}</span>
          </div>
          <p>{release.body?.trim() || "Sem notas."}</p>
          <button className="link" onClick={() => openUrl(release.htmlUrl)}>
            Ver no GitHub ↗
          </button>
        </div>
      )}
    </section>
  );
}
