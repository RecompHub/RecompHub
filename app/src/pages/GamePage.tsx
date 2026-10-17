import { useStore } from "../store";
import { GameArt } from "../components/GameArt";
import { GameActions } from "../components/GameActions";
import { InfoPanel, ProjectSection, ReleaseNotes, Screenshots } from "../components/GameDetails";

/** Página de um jogo do catálogo (vinda do Início ou do Explorar). */
export function GamePage({ gameId }: { gameId: string }) {
  const { gameById } = useStore();
  const game = gameById(gameId);
  if (!game) return <p className="page muted">Jogo não encontrado no catálogo.</p>;

  return (
    <div className="page game-page">
      <div className="hero">
        <GameArt game={game} kind="hero" className="hero-img" />
        <div className="hero-overlay">
          <GameArt game={game} kind="logo" className="hero-logo" />
        </div>
      </div>
      <div className="action-bar">
        <GameActions game={game} showLibraryButton />
      </div>

      <div className="detail-grid">
        <div className="detail-main">
          <ProjectSection game={game} />
          <section>
            <h2 className="section-title">Sobre</h2>
            <p className="summary">{game.game?.summary ?? game.recomp?.description ?? "Sem descrição."}</p>
          </section>
          <Screenshots game={game} />
          <ReleaseNotes game={game} />
        </div>
        <InfoPanel game={game} />
      </div>
    </div>
  );
}
