import { useState } from "react";
import { gameTitle } from "../format";
import type { Game } from "../types";

type Kind = "grid" | "hero" | "logo" | "icon";

/**
 * Imagem do SteamGridDB com placeholder quando falta a imagem ou ela não carrega.
 * O ícone que não abre (ex.: .ico malformado) cai para a capa; o logo, sem
 * imagem, vira o nome do jogo em texto.
 */
export function GameArt({ game, kind, className = "" }: { game: Game; kind: Kind; className?: string }) {
  const candidates = [game.images[kind], kind === "icon" ? game.images.grid : undefined].filter(
    (u): u is string => !!u,
  );
  const [attempt, setAttempt] = useState(0);
  const url = candidates[attempt];

  if (url) {
    return (
      <img
        className={`art art-${kind} ${className}`}
        src={url}
        alt={kind === "logo" ? gameTitle(game) : ""}
        loading="lazy"
        draggable={false}
        onError={() => setAttempt((a) => a + 1)}
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
