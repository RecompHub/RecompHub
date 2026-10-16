import { useStore, type Route } from "../store";

const TABS: { page: Route["page"]; label: string }[] = [
  { page: "home", label: "Início" },
  { page: "explore", label: "Explorar" },
  { page: "library", label: "Biblioteca" },
];

export function TopBar() {
  const { route, navigate, goBack, canGoBack, refreshCatalog, refreshing } = useStore();
  // A página de um jogo pertence à aba de onde veio; aqui destacamos "Explorar".
  const active = route.page === "game" ? "explore" : route.page;

  return (
    <header className="topbar">
      <button className="topbar-back" onClick={goBack} disabled={!canGoBack} title="Voltar">
        ‹
      </button>
      <span className="brand">RecompHub</span>
      <nav className="tabs">
        {TABS.map((tab) => (
          <button
            key={tab.page}
            className={`tab ${active === tab.page ? "tab-active" : ""}`}
            onClick={() => navigate({ page: tab.page } as Route)}
          >
            {tab.label}
          </button>
        ))}
      </nav>
      <button className="btn btn-ghost" onClick={refreshCatalog} disabled={refreshing} title="Baixar a lista de jogos mais recente">
        {refreshing ? "Atualizando…" : "⟳ Atualizar lista"}
      </button>
      <button
        className={`topbar-icon ${route.page === "settings" ? "topbar-icon-active" : ""}`}
        onClick={() => navigate({ page: "settings" })}
        title="Configurações"
        aria-label="Configurações"
      >
        ⚙
      </button>
    </header>
  );
}
