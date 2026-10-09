const compact = new Intl.NumberFormat("en", { notation: "compact", maximumFractionDigits: 1 });
const whole = new Intl.NumberFormat("en");

export const fmtCompact = (n: number) => compact.format(n);
export const fmtWhole = (n: number) => whole.format(Math.round(n));
export const fmtPct = (n: number) => `${(n * 100).toFixed(1)}%`;

export function fmtDuration(s: number) {
  const m = Math.floor(s / 60);
  const sec = Math.floor(s % 60);
  return `${m}:${sec.toString().padStart(2, "0")}`;
}

export function fmtTime(ms: number) {
  return new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

/** Skill names are not decoded yet; show the id until a skill table is added. */
export const skillName = (id: number) => `Skill ${id}`;

/** Stable colour per actor so bars keep their colour as ranks change. */
export function actorColor(id: number, isSelf: boolean) {
  if (isSelf) return "var(--self)";
  // No yellows: those are reserved for you.
  const hues = [205, 160, 280, 340, 95, 250, 185, 310];
  return `hsl(${hues[id % hues.length]} 65% 55%)`;
}
