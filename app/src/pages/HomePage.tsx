import { useEffect, useState } from "react";
import { useStore } from "../store";
import { GameArt } from "../components/GameArt";
import { GameCard } from "../components/GameCard";
import { formatRelative } from "../format";
import type { Game } from "../types";
import { t } from "../i18n";

const time = (iso: string | null | undefined) => (iso ? new Date(iso).getTime() : 0);
const updatedAt = (g: Game) => time(g.recomp?.latestRelease?.publishedAt);
const launchedAt = (g: Game) => time(g.recomp?.firstReleaseAt);

const SLIDES = 8;
const SLIDE_MS = 6000;

export function HomePage() {
  const { catalog } = useStore();
  if (!catalog) return <p className="page muted">{t("Carregando…")}</p>;

  const byUpdate = [...catalog.games].sort((a, b) => updatedAt(b) - updatedAt(a));
  const byLaunch = [...catalog.games].sort((a, b) => launchedAt(b) - launchedAt(a));
  const byStars = [...catalog.games].sort((a, b) => (b.recomp?.stars ?? 0) - (a.recomp?.stars ?? 0));

  return (
    <div className="page home">
      <UpdatesCarousel games={byUpdate.slice(0, SLIDES)} />
      <Shelf title={t("Novos recomps")} games={byLaunch} caption={(g) => t("Lançado {when}", { when: formatRelative(g.recomp?.firstReleaseAt) })} />
      <Shelf title={t("Mais populares")} games={byStars} caption={(g) => `★ ${g.recomp?.stars ?? 0}`} />
    </div>
  );
}

/** Banners dos jogos atualizados mais recentemente; troca sozinho e pausa com o mouse em cima. */
function UpdatesCarousel({ games }: { games: Game[] }) {
  const { navigate } = useStore();
  const [index, setIndex] = useState(0);
  const [paused, setPaused] = useState(false);
  const count = games.length;

  useEffect(() => {
    if (paused || count < 2) return;
    const timer = setTimeout(() => setIndex((i) => (i + 1) % count), SLIDE_MS);
    return () => clearTimeout(timer);
  }, [index, paused, count]);

  if (count === 0) return null;
  const go = (delta: number) => setIndex((i) => (i + delta + count) % count);

  return (
    <section
      className="carousel"
      aria-roledescription={t("carrossel")}
      aria-label={t("Atualizações recentes")}
      onMouseEnter={() => setPaused(true)}
      onMouseLeave={() => setPaused(false)}
    >
      {games.map((g, i) => {
        const release = g.recomp?.latestRelease;
        return (
          <button
            key={g.id}
            className={`feature carousel-slide ${i === index ? "carousel-slide-active" : ""}`}
            onClick={() => navigate({ page: "game", gameId: g.id })}
            aria-hidden={i !== index}
            tabIndex={i === index ? 0 : -1}
          >
            <GameArt game={g} kind="hero" className="feature-hero" />
            <div className="feature-overlay">
              <span className="feature-label">{t("Atualizado {when}", { when: formatRelative(release?.publishedAt) })}</span>
              <GameArt game={g} kind="logo" className="feature-logo" />
              <span className="feature-tag">
                {g.name} · {release?.tag}
              </span>
            </div>
          </button>
        );
      })}

      {count > 1 && (
        <>
          <button className="carousel-arrow carousel-prev" onClick={() => go(-1)} aria-label={t("Anterior")}>
            ‹
          </button>
          <button className="carousel-arrow carousel-next" onClick={() => go(1)} aria-label={t("Próximo")}>
            ›
          </button>
          <div className="carousel-dots">
            {games.map((g, i) => (
              <button
                key={g.id}
                className={`carousel-dot ${i === index ? "carousel-dot-active" : ""}`}
                onClick={() => setIndex(i)}
                aria-label={t("Ir para o banner {n}", { n: i + 1 })}
              />
            ))}
          </div>
        </>
      )}
    </section>
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
