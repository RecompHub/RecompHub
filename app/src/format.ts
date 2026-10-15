const dateFormat = new Intl.DateTimeFormat("pt-BR", { day: "2-digit", month: "short", year: "numeric" });
const numberFormat = new Intl.NumberFormat("pt-BR", { notation: "compact", maximumFractionDigits: 1 });

export function formatDate(iso: string | null | undefined): string {
  if (!iso) return "—";
  // Datas sem hora (ex.: "1997-04-27") seriam lidas como UTC e podem voltar um dia.
  const date = iso.length === 10 ? new Date(`${iso}T12:00:00`) : new Date(iso);
  return dateFormat.format(date);
}

export function formatYear(iso: string | null | undefined): string {
  return iso ? iso.slice(0, 4) : "—";
}

export function formatCount(n: number | undefined): string {
  return n === undefined ? "—" : numberFormat.format(n);
}

export function formatPlayTime(seconds: number): string {
  if (seconds < 60) return seconds === 0 ? "Nunca jogado" : "Menos de 1 min";
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes} min`;
  const hours = seconds / 3600;
  return `${hours.toLocaleString("pt-BR", { maximumFractionDigits: 1 })} h`;
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

/** "há 3 dias", "há 2 meses"... */
export function formatRelative(iso: string | null | undefined): string {
  if (!iso) return "—";
  const days = Math.floor((Date.now() - new Date(iso).getTime()) / 86_400_000);
  if (days < 1) return "hoje";
  if (days < 2) return "ontem";
  if (days < 30) return `há ${days} dias`;
  const months = Math.floor(days / 30);
  if (months < 12) return months === 1 ? "há 1 mês" : `há ${months} meses`;
  const years = Math.floor(days / 365);
  return years === 1 ? "há 1 ano" : `há ${years} anos`;
}
