import { useEffect, useState } from "react";
import { api } from "../api";
import { useStore } from "../store";
import { formatDate } from "../format";
import type { Game, TrophiesView } from "../types";

/** Lista de troféus do jogo na Biblioteca. */
export function TrophiesSection({ game }: { game: Game }) {
  const { library } = useStore();
  const [view, setView] = useState<TrophiesView | null>(null);
  const unlockedCount = Object.keys(library[game.id]?.trophies ?? {}).length;

  const reload = () => api.getTrophies(game.id).then(setView).catch(() => setView(null));

  // Recarrega ao trocar de jogo e a cada desbloqueio.
  useEffect(() => {
    reload();
  }, [game.id, unlockedCount]);

  const edit = async () => {
    await api.editTrophies(game.id);
    reload();
  };

  const trophies = view?.trophies ?? [];
  const done = trophies.filter((t) => t.unlockedAt).length;

  return (
    <section>
      <div className="trophies-head">
        <h2 className="section-title">
          Troféus {trophies.length > 0 && <span className="muted">· {done}/{trophies.length}</span>}
        </h2>
        <div className="trophies-actions">
          <button className="btn btn-ghost" onClick={edit} title={view?.path}>
            {trophies.length ? "Editar troféus" : "Criar troféus"}
          </button>
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

      {view?.loadError && <p className="settings-error">Arquivo de troféus inválido: {view.loadError}</p>}

      {trophies.length === 0 && !view?.loadError && (
        <p className="muted small">
          Nenhum troféu definido. Experimental: os troféus são escritos num arquivo local, com condições no formato
          do RetroAchievements, e avaliados enquanto o jogo roda.
        </p>
      )}

      <ul className="trophy-list">
        {trophies.map((t) => (
          <li key={t.id} className={`trophy ${t.unlockedAt ? "trophy-unlocked" : ""}`}>
            <span className="trophy-icon">{t.unlockedAt ? "🏆" : "🔒"}</span>
            <div className="trophy-text">
              <strong>{t.title}</strong>
              <span className="muted small">{t.description}</span>
              {t.error && <span className="trophy-error">Condição inválida: {t.error}</span>}
            </div>
            <div className="trophy-meta">
              {t.points > 0 && <span>{t.points} pts</span>}
              {t.unlockedAt && <span className="muted small">{formatDate(t.unlockedAt)}</span>}
            </div>
          </li>
        ))}
      </ul>
    </section>
  );
}

/** Notificações de troféu no canto da tela; somem sozinhas. */
export function TrophyToasts() {
  const { unlocks, dismissUnlock, gameById } = useStore();

  useEffect(() => {
    const timers = unlocks.map((u) => setTimeout(() => dismissUnlock(u.key), 7000));
    return () => timers.forEach(clearTimeout);
  }, [unlocks]);

  return (
    <div className="trophy-toasts">
      {unlocks.map((u) => (
        <button key={u.key} className="trophy-toast" onClick={() => dismissUnlock(u.key)}>
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
