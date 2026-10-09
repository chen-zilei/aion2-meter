import { useEffect, useState } from "react";
import { SERVER_UTC_OFFSET, type RegionId } from "./schedule";

/** The region is shared with the Dailies & weeklies tab, which keeps it in its saved state. */
const TRACKER_KEY = "aion2-meter.tracker";

export function loadRegion(): RegionId {
  try {
    const region = JSON.parse(localStorage.getItem(TRACKER_KEY) ?? "{}").region;
    return region in SERVER_UTC_OFFSET ? region : "global";
  } catch {
    return "global";
  }
}

export function saveRegion(region: RegionId) {
  try {
    const saved = JSON.parse(localStorage.getItem(TRACKER_KEY) ?? "{}");
    localStorage.setItem(TRACKER_KEY, JSON.stringify({ ...saved, region }));
  } catch {
    // Storage unavailable: the choice lasts for this session only.
  }
}

export function useNow(intervalMs: number) {
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), intervalMs);
    return () => clearInterval(id);
  }, [intervalMs]);
  return now;
}

export const pad = (n: number) => String(n).padStart(2, "0");

export function countdown(ms: number) {
  const s = Math.max(0, Math.floor(ms / 1000));
  const d = Math.floor(s / 86400), h = Math.floor((s % 86400) / 3600), m = Math.floor((s % 3600) / 60);
  return d ? `${d}d ${h}h ${pad(m)}m` : `${h}:${pad(m)}:${pad(s % 60)}`;
}

/** Region shown in another window, kept in step when the app window changes it. */
export function useSharedRegion(): RegionId {
  const [region, setRegion] = useState<RegionId>(loadRegion);
  useEffect(() => {
    const onStorage = () => setRegion(loadRegion());
    window.addEventListener("storage", onStorage);
    return () => window.removeEventListener("storage", onStorage);
  }, []);
  return region;
}
