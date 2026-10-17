import { useStore, type Route } from "../store";
import { t } from "../i18n";

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
      <button className="topbar-back" onClick={goBack} disabled={!canGoBack} title={t("Voltar")}>
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
            {t(tab.label)}
          </button>
        ))}
      </nav>
      <button className="btn btn-ghost" onClick={refreshCatalog} disabled={refreshing} title={t("Baixar a lista de jogos mais recente")}>
        {refreshing ? t("Atualizando…") : t("⟳ Atualizar lista")}
      </button>
      <button
        className={`topbar-icon ${route.page === "settings" ? "topbar-icon-active" : ""}`}
        onClick={() => navigate({ page: "settings" })}
        title={t("Configurações")}
        aria-label={t("Configurações")}
      >
        ⚙
      </button>
    </header>
  );
}
