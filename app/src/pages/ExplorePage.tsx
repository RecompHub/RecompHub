import { useMemo, useState } from "react";
import { useStore } from "../store";
import { GameArt } from "../components/GameArt";
import { formatCount, formatDate, formatYear, gameTitle } from "../format";
import type { Game } from "../types";
import { locale, t } from "../i18n";

type SortKey = "updated" | "recompRelease" | "gameRelease" | "name" | "stars" | "downloads";

const SORTS: { key: SortKey; label: string; value: (g: Game) => number | string }[] = [
  { key: "updated", label: "Última atualização", value: (g) => g.recomp?.latestRelease?.publishedAt ?? "" },
  { key: "recompRelease", label: "Lançamento do recomp", value: (g) => g.recomp?.firstReleaseAt ?? "" },
  { key: "gameRelease", label: "Lançamento do jogo", value: (g) => g.game?.releaseDate ?? "" },
  { key: "name", label: "Nome", value: (g) => gameTitle(g).toLocaleLowerCase(locale()) },
  { key: "stars", label: "Estrelas no GitHub", value: (g) => g.recomp?.stars ?? 0 },
  { key: "downloads", label: "Downloads", value: (g) => g.recomp?.totalDownloads ?? 0 },
];

type LibraryFilter = "all" | "in" | "out";

export function ExplorePage() {
  const { catalog, library, platform, navigate } = useStore();
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState<SortKey>("updated");
  // Nome começa em A→Z; os demais, do maior/mais recente para o menor.
  const [ascending, setAscending] = useState(false);
  const [platforms, setPlatforms] = useState<Set<string>>(new Set());
  const [genres, setGenres] = useState<Set<string>>(new Set());
  const [onlySupported, setOnlySupported] = useState(false);
  const [onlyAchievements, setOnlyAchievements] = useState(false);
  const [libraryFilter, setLibraryFilter] = useState<LibraryFilter>("all");

  const games = catalog?.games ?? [];
  const allPlatforms = useMemo(() => unique(games.map((g) => g.originalPlatform)), [games]);
  const allGenres = useMemo(() => unique(games.flatMap((g) => g.game?.genres ?? [])), [games]);

  const results = useMemo(() => {
    const q = query.trim().toLocaleLowerCase(locale());
    const filtered = games.filter((g) => {
      if (q && ![gameTitle(g), g.name, g.originalTitle, g.repo].some((s) => s.toLocaleLowerCase(locale()).includes(q))) return false;
      if (platforms.size && !platforms.has(g.originalPlatform)) return false;
      if (genres.size && !(g.game?.genres ?? []).some((x) => genres.has(x))) return false;
      if (onlySupported && !(platform in g.platforms)) return false;
      if (onlyAchievements && !g.achievements.enabled) return false;
      if (libraryFilter === "in" && !library[g.id]) return false;
      if (libraryFilter === "out" && library[g.id]) return false;
      return true;
    });
    const value = SORTS.find((s) => s.key === sort)!.value;
    return filtered.sort((a, b) => {
      const va = value(a);
      const vb = value(b);
      const cmp = va < vb ? -1 : va > vb ? 1 : 0;
      return ascending ? cmp : -cmp;
    });
  }, [games, query, platforms, genres, onlySupported, onlyAchievements, libraryFilter, library, platform, sort, ascending]);

  const changeSort = (key: SortKey) => {
    setSort(key);
    setAscending(key === "name");
  };

  return (
    <div className="split">
      <aside className="sidebar filters">
        <input
          className="search"
          type="search"
          placeholder={t("Buscar por nome…")}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />

        <h4>{t("Ordenar por")}</h4>
        <div className="sort-row">
          <select value={sort} onChange={(e) => changeSort(e.target.value as SortKey)}>
            {SORTS.map((s) => (
              <option key={s.key} value={s.key}>
                {t(s.label)}
              </option>
            ))}
          </select>
          <button className="btn btn-ghost" onClick={() => setAscending((a) => !a)} title={t("Inverter ordem")}>
            {ascending ? "↑" : "↓"}
          </button>
        </div>

        <h4>{t("Biblioteca")}</h4>
        {(["all", "in", "out"] as const).map((v) => (
          <label key={v} className="check">
            <input type="radio" name="library" checked={libraryFilter === v} onChange={() => setLibraryFilter(v)} />
            {t(v === "all" ? "Todos" : v === "in" ? "Na biblioteca" : "Fora da biblioteca")}
          </label>
        ))}

        <h4>{t("Disponibilidade")}</h4>
        <label className="check">
          <input type="checkbox" checked={onlySupported} onChange={(e) => setOnlySupported(e.target.checked)} />
          {t("Disponível no meu sistema")}
        </label>
        <label className="check">
          <input type="checkbox" checked={onlyAchievements} onChange={(e) => setOnlyAchievements(e.target.checked)} />
          {t("Com conquistas")}
        </label>

        <CheckGroup title={t("Plataforma original")} options={allPlatforms} selected={platforms} onChange={setPlatforms} />
        <CheckGroup title={t("Gênero")} options={allGenres} selected={genres} onChange={setGenres} />
      </aside>

      <section className="results">
        <p className="muted">{t(results.length === 1 ? "{n} resultado" : "{n} resultados", { n: results.length })}</p>
        {results.map((g) => (
          <button key={g.id} className="result-row" onClick={() => navigate({ page: "game", gameId: g.id })}>
            <GameArt game={g} kind="grid" className="result-art" />
            <div className="result-main">
              <strong>
                {gameTitle(g)}
                {g.aiAssisted && (
                  <span className="badge badge-ai" title={t("Projeto feito em grande parte com IA")}>
                    {t("IA")}
                  </span>
                )}
              </strong>
              <span className="muted">
                {[g.originalPlatform, g.game?.releaseDate && formatYear(g.game.releaseDate), g.game?.genres.join(", "), g.name]
                  .filter(Boolean)
                  .join(" · ")}
              </span>
              {g.recomp?.description && <span className="result-desc">{g.recomp.description}</span>}
            </div>
            <div className="result-meta">
              <span>{g.recomp?.latestRelease?.tag ?? "—"}</span>
              <span className="muted">{t("atualizado {date}", { date: formatDate(g.recomp?.latestRelease?.publishedAt) })}</span>
              <span className="muted">★ {formatCount(g.recomp?.stars)}</span>
              {library[g.id] && <span className="badge">{t(library[g.id].installed ? "Instalado" : "Na biblioteca")}</span>}
            </div>
          </button>
        ))}
      </section>
    </div>
  );
}

function CheckGroup({
  title,
  options,
  selected,
  onChange,
}: {
  title: string;
  options: string[];
  selected: Set<string>;
  onChange: (s: Set<string>) => void;
}) {
  if (options.length === 0) return null;
  const toggle = (opt: string) => {
    const next = new Set(selected);
    if (next.has(opt)) next.delete(opt);
    else next.add(opt);
    onChange(next);
  };
  return (
    <>
      <h4>{title}</h4>
      {options.map((opt) => (
        <label key={opt} className="check">
          <input type="checkbox" checked={selected.has(opt)} onChange={() => toggle(opt)} />
          {opt}
        </label>
      ))}
    </>
  );
}

function unique(values: string[]): string[] {
  return [...new Set(values)].sort((a, b) => a.localeCompare(b, locale()));
}
