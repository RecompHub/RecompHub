import { useState } from "react";
import { gameTitle } from "../format";
import type { Game } from "../types";

type Kind = "grid" | "hero" | "logo" | "icon";

/**
 * Imagem do SteamGridDB com placeholder quando falta a imagem ou ela não carrega.
 * O logo, sem imagem, vira o nome do jogo em texto.
 */
export function GameArt({ game, kind, className = "" }: { game: Game; kind: Kind; className?: string }) {
  const url = game.images[kind];
  const [failed, setFailed] = useState(false);

  if (url && !failed) {
    return (
      <img
        className={`art art-${kind} ${className}`}
        src={url}
        alt={kind === "logo" ? gameTitle(game) : ""}
        loading="lazy"
        draggable={false}
        onError={() => setFailed(true)}
      />
    );
  }

  if (kind === "logo") {
    return <h1 className={`art-logo-text ${className}`}>{gameTitle(game)}</h1>;
  }
  return (
    <div
      className={`art art-${kind} art-placeholder ${className}`}
      style={{ "--hue": hue(game.id) } as React.CSSProperties}
      aria-hidden
    >
      {kind === "grid" && <span>{game.originalTitle}</span>}
      {kind === "icon" && <span>{gameTitle(game).charAt(0)}</span>}
    </div>
  );
}

// Cor estável por jogo, para os placeholders não ficarem todos iguais.
function hue(id: string): number {
  let h = 0;
  for (const c of id) h = (h * 31 + c.charCodeAt(0)) % 360;
  return h;
}
