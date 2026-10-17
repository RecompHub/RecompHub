import { locale, t } from "./i18n";

// Criados a cada chamada: o idioma pode mudar com o app aberto.
const dateFormat = () => new Intl.DateTimeFormat(locale(), { day: "2-digit", month: "short", year: "numeric" });
const numberFormat = () => new Intl.NumberFormat(locale(), { notation: "compact", maximumFractionDigits: 1 });

export function formatDate(iso: string | null | undefined): string {
  if (!iso) return "—";
  // Datas sem hora (ex.: "1997-04-27") seriam lidas como UTC e podem voltar um dia.
  const date = iso.length === 10 ? new Date(`${iso}T12:00:00`) : new Date(iso);
  return dateFormat().format(date);
}

export function formatYear(iso: string | null | undefined): string {
  return iso ? iso.slice(0, 4) : "—";
}

export function formatCount(n: number | undefined): string {
  return n === undefined ? "—" : numberFormat().format(n);
}

export function formatPlayTime(seconds: number): string {
  if (seconds < 60) return seconds === 0 ? t("Nunca jogado") : t("Menos de 1 min");
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes} min`;
  const hours = seconds / 3600;
  return `${hours.toLocaleString(locale(), { maximumFractionDigits: 1 })} h`;
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

/** "há 3 dias", "há 2 meses"... ("3 days ago" em inglês) */
export function formatRelative(iso: string | null | undefined): string {
  if (!iso) return "—";
  const days = Math.floor((Date.now() - new Date(iso).getTime()) / 86_400_000);
  if (days < 1) return t("hoje");
  if (days < 2) return t("ontem");
  if (days < 30) return t("há {n} dias", { n: days });
  const months = Math.floor(days / 30);
  if (months < 12) return months === 1 ? t("há 1 mês") : t("há {n} meses", { n: months });
  const years = Math.floor(days / 365);
  return years === 1 ? t("há 1 ano") : t("há {n} anos", { n: years });
}

/** Nome do jogo para exibição (IGDB, ou o título original). O nome do
 * projeto recomp/port (`game.name`) aparece só dentro da página do jogo. */
export function gameTitle(game: { game?: { title: string } | null; originalTitle: string }): string {
  return game.game?.title || game.originalTitle;
}
