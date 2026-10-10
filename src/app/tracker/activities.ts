/**
 * AION 2 daily and weekly activities, as listed by community guides in October 2026.
 * Counts are the Global (NA/EU) values. Where sources disagree, `disputed` says how.
 */

export type Period = "daily" | "weekly";
/**
 * "server" and "account": shared by every character (on the server, or on the account).
 * "character": each character has its own.
 */
export type Scope = "server" | "account" | "character";

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
  /** NPC shop purchase with a weekly limit; listed in its own section. */
  shop?: string;
}

export const ACTIVITIES: Activity[] = [
  // Daily
  {
    id: "duty", name: "Duty Missions", period: "daily", scope: "server", count: 5,
    note: "Journal (J) → Duty. 50,000 bound Kinah and 1,000 Abyss Points each. Reroll weak rewards.",
    disputed: "One guide says unused completions bank up to 20; others say they don't carry over.",
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
    id: "fissure", name: "Daily Dungeon: Unknown Fissure", period: "weekly", scope: "server", count: 14,
    note: "A solo wave dungeon (Daeva Bio-Research Base), not an Expedition. Unlocks at level 30 and rewards Enhance Stones. 14 entries a week shared by the whole server, despite the name, so two a day keeps pace.",
  },
  {
    id: "exploration", name: "Expedition: Exploration rewards", period: "weekly", scope: "character", count: 1,
    note: "Exploration is the easier mode of each Expedition dungeon (Epic gear). 7 reward counts per dungeon, refilled on Wednesday.",
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

  // Weekly NPC shop purchases (reset with the weekly reset)
  {
    id: "abyss-command", name: "Command scrolls: Common", period: "weekly", scope: "server", count: 5,
    shop: "Command Merchant",
    note: "5 of each rarity, 20 a week shared by the server (checked in game). Common ones are 15,000 Kinah each.",
  },
  {
    id: "cmd-veteran", name: "Command scrolls: Rare", period: "weekly", scope: "server", count: 5,
    shop: "Command Merchant", note: "Veteran tier, 37,500 Kinah each.",
  },
  {
    id: "cmd-elite", name: "Command scrolls: Epic", period: "weekly", scope: "server", count: 5,
    shop: "Command Merchant", note: "Elite tier, 75,000 Kinah each.",
  },
  {
    id: "cmd-special", name: "Command scrolls: Unique", period: "weekly", scope: "server", count: 5,
    shop: "Command Merchant", note: "Special Mission tier, 150,000 Kinah each.",
  },
  {
    id: "ap-manastone", name: "Lesser Abyssal Manastone", period: "weekly", scope: "character", count: 50,
    shop: "Abyss Trade Shop", note: "1,000 Abyss Points each.",
  },
  {
    id: "ap-soulstone", name: "Lesser Abyssal Soulstone", period: "weekly", scope: "character", count: 50,
    shop: "Abyss Trade Shop", note: "2,000 Abyss Points each.",
  },
  {
    id: "ap-medal", name: "Silver Medal of Merit", period: "weekly", scope: "character", count: 10,
    shop: "Abyss Trade Shop", note: "10,000 Abyss Points each.",
  },
  {
    id: "ap-potential", name: "Potential Stone: Abyss (Unique)", period: "weekly", scope: "account", count: 8,
    shop: "Abyss Trade Shop", note: "Price climbs as you buy: 2 at 25,000 AP, then 50,000, 75,000 and 100,000 AP, up to 8.",
    disputed: "The tiers are read from metabot.gg's limits of 2 / 4 / 6 / 8 per price; no guide spells out how they stack.",
  },
  {
    id: "nm-codex", name: "Soul Codex ×5", period: "weekly", scope: "character", count: 5,
    shop: "Nightmare Trade Shop", note: "400 Phantasmal Fragments each.",
  },
  {
    id: "nm-codex-reset", name: "Soul Codex: Reset", period: "weekly", scope: "character", count: 3,
    shop: "Nightmare Trade Shop", note: "8,000 Phantasmal Fragments each.",
  },
  {
    id: "wb-odyle", name: "Odyle Energy (Bound)", period: "weekly", scope: "character", count: 20,
    shop: "Wind Breeze Merchant (Membership)", note: "100,000 Kinah each. Needs an active Membership.",
    disputed: "One guide says alts can only buy 4 a week.",
  },
  {
    id: "wb-fissure", name: "Unknown Fissure tickets", period: "weekly", scope: "character", count: 21,
    shop: "Wind Breeze Merchant (Membership)", disputed: "Only one guide lists this, and it gives no price.",
  },
  {
    id: "wb-soul-crystal", name: "Soul Crystals", period: "weekly", scope: "character", count: 1,
    shop: "Wind Breeze Merchant (Membership)", disputed: "Only one guide lists this, with no amount or price.",
  },
];

/**
 * Things that refill on a timer and stop refilling at a cap. Nothing is lost at the daily or weekly reset;
 * what's lost is any refill that arrives while you're full.
 */
export interface Stack {
  id: string;
  name: string;
  scope: Scope;
  /** How much each refill adds. */
  gain: number;
  /** "daily": at each daily reset. A number: every that many hours. */
  every: "daily" | number;
  cap: number;
  /** Cap with an active Membership, where it differs. */
  memberCap?: number;
  unit: string;
  /** What one use costs, for the spend button. */
  spend: number;
  note?: string;
  disputed?: string;
}

export const STACKS: Stack[] = [
  {
    id: "nightmare", name: "Nightmare", scope: "character", gain: 2, every: "daily", cap: 14, unit: "attempts", spend: 1,
    note: "Only used up on a clear. Unlocks at level 45.",
  },
  {
    id: "shugo", name: "Shugo Festival keys", scope: "server", gain: 3, every: "daily", cap: 12, memberCap: 21, unit: "keys", spend: 1,
    note: "Festival runs hourly on the hour.",
    disputed: "One guide says +2 keys a day with a cap of 14; that likely describes the Korean server.",
  },
  {
    id: "invasion", name: "Dimensional Invasion keys", scope: "server", gain: 1, every: "daily", cap: 7, unit: "keys", spend: 1,
    note: "Invasions open every hour at :30 (timer icon by the minimap; see the Timers tab). You can join without a key; the key is only spent when you flip reward cards at the end.",
  },
  {
    id: "odyle", name: "Odyle Energy", scope: "character", gain: 15, every: 3, cap: 560, memberCap: 840, unit: "energy", spend: 40,
    note: "Reward cubes cost 40. Spend it on Expeditions and Transcendence.",
  },
  {
    id: "conquest", name: "Expedition: Conquest rewards", scope: "character", gain: 1, every: 8, cap: 21, unit: "charges", spend: 1,
    note: "Conquest is the harder mode of the same Expedition dungeons and the one that drops Unique gear. Charges are shared by all Conquest dungeons.",
  },
  {
    id: "transcendence", name: "Transcendence rewards", scope: "character", gain: 1, every: 12, cap: 14, unit: "charges", spend: 1,
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
  { id: "asia", label: "Asia (Global client): 07:00 UTC, Wednesday", hourUtc: 7, weekdayUtc: 3 },
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

/**
 * Where a timed refill falls within its cycle, in ms after a multiple of the interval since the epoch.
 * By default refills line up with the daily reset (e.g. Odyle at 07:00, 10:00, 13:00 UTC on Global),
 * which matches what players see in game; `phase` overrides it when someone's timer differs.
 */
export function defaultPhase(stack: Stack, region: Region): number {
  if (stack.every === "daily") return 0;
  const everyMs = stack.every * HOUR;
  return (region.hourUtc * HOUR) % everyMs;
}

/** Phase that puts the next refill `ms` from `now`. */
export function phaseFromNext(stack: Stack, now: number, ms: number): number {
  const everyMs = stack.every === "daily" ? DAY : stack.every * HOUR;
  return (((now + ms) % everyMs) + everyMs) % everyMs;
}

/** What a stack holds now, from the amount entered at `at`; and when the next refill and the cap arrive. */
export function projectStack(
  stack: Stack,
  cap: number,
  entered: { value: number; at: number },
  region: Region,
  now: number,
  phase = defaultPhase(stack, region),
) {
  // `now` can trail the moment the amount was typed, so never count a negative number of refills.
  const t = Math.max(now, entered.at);
  const everyMs = stack.every === "daily" ? DAY : stack.every * HOUR;
  const tick = (x: number) => Math.floor((x - phase) / everyMs);
  const refills =
    stack.every === "daily"
      ? Math.max(0, Math.round((periodStart("daily", region, t) - periodStart("daily", region, entered.at)) / DAY))
      : tick(t) - tick(entered.at);
  // Items can push some stacks past the cap; refills just stop until it drops below.
  const start = Math.max(0, entered.value);
  const value = start >= cap ? start : Math.min(cap, start + refills * stack.gain);
  const full = value >= cap;
  const nextAt = stack.every === "daily" ? nextReset("daily", region, t) : (tick(t) + 1) * everyMs + phase;
  const needed = Math.ceil((cap - value) / stack.gain);
  const fullAt = full ? now : nextAt + (needed - 1) * everyMs;
  return { value, full, nextAt, fullAt };
}
