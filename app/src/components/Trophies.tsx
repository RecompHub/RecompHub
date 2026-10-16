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
  dusklight: "Conquistas detectadas pelo próprio Dusklight",
};

/** Lista de troféus do jogo na Biblioteca. */
export function TrophiesSection({ game }: { game: Game }) {
  const { library, running } = useStore();
  const [view, setView] = useState<TrophiesView | null>(null);
  const unlockedCount = Object.keys(library[game.id]?.trophies ?? {}).length;
  const isRunning = running.has(game.id);

  const reload = () => api.getTrophies(game.id).then(setView).catch(() => setView(null));

  // Recarrega ao trocar de jogo, a cada desbloqueio e ao fechar o jogo.
  useEffect(() => {
    reload();
  }, [game.id, unlockedCount, isRunning]);

  const edit = async () => {
    await api.editTrophies(game.id);
    reload();
  };

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
        <div className="trophies-actions">
          {!provider && (
            <button className="btn btn-ghost" onClick={edit} title={view?.path}>
              {trophies.length ? "Editar troféus" : "Criar troféus"}
            </button>
          )}
          <button className="btn btn-ghost" onClick={reload}>
            Recarregar
          </button>
        </div>
      </div>

      {trophies.length > 0 && (
        <div className="trophy-progress">
          <div style={{ width: `${(done / trophies.length) * 100}%` }} />
        </div>
      )}

      {provider && (
        <p className="muted small">
          {PROVIDER_LABELS[provider] ?? "Conquistas do próprio jogo"}. Atualizam enquanto você joga pelo RecompHub.
        </p>
      )}

      {view?.loadError && <p className="settings-error">Arquivo de troféus inválido: {view.loadError}</p>}

      {trophies.length === 0 && !view?.loadError && (
        <p className="muted small">
          Nenhum troféu definido. Experimental: os troféus são escritos num arquivo local, com condições no formato
          do RetroAchievements, e avaliados enquanto o jogo roda.
        </p>
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
              <TrophyRow key={t.key} trophy={t} />
            ))}
          </ul>
        </div>
      ))}
    </section>
  );
}

function TrophyRow({ trophy: t }: { trophy: TrophyView }) {
  const counter = t.goal > 1;
  return (
    <li className={`trophy ${t.unlocked ? "trophy-unlocked" : ""}`}>
      <span className="trophy-icon">{t.unlocked ? "🏆" : "🔒"}</span>
      <div className="trophy-text">
        <strong>{t.title}</strong>
        <span className="muted small">{t.description}</span>
        {counter && !t.unlocked && (
          <div className="trophy-counter">
            <div className="trophy-progress">
              <div style={{ width: `${Math.min(100, (t.progress / t.goal) * 100)}%` }} />
            </div>
            <span className="small">
              {t.progress}/{t.goal}
            </span>
          </div>
        )}
        {t.error && <span className="trophy-error">Condição inválida: {t.error}</span>}
      </div>
      <div className="trophy-meta">
        {t.points > 0 && <span>{t.points} pts</span>}
        {t.unlockedAt && <span className="muted small">{formatDate(t.unlockedAt)}</span>}
      </div>
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
