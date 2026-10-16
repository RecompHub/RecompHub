// Espelha crates/catalog/src/lib.rs e app/src-tauri/src/state.rs (serde camelCase).

export interface Catalog {
  schemaVersion: number;
  generatedAt: string;
  games: Game[];
}

export interface Game {
  id: string;
  name: string;
  repo: string;
  originalTitle: string;
  originalPlatform: string;
  platforms: Record<string, PlatformAsset>;
  requiredRom: { description: string; sha1: string[] };
  achievements: { enabled: boolean; supportedVersions: string[] };
  sgdbId?: number;
  igdbId?: number;
  images: Images;
  recomp?: RecompInfo;
  game?: GameInfo;
}

export interface PlatformAsset {
  assetPattern: string;
  executable: string;
}

export interface Images {
  grid?: string;
  hero?: string;
  logo?: string;
  icon?: string;
}

export interface RecompInfo {
  description: string | null;
  homepage: string | null;
  license: string | null;
  topics: string[];
  stars: number;
  firstReleaseAt: string | null;
  latestRelease: ReleaseSummary | null;
  releaseCount: number;
  totalDownloads: number;
}

export interface ReleaseSummary {
  tag: string;
  name: string | null;
  publishedAt: string | null;
  htmlUrl: string;
}

export interface GameInfo {
  title: string;
  releaseDate: string | null;
  developers: string[];
  publishers: string[];
  genres: string[];
  summary: string | null;
  rating: number | null;
  url: string | null;
  screenshots: string[];
}

export type Library = Record<string, LibraryEntry>;

export interface LibraryEntry {
  addedAt: string;
  installed: InstalledGame | null;
  playTimeSeconds: number;
  lastPlayedAt: string | null;
  /** id do troféu → quando foi desbloqueado. */
  trophies: Record<string, string>;
}

export interface InstalledGame {
  installedTag: string;
  installedAt: string;
  installPath: string;
  executable: string;
  executableHash: string;
  /** Instalada fora do launcher; desinstalar não apaga os arquivos. */
  imported: boolean;
}

/** Release completa, buscada sob demanda (notas da versão). */
export interface Release {
  tagName: string;
  name: string | null;
  publishedAt: string | null;
  body: string | null;
  htmlUrl: string;
}

export interface InstallProgress {
  gameId: string;
  stage: "download" | "extract";
  downloaded: number;
  total: number;
}

export interface SettingsView {
  gamesDir: string;
  defaultGamesDir: string;
  isDefaultGamesDir: boolean;
}

export interface Trophy {
  id: number;
  title: string;
  description: string;
  points: number;
  conditions: string;
}

export interface TrophyView extends Trophy {
  unlockedAt: string | null;
  error: string | null;
}

export interface TrophiesView {
  path: string;
  trophies: TrophyView[];
  loadError: string | null;
}

export interface TrophyUnlocked {
  gameId: string;
  id: number;
  title: string;
  description: string;
}
