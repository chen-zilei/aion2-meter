/**
 * AION 2 server-wide timed events, from community guides in October 2026.
 * Daily and weekly resets live in ../tracker/activities.ts (REGIONS, nextReset); this file
 * adds everything else on a clock, keyed by the same region ids.
 *
 * Times are written in each region's server time. The Global client runs on UTC+9
 * in every region, Asia included (its 16:00 server-time reset is 07:00 UTC), Korea on KST (UTC+9), Taiwan on UTC+8.
 */

export type RegionId = "global" | "asia" | "kr" | "tw";

/** Server clock offset from UTC, in hours. */
export const SERVER_UTC_OFFSET: Record<RegionId, number> = { global: 9, asia: 9, kr: 9, tw: 8 };

export interface TimedEvent {
  id: string;
  name: string;
  regions: RegionId[];
  /** Server-time "HH:MM" start times. */
  times: string[];
  /** Server-time weekdays it runs on, 0 = Sunday. Every day when omitted. */
  days?: number[];
  /** How long it stays open, in minutes, when known. */
  openMin?: number;
  note?: string;
  /** Set when the time comes from a single source or the sources disagree. */
  unconfirmed?: string;
}

const everyHours = (step: number, first: number, minute = 0) =>
  Array.from({ length: 24 / step }, (_, i) => `${String(first + i * step).padStart(2, "0")}:${String(minute).padStart(2, "0")}`);

export const EVENTS: TimedEvent[] = [
  {
    id: "rift-global", name: "Spacetime Rift portals", regions: ["global"],
    times: everyHours(3, 0), openMin: 10,
    note: "Every 3 hours. The entrance closes after 10 minutes and fills fast; you get 1 hour inside.",
    unconfirmed: "From guides, not checked in game. The Asia server turned out to differ from these guides, so NA and EU may too.",
  },
  {
    id: "rift-asia", name: "Spacetime Rift portals", regions: ["asia"],
    times: everyHours(3, 2), openMin: 10,
    note: "Every 3 hours (confirmed in game, October 2026). The entrance closes after 10 minutes and fills fast; you get 1 hour inside.",
  },
  {
    id: "rift-krtw", name: "Spacetime Rift portals", regions: ["kr", "tw"],
    times: everyHours(3, 2), openMin: 10,
    note: "Every 3 hours. The entrance closes after 10 minutes and fills fast; you get 1 hour inside.",
    unconfirmed: "Only one source (aion2hub) lists the Korea and Taiwan times.",
  },
  {
    id: "field-hourly", name: "Field events and minigames", regions: ["global", "asia", "kr", "tw"],
    times: everyHours(1, 0),
    note: "Shugo Festival, invasions and minigames start on the hour.",
  },
  {
    id: "beritra", name: "Beritra Air Raid", regions: ["global", "asia", "kr", "tw"],
    times: everyHours(1, 0, 30),
    unconfirmed: "Only one source lists it, and doesn't say which regions have it.",
  },
  {
    id: "nahma", name: "Guardian Lord Nahma (Lower and Middle Reshanta)", regions: ["global", "asia"],
    times: ["21:00"], days: [0, 5],
    unconfirmed: "Only one source lists Abyss bosses, and it doesn't name the time zone; shown as server time (UTC+9).",
  },
  {
    id: "executors", name: "Abyss Executors (Lower and Middle Reshanta)", regions: ["global", "asia"],
    times: ["21:30"], days: [1, 4, 6],
    note: "Tamasa, Kaira and Argo in Lower; Ducal, Marakha and Dramos in Middle.",
    unconfirmed: "Only one source lists Abyss bosses, and it doesn't name the time zone; shown as server time (UTC+9).",
  },
  {
    id: "abyss-rift-zone", name: "Abyss Rift Zone (PvP)", regions: ["kr", "tw"],
    times: ["22:00"], days: [2, 4], openMin: 5,
    note: "Queue opens for 5 minutes, then a 30-minute match.",
  },
  {
    id: "rift-domination", name: "Spacetime Rift Domination (PvP)", regions: ["kr", "tw"],
    times: ["20:00", "23:00"], days: [1, 4, 6], openMin: 5,
    note: "5 minutes of prep, then a 15-minute match. Entry closes 5 minutes after the start.",
  },
];

const MIN = 60_000;
const DAY = 1440 * MIN;

/** Most recent start at or before `now`, in epoch ms. */
export function lastStart(event: TimedEvent, region: RegionId, now: number): number {
  return occurrence(event, region, now, -1);
}

/** Next start after `now`, in epoch ms. */
export function nextStart(event: TimedEvent, region: RegionId, now: number): number {
  return occurrence(event, region, now, 1);
}

function occurrence(event: TimedEvent, region: RegionId, now: number, dir: 1 | -1): number {
  const offset = SERVER_UTC_OFFSET[region] * 60 * MIN;
  // Work in a shifted clock where UTC fields read as server time.
  const server = now + offset;
  const midnight = Math.floor(server / DAY) * DAY;
  const minutes = event.times.map((t) => {
    const [h, m] = t.split(":").map(Number);
    return h * 60 + m;
  });
  let best = NaN;
  for (let d = -8; d <= 8; d++) {
    const day = midnight + d * DAY;
    if (event.days && !event.days.includes(new Date(day).getUTCDay())) continue;
    for (const m of minutes) {
      const t = day + m * MIN;
      if (dir === 1 ? t > server && !(t >= best) : t <= server && !(t <= best)) best = t;
    }
  }
  return best - offset;
}
