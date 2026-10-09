import { useEffect, useState } from "react";
import { REGIONS, nextReset } from "../tracker/activities";
import { EVENTS, SERVER_UTC_OFFSET, lastStart, nextStart, type RegionId } from "./schedule";
import "./timers.css";

/** The region is shared with the Dailies & weeklies tab, which keeps it in its saved state. */
const TRACKER_KEY = "aion2-meter.tracker";

function loadRegion(): RegionId {
  try {
    const region = JSON.parse(localStorage.getItem(TRACKER_KEY) ?? "{}").region;
    return region in SERVER_UTC_OFFSET ? region : "global";
  } catch {
    return "global";
  }
}

function saveRegion(region: RegionId) {
  try {
    const saved = JSON.parse(localStorage.getItem(TRACKER_KEY) ?? "{}");
    localStorage.setItem(TRACKER_KEY, JSON.stringify({ ...saved, region }));
  } catch {
    // Storage unavailable: the choice lasts for this session only.
  }
}

function useNow(intervalMs: number) {
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), intervalMs);
    return () => clearInterval(id);
  }, [intervalMs]);
  return now;
}

const pad = (n: number) => String(n).padStart(2, "0");

function countdown(ms: number) {
  const s = Math.max(0, Math.floor(ms / 1000));
  const d = Math.floor(s / 86400), h = Math.floor((s % 86400) / 3600), m = Math.floor((s % 3600) / 60);
  return d ? `${d}d ${h}h ${pad(m)}m` : `${h}:${pad(m)}:${pad(s % 60)}`;
}

const localTime = (t: number, withDay: boolean) =>
  new Date(t).toLocaleString(undefined, { weekday: withDay ? "short" : undefined, hour: "2-digit", minute: "2-digit" });

function serverClock(now: number, region: RegionId) {
  const offset = SERVER_UTC_OFFSET[region];
  const d = new Date(now + offset * 3_600_000);
  return `${pad(d.getUTCHours())}:${pad(d.getUTCMinutes())} (UTC+${offset})`;
}

interface Row {
  id: string;
  name: string;
  at: number;
  openUntil?: number;
  note?: string;
  unconfirmed?: string;
}

export function TimersPage() {
  const now = useNow(1000);
  const [region, setRegion] = useState<RegionId>(loadRegion);
  const zone = Intl.DateTimeFormat().resolvedOptions().timeZone;
  const tracked = REGIONS.find((r) => r.id === region) ?? REGIONS[0];

  const resets: Row[] = [
    { id: "daily", name: "Daily reset", at: nextReset("daily", tracked, now) },
    { id: "weekly", name: "Weekly reset", at: nextReset("weekly", tracked, now) },
  ];
  const events: Row[] = EVENTS.filter((e) => e.regions.includes(region))
    .map((e) => {
      const last = lastStart(e, region, now);
      const openUntil = e.openMin && now < last + e.openMin * 60_000 ? last + e.openMin * 60_000 : undefined;
      return { id: e.id, name: e.name, at: nextStart(e, region, now), openUntil, note: e.note, unconfirmed: e.unconfirmed };
    })
    .sort((a, b) => a.at - b.at);

  const pick = (id: RegionId) => {
    setRegion(id);
    saveRegion(id);
  };

  return (
    <div className="timers">
      <div className="page-head">
        <div>
          <div>Times in your time zone ({zone})</div>
          <div className="dim small">Server time now: {serverClock(now, region)}</div>
        </div>
        <select value={region} onChange={(e) => pick(e.target.value as RegionId)} title="Server region">
          {REGIONS.map((r) => <option key={r.id} value={r.id}>{r.label}</option>)}
        </select>
      </div>

      <div className="card">
        <h2>Resets</h2>
        <TimerList rows={resets} now={now} />
      </div>
      <div className="card">
        <h2>Coming up</h2>
        <TimerList rows={events} now={now} />
      </div>
      <p className="dim small">
        From community guides (aion2hub.com, metabot.gg, expcarry.com), October 2026. Patches can move these; the in-game
        countdown wins if they disagree. Times marked unconfirmed come from a single source.
      </p>
    </div>
  );
}

function TimerList({ rows, now }: { rows: Row[]; now: number }) {
  return (
    <ul className="timer-list">
      {rows.map((r) => (
        <li key={r.id} className={r.openUntil ? "open" : ""}>
          <div className="timer-text">
            <div>
              {r.name}
              {r.unconfirmed && <span className="tag warn" title={r.unconfirmed}>unconfirmed</span>}
            </div>
            {r.note && <div className="dim small">{r.note}</div>}
            {r.unconfirmed && <div className="disputed small">{r.unconfirmed}</div>}
          </div>
          <div className="timer-when">
            {r.openUntil ? (
              <div className="timer-count num open-now">Open, closes in {countdown(r.openUntil - now)}</div>
            ) : (
              <div className="timer-count num">{countdown(r.at - now)}</div>
            )}
            <div className="dim small num">{localTime(r.at, r.at - now > 86_400_000 / 2)}</div>
          </div>
        </li>
      ))}
    </ul>
  );
}
