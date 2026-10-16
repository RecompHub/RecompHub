import { StoreProvider, useStore } from "./store";
import { TopBar } from "./components/TopBar";
import { UpdateBanner } from "./components/UpdateBanner";
import { HomePage } from "./pages/HomePage";
import { ExplorePage } from "./pages/ExplorePage";
import { LibraryPage } from "./pages/LibraryPage";
import { GamePage } from "./pages/GamePage";
import { SettingsPage } from "./pages/SettingsPage";
import "./App.css";

function Shell() {
  const { route, error, dismissError } = useStore();
  return (
    <div className="shell">
      <TopBar />
      <UpdateBanner />
      <main className="content">
        {route.page === "home" && <HomePage />}
        {route.page === "explore" && <ExplorePage />}
        {route.page === "library" && <LibraryPage gameId={route.gameId} />}
        {route.page === "game" && <GamePage key={route.gameId} gameId={route.gameId} />}
        {route.page === "settings" && <SettingsPage />}
      </main>
      {error && (
        <div className="toast" role="alert">
          <span>{error}</span>
          <button className="btn btn-ghost" onClick={dismissError}>
            Fechar
          </button>
        </div>
      )}
    </div>
  );
}

export default function App() {
  return (
    <StoreProvider>
      <Shell />
    </StoreProvider>
  );
}
