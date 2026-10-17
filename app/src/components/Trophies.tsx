import { useEffect, useState } from "react";
import { api } from "../api";
import { useStore } from "../store";
import { formatDate } from "../format";
import type { Game, TrophiesView, TrophyView } from "../types";

const CATEGORY_LABELS: Record<string, string> = {
  challenge: "Desafios",
  collection: "Coleção",
  minigame: "Minigames",
  misc: "Diversos",
  glitched: "Glitches",
};

const PROVIDER_LABELS: Record<string, string> = {
  dusklight: "Conquistas do próprio Dusklight",
};

/** Lista de conquistas do jogo na Biblioteca, marcadas à mão pelo usuário. */
export function TrophiesSection({ game }: { game: Game }) {
  const { library, setTrophy } = useStore();
  const [view, setView] = useState<TrophiesView | null>(null);
  const marks = library[game.id]?.trophies ?? {};
  const marksKey = Object.keys(marks).sort().join(",");

  // Recarrega ao trocar de jogo e a cada marcação.
  useEffect(() => {
    api.getTrophies(game.id).then(setView).catch(() => setView(null));
  }, [game.id, marksKey]);

  const trophies = view?.trophies ?? [];
  const done = trophies.filter((t) => t.unlocked).length;
  const provider = view?.provider ?? null;

  // Agrupa por categoria quando o jogo usa categorias.
  const groups = new Map<string, TrophyView[]>();
  for (const t of trophies) {
    const list = groups.get(t.category) ?? [];
    list.push(t);
    groups.set(t.category, list);
  }

  return (
    <section>
      <div className="trophies-head">
        <h2 className="section-title">
          Troféus {trophies.length > 0 && <span className="muted">· {done}/{trophies.length}</span>}
        </h2>
      </div>

      {trophies.length > 0 && (
        <div className="trophy-progress">
          <div style={{ width: `${(done / trophies.length) * 100}%` }} />
        </div>
      )}

      {provider && (
        <p className="muted small">
          {PROVIDER_LABELS[provider] ?? "Conquistas do próprio jogo"}. Marque as que você já fez; a detecção
          automática virá numa versão futura.
        </p>
      )}

      {view?.loadError && <p className="settings-error">Arquivo de troféus inválido: {view.loadError}</p>}

      {trophies.length === 0 && !view?.loadError && (
        <p className="muted small">Este jogo ainda não tem uma lista de conquistas.</p>
      )}

      {[...groups].map(([category, list]) => (
        <div key={category}>
          {category && (
            <h4 className="trophy-category">
              {CATEGORY_LABELS[category] ?? category}{" "}
              <span className="muted">
                {list.filter((t) => t.unlocked).length}/{list.length}
              </span>
            </h4>
          )}
          <ul className="trophy-list">
            {list.map((t) => (
              <TrophyRow key={t.key} trophy={t} onToggle={(on) => setTrophy(game.id, t.key, on)} />
            ))}
          </ul>
        </div>
      ))}
    </section>
  );
}

function TrophyRow({ trophy: t, onToggle }: { trophy: TrophyView; onToggle: (unlocked: boolean) => void }) {
  return (
    <li>
      <label className={`trophy trophy-toggle ${t.unlocked ? "trophy-unlocked" : ""}`}>
        <input
          type="checkbox"
          className="trophy-check"
          checked={t.unlocked}
          onChange={(e) => onToggle(e.target.checked)}
          aria-label={`${t.title}: ${t.unlocked ? "feita" : "não feita"}`}
        />
        <span className="trophy-icon">{t.unlocked ? "🏆" : "🔒"}</span>
        <span className="trophy-text">
          <strong>{t.title}</strong>
          <span className="muted small">{t.description}</span>
          {t.error && <span className="trophy-error">Condição inválida: {t.error}</span>}
        </span>
        <span className="trophy-meta">
          {t.points > 0 && <span>{t.points} pts</span>}
          {t.unlockedAt && <span className="muted small">{formatDate(t.unlockedAt)}</span>}
        </span>
      </label>
    </li>
  );
}

/** Notificações de troféu no canto da tela; somem sozinhas. */
export function TrophyToasts() {
  const { unlocks, dismissUnlock, gameById } = useStore();

  useEffect(() => {
    const timers = unlocks.map((u) => setTimeout(() => dismissUnlock(u.toastId), 7000));
    return () => timers.forEach(clearTimeout);
  }, [unlocks]);

  return (
    <div className="trophy-toasts">
      {unlocks.map((u) => (
        <button key={u.toastId} className="trophy-toast" onClick={() => dismissUnlock(u.toastId)}>
          <span className="trophy-icon">🏆</span>
          <span className="trophy-text">
            <span className="trophy-toast-label">Troféu desbloqueado · {gameById(u.gameId)?.name}</span>
            <strong>{u.title}</strong>
            <span className="muted small">{u.description}</span>
          </span>
        </button>
      ))}
    </div>
  );
}
