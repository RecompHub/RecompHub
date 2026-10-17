import { useStore, useGameStatus } from "../store";
import { GameArt } from "./GameArt";
import { gameTitle } from "../format";
import type { Game } from "../types";
import { t } from "../i18n";

/** Capa vertical clicável, usada nas prateleiras da tela inicial. */
export function GameCard({ game, caption }: { game: Game; caption?: string }) {
  const { navigate } = useStore();
  const s = useGameStatus(game);
  return (
    <button className="game-card" onClick={() => navigate({ page: "game", gameId: game.id })}>
      <div className="game-card-art">
        <GameArt game={game} kind="grid" />
        {s.installed && <span className="badge badge-installed">{t("Instalado")}</span>}
        {!s.installed && s.inLibrary && <span className="badge">{t("Na biblioteca")}</span>}
      </div>
      <span className="game-card-name">{gameTitle(game)}</span>
      {caption && <span className="game-card-caption">{caption}</span>}
    </button>
  );
}
