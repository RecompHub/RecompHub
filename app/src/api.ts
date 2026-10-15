import { invoke } from "@tauri-apps/api/core";
import type { Catalog, Library, Release } from "./types";

// Um wrapper por comando de app/src-tauri/src/lib.rs.
export const api = {
  getCatalog: () => invoke<Catalog>("get_catalog"),
  refreshCatalog: () => invoke<Catalog>("refresh_catalog"),
  currentPlatform: () => invoke<string>("current_platform"),
  getLibrary: () => invoke<Library>("get_library"),
  runningGames: () => invoke<string[]>("running_games"),
  addToLibrary: (gameId: string) => invoke<Library>("add_to_library", { gameId }),
  removeFromLibrary: (gameId: string) => invoke<Library>("remove_from_library", { gameId }),
  latestRelease: (gameId: string) => invoke<Release>("latest_release", { gameId }),
  installGame: (gameId: string) => invoke<Library>("install_game", { gameId }),
  uninstallGame: (gameId: string) => invoke<Library>("uninstall_game", { gameId }),
  launchGame: (gameId: string) => invoke<void>("launch_game", { gameId }),
  openInstallDir: (gameId: string) => invoke<void>("open_install_dir", { gameId }),
};
