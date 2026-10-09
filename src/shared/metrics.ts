import type { ActorStats } from "./types";

/** What the meter ranks by. */
export type Metric = "damage" | "healing" | "taken";

export const METRICS: { id: Metric; label: string; short: string }[] = [
  { id: "damage", label: "Damage", short: "DMG" },
  { id: "healing", label: "Healing", short: "HEAL" },
  { id: "taken", label: "Taken", short: "TAKEN" },
];

export function metricValue(a: ActorStats, m: Metric) {
  return m === "damage" ? a.damage : m === "healing" ? a.healing : a.damageTaken;
}

/** Per second, or null where a rate means little (damage taken). */
export function metricRate(a: ActorStats, m: Metric) {
  return m === "damage" ? a.dps : m === "healing" ? a.hps : null;
}

/** Actors with something to show for `m`, highest first. */
export function ranked(actors: ActorStats[], m: Metric) {
  return actors.filter((a) => metricValue(a, m) > 0).sort((a, b) => metricValue(b, m) - metricValue(a, m));
}

export function metricTotal(actors: ActorStats[], m: Metric) {
  return actors.reduce((sum, a) => sum + metricValue(a, m), 0);
}
