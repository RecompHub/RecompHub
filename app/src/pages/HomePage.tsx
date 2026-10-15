import { useStore } from "../store";
import { GameArt } from "../components/GameArt";
import { GameCard } from "../components/GameCard";
import { formatRelative } from "../format";
import type { Game } from "../types";

const time = (iso: string | null | undefined) => (iso ? new Date(iso).getTime() : 0);
const updatedAt = (g: Game) => time(g.recomp?.latestRelease?.publishedAt);
const launchedAt = (g: Game) => time(g.recomp?.firstReleaseAt);

export function HomePage() {
  const { catalog, navigate } = useStore();
  if (!catalog) return <p className="page muted">Carregando…</p>;

  const byUpdate = [...catalog.games].sort((a, b) => updatedAt(b) - updatedAt(a));
  const byLaunch = [...catalog.games].sort((a, b) => launchedAt(b) - launchedAt(a));
  const byStars = [...catalog.games].sort((a, b) => (b.recomp?.stars ?? 0) - (a.recomp?.stars ?? 0));
  const featured = byUpdate[0];

  return (
    <div className="page home">
      {featured && (
        <button className="feature" onClick={() => navigate({ page: "game", gameId: featured.id })}>
          <GameArt game={featured} kind="hero" className="feature-hero" />
          <div className="feature-overlay">
            <span className="feature-label">Atualizado {formatRelative(featured.recomp?.latestRelease?.publishedAt)}</span>
            <GameArt game={featured} kind="logo" className="feature-logo" />
            <span className="feature-tag">
              {featured.recomp?.latestRelease?.name || featured.recomp?.latestRelease?.tag}
            </span>
          </div>
        </button>
      )}

      <Shelf
        title="Atualizações recentes"
        games={byUpdate}
        caption={(g) => `${g.recomp?.latestRelease?.tag ?? ""} · ${formatRelative(g.recomp?.latestRelease?.publishedAt)}`}
      />
      <Shelf title="Novos recomps" games={byLaunch} caption={(g) => `Lançado ${formatRelative(g.recomp?.firstReleaseAt)}`} />
      <Shelf title="Mais populares" games={byStars} caption={(g) => `★ ${g.recomp?.stars ?? 0}`} />
    </div>
  );
}

function Shelf({ title, games, caption }: { title: string; games: Game[]; caption: (g: Game) => string }) {
  return (
    <section>
      <h2 className="section-title">{title}</h2>
      <div className="shelf">
        {games.slice(0, 12).map((g) => (
          <GameCard key={g.id} game={g} caption={caption(g)} />
        ))}
      </div>
    </section>
  );
}
