import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import type { Catalog, Game, InstallProgress, Library } from "./types";

export type Route =
  | { page: "home" }
  | { page: "explore" }
  | { page: "library"; gameId?: string }
  | { page: "game"; gameId: string }
  | { page: "settings" };

interface Store {
  catalog: Catalog | null;
  platform: string;
  library: Library;
  running: Set<string>;
  progress: Record<string, InstallProgress>;
  refreshing: boolean;
  /** Último erro de uma ação, mostrado num aviso no rodapé. */
  error: string | null;
  route: Route;
  canGoBack: boolean;

  navigate: (route: Route) => void;
  goBack: () => void;
  dismissError: () => void;
  refreshCatalog: () => Promise<void>;
  addToLibrary: (id: string) => Promise<void>;
  removeFromLibrary: (id: string) => Promise<void>;
  install: (id: string) => Promise<void>;
  uninstall: (id: string) => Promise<void>;
  importInstall: (id: string, path: string) => Promise<void>;
  launch: (id: string) => Promise<void>;
  openInstallDir: (id: string) => Promise<void>;
  gameById: (id: string) => Game | undefined;
}

const StoreContext = createContext<Store | null>(null);

export function useStore(): Store {
  const store = useContext(StoreContext);
  if (!store) throw new Error("useStore fora do StoreProvider");
  return store;
}

export function StoreProvider({ children }: { children: ReactNode }) {
  const [catalog, setCatalog] = useState<Catalog | null>(null);
  const [platform, setPlatform] = useState("");
  const [library, setLibrary] = useState<Library>({});
  const [running, setRunning] = useState<Set<string>>(new Set());
  const [progress, setProgress] = useState<Record<string, InstallProgress>>({});
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [history, setHistory] = useState<Route[]>([{ page: "home" }]);

  const route = history[history.length - 1];

  const refreshCatalog = useCallback(async () => {
    setRefreshing(true);
    try {
      setCatalog(await api.refreshCatalog());
    } catch (e) {
      setError(`Não foi possível atualizar a lista de jogos: ${e}`);
    } finally {
      setRefreshing(false);
    }
  }, []);

  useEffect(() => {
    Promise.all([api.getCatalog(), api.currentPlatform(), api.getLibrary(), api.runningGames()])
      .then(([c, p, l, r]) => {
        setCatalog(c);
        setPlatform(p);
        setLibrary(l);
        setRunning(new Set(r));
      })
      .catch((e) => setError(String(e)));

    const unlisteners = [
      listen<InstallProgress>("install-progress", (e) =>
        setProgress((prev) => ({ ...prev, [e.payload.gameId]: e.payload })),
      ),
      listen<string>("game-started", (e) => setRunning((prev) => new Set(prev).add(e.payload))),
      listen<{ gameId: string }>("game-exited", (e) => {
        setRunning((prev) => {
          const next = new Set(prev);
          next.delete(e.payload.gameId);
          return next;
        });
        api.getLibrary().then(setLibrary);
      }),
    ];
    return () => unlisteners.forEach((u) => u.then((f) => f()));
  }, []);

  // Busca o catálogo publicado em segundo plano; sem rede, fica o local.
  useEffect(() => {
    api.refreshCatalog().then(setCatalog).catch(() => {});
  }, []);

  const run = useCallback(async (action: () => Promise<Library | void>) => {
    try {
      const result = await action();
      if (result) setLibrary(result);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const store = useMemo<Store>(
    () => ({
      catalog,
      platform,
      library,
      running,
      progress,
      refreshing,
      error,
      route,
      canGoBack: history.length > 1,
      navigate: (r) => setHistory((h) => [...h.slice(-30), r]),
      goBack: () => setHistory((h) => (h.length > 1 ? h.slice(0, -1) : h)),
      dismissError: () => setError(null),
      refreshCatalog,
      addToLibrary: (id) => run(() => api.addToLibrary(id)),
      removeFromLibrary: (id) => run(() => api.removeFromLibrary(id)),
      install: async (id) => {
        await run(() => api.installGame(id));
        setProgress(({ [id]: _, ...rest }) => rest);
      },
      uninstall: (id) => run(() => api.uninstallGame(id)),
      importInstall: (id, path) => run(() => api.importGame(id, path)),
      launch: (id) => run(() => api.launchGame(id)),
      openInstallDir: (id) => run(() => api.openInstallDir(id)),
      gameById: (id) => catalog?.games.find((g) => g.id === id),
    }),
    [catalog, platform, library, running, progress, refreshing, error, route, history.length, refreshCatalog, run],
  );

  return <StoreContext.Provider value={store}>{children}</StoreContext.Provider>;
}

/** Estado derivado de um jogo, usado pelos botões e listas. */
export function useGameStatus(game: Game) {
  const { library, running, progress, platform } = useStore();
  const entry = library[game.id];
  const installed = entry?.installed ?? null;
  const latestTag = game.recomp?.latestRelease?.tag;
  return {
    entry,
    inLibrary: entry !== undefined,
    installed,
    supported: platform in game.platforms,
    running: running.has(game.id),
    progress: progress[game.id],
    updateAvailable: !!installed && !!latestTag && installed.installedTag !== latestTag,
  };
}
