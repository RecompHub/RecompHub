import { createContext, Fragment, useCallback, useContext, useEffect, useState, type ReactNode } from "react";
import { api } from "./api";
import { EN } from "./locales/en";

/**
 * Idiomas da interface. O texto em português é a própria chave da tradução:
 * `t("Instalar")` devolve "Install" em inglês e o original em português.
 * Texto sem tradução cai no português (e avisa no console em dev).
 */
export type Lang = "pt" | "en";
/** `null` = seguir o idioma do sistema. */
export type LangChoice = Lang | null;

const CACHE_KEY = "recomphub.language";

function systemLang(): Lang {
  return navigator.language.toLowerCase().startsWith("pt") ? "pt" : "en";
}

// A escolha fica no settings.json (backend); o cache local só evita a
// interface piscar no outro idioma enquanto as configurações carregam.
function cachedChoice(): LangChoice {
  try {
    const v = localStorage.getItem(CACHE_KEY);
    return v === "pt" || v === "en" ? v : null;
  } catch {
    return null;
  }
}

function cacheChoice(choice: LangChoice) {
  try {
    if (choice) localStorage.setItem(CACHE_KEY, choice);
    else localStorage.removeItem(CACHE_KEY);
  } catch {
    // Sem armazenamento local: só perde o cache.
  }
}

let current: Lang = cachedChoice() ?? systemLang();

export function lang(): Lang {
  return current;
}

/** Locale para `Intl` e `localeCompare`. */
export function locale(): string {
  return current === "en" ? "en-US" : "pt-BR";
}

export function t(text: string, vars?: Record<string, string | number>): string {
  let out = text;
  if (current === "en") {
    const translated = EN[text];
    if (translated === undefined && import.meta.env.DEV) console.warn(`[i18n] sem tradução: ${text}`);
    out = translated ?? text;
  }
  if (vars) out = out.replace(/\{(\w+)\}/g, (match, key: string) => (key in vars ? String(vars[key]) : match));
  return out;
}

interface LangState {
  lang: Lang;
  choice: LangChoice;
  setChoice: (choice: LangChoice) => void;
}

const LangContext = createContext<LangState | null>(null);

export function useLang(): LangState {
  const state = useContext(LangContext);
  if (!state) throw new Error("useLang fora do LangProvider");
  return state;
}

export function LangProvider({ children }: { children: ReactNode }) {
  const [choice, setChoiceState] = useState<LangChoice>(cachedChoice);
  const [loaded, setLoaded] = useState(false);
  const effective = choice ?? systemLang();
  current = effective;

  useEffect(() => {
    api
      .getSettings()
      .then((s) => {
        const saved = s.language === "pt" || s.language === "en" ? s.language : null;
        setChoiceState(saved);
        cacheChoice(saved);
      })
      .catch(() => {})
      .finally(() => setLoaded(true));
  }, []);

  // Salva a escolha e deixa as mensagens de erro do backend no mesmo idioma.
  useEffect(() => {
    document.documentElement.lang = locale();
    if (loaded) api.setLanguage(choice, effective).catch(() => {});
  }, [loaded, choice, effective]);

  const setChoice = useCallback((next: LangChoice) => {
    setChoiceState(next);
    cacheChoice(next);
  }, []);

  return (
    <LangContext.Provider value={{ lang: effective, choice, setChoice }}>
      {/* Remonta a interface ao trocar de idioma; a rota fica no store. */}
      <Fragment key={effective}>{children}</Fragment>
    </LangContext.Provider>
  );
}
