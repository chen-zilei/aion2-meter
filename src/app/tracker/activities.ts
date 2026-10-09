/**
 * AION 2 daily and weekly activities, as listed by community guides in October 2026.
 * Counts are the Global (NA/EU) values. Where sources disagree, `disputed` says how.
 */

export type Period = "daily" | "weekly";
/** "server": shared by every character on the server. "character": each character has its own. */
export type Scope = "server" | "character";

export interface Activity {
  id: string;
  name: string;
  period: Period;
  scope: Scope;
  /** How many times it can be done per period; shown as pips when more than 1. */
  count: number;
  note?: string;
  disputed?: string;
  custom?: boolean;
}

export const ACTIVITIES: Activity[] = [
  // Daily
  {
    id: "duty", name: "Duty Missions", period: "daily", scope: "server", count: 5,
    note: "Journal (J) → Duty. 50,000 bound Kinah and 1,000 Abyss Points each. Reroll weak rewards.",
    disputed: "One guide says unused completions bank up to 20; others say they don't carry over.",
  },
  {
    id: "nightmare", name: "Nightmare", period: "daily", scope: "character", count: 2,
    note: "+2 attempts a day, banks up to 14. Only used up on a clear. Unlocks at level 45.",
  },
  {
    id: "shugo", name: "Shugo Festival", period: "daily", scope: "server", count: 3,
    note: "+3 keys a day, cap 12 (21 with Membership). Festival runs hourly on the hour.",
    disputed: "One guide says +2 keys a day with a cap of 14; that likely describes the Korean server.",
  },
  {
    id: "invasion", name: "Dimensional Invasion", period: "daily", scope: "server", count: 1,
    note: "+1 key a day, cap 7.",
  },
  {
    id: "odyle", name: "Spend Odyle Energy", period: "daily", scope: "character", count: 1,
    note: "+15 every 3 hours (120/day), cap 560 (840 with Membership). Reward cubes cost 40. Don't let it cap.",
  },
  {
    id: "conquest", name: "Expedition: Conquest rewards", period: "daily", scope: "character", count: 1,
    note: "+1 reward charge every 8 hours, cap 21. Spend them so they don't cap.",
  },
  {
    id: "transcendence", name: "Transcendence rewards", period: "daily", scope: "character", count: 1,
    note: "+1 reward charge every 12 hours, cap 14.",
  },
  {
    id: "supply-daily", name: "Supply Requests (daily)", period: "daily", scope: "server", count: 1,
    note: "Turn-ins give Abyss Points without PvP. Compare rewards against Market prices.",
  },
  {
    id: "rifts", name: "Spacetime Rifts", period: "daily", scope: "character", count: 1,
    note: "Optional. A rift opens every 3 hours; good for Abyss Points after the weekly limit.",
  },
  {
    id: "attendance", name: "Attendance reward", period: "daily", scope: "character", count: 1,
    disputed: "Only one guide lists it, and says it resets at server midnight rather than at the daily reset.",
  },

  // Weekly
  {
    id: "raids", name: "Sanctuary raids", period: "weekly", scope: "character", count: 4,
    note: "4 attempts per raid. Abyssal Forge: Ludra gives 1 rewarded final-boss kill; other raids give 2.",
  },
  {
    id: "ascension", name: "Ascension Trial", period: "weekly", scope: "character", count: 3,
    note: "Solo. Dying loses the reward.",
  },
  {
    id: "subjugation", name: "Subjugation", period: "weekly", scope: "character", count: 3,
    note: "3 tickets for the 4-player dungeons.",
  },
  {
    id: "fissure", name: "Unknown Fissure (daily dungeon)", period: "weekly", scope: "server", count: 14,
    note: "14 entries a week shared by the whole server, despite the name. Two a day keeps pace.",
  },
  {
    id: "exploration", name: "Expedition exploration rewards", period: "weekly", scope: "character", count: 1,
    note: "7 reward counts per Expedition dungeon.",
  },
  {
    id: "command", name: "Command Missions", period: "weekly", scope: "server", count: 12,
    note: "Buy scrolls from the capital's Command Merchant before reset; finish them any time after.",
  },
  {
    id: "abyss-command", name: "Abyss Command scrolls", period: "weekly", scope: "server", count: 1,
    note: "Separate weekly limit at the Abyss Merchant, up to 20 across tiers.",
  },
  {
    id: "battlefield", name: "Battlefield wins", period: "weekly", scope: "character", count: 3,
    note: "Up to 3 weekly victory rewards.",
  },
  {
    id: "abyss-time", name: "Abyss time (Lower, Middle, Upper)", period: "weekly", scope: "character", count: 3,
    note: "7 hours per layer (14 with Membership for Lower and Middle). One pip per layer used.",
  },
  {
    id: "growth", name: "Growth dungeons", period: "weekly", scope: "character", count: 2,
    note: "Silent Graveyard and Fafnite Amphitheater, 7 hours each. Time only counts while inside.",
  },
  {
    id: "substance-morph", name: "Odyle Energy crafts (Substance Morph)", period: "weekly", scope: "character", count: 4,
    note: "4 crafts per character, 16 per server. Worth doing on alts too.",
  },
  {
    id: "abyss-shop", name: "Abyss shop (stigma shards)", period: "weekly", scope: "character", count: 1,
    note: "Commonly called the best use of Abyss Points.",
  },
  {
    id: "corridor", name: "Abyss Corridor", period: "weekly", scope: "character", count: 1,
    note: "Only while your faction holds an Artifact; use it before the next Artifact Siege.",
  },
  {
    id: "supply-weekly", name: "Supply Requests (weekly)", period: "weekly", scope: "server", count: 1,
  },
  {
    id: "daeva-pass", name: "Daeva Pass weekly EXP", period: "weekly", scope: "character", count: 1,
    note: "Weekly cap of 50,000 (Season 1). Event passes have their own, smaller cap.",
  },
  {
    id: "season-missions", name: "Season Missions", period: "weekly", scope: "character", count: 1,
    note: "Weekly rewards pay out on points earned; points restart at reset.",
  },
];

export interface Region {
  id: string;
  label: string;
  /** Daily reset hour in UTC. */
  hourUtc: number;
  /** Weekly reset weekday in UTC, 0 = Sunday. */
  weekdayUtc: number;
}

export const REGIONS: Region[] = [
  { id: "global", label: "Global (NA / EU): 07:00 UTC, Wednesday", hourUtc: 7, weekdayUtc: 3 },
  { id: "kr", label: "Korea: 05:00 KST, Wednesday", hourUtc: 20, weekdayUtc: 2 },
  { id: "tw", label: "Taiwan: 05:00 GMT+8, Wednesday", hourUtc: 21, weekdayUtc: 2 },
];

const HOUR = 3_600_000;
const DAY = 24 * HOUR;

/** Start of the daily or weekly period containing `now`, in epoch ms. */
export function periodStart(period: Period, region: Region, now: number): number {
  const d = new Date(now);
  let start = Date.UTC(d.getUTCFullYear(), d.getUTCMonth(), d.getUTCDate(), region.hourUtc);
  if (start > now) start -= DAY;
  if (period === "weekly") start -= ((new Date(start).getUTCDay() - region.weekdayUtc + 7) % 7) * DAY;
  return start;
}

export function nextReset(period: Period, region: Region, now: number): number {
  return periodStart(period, region, now) + (period === "daily" ? DAY : 7 * DAY);
}
