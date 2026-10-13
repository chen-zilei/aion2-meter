import { REGIONS, nextReset } from "../tracker/activities";
import { ARTIFACT_SIEGE, EVENTS, nextStart } from "./schedule";
import { countdown, useNow, useSharedRegion } from "./clock";

/** Drops the minutes once a timer is a day or more out, so four of them fit the overlay's width. */
const short = (ms: number) => (ms >= 86_400_000 ? countdown(ms).replace(/ \d+m$/, "") : countdown(ms));

/** One-line countdowns for the overlay: next rift, Artifact Siege, daily reset and weekly reset. */
export function OverlayTimers() {
  const now = useNow(1000);
  const region = useSharedRegion();
  const tracked = REGIONS.find((r) => r.id === region) ?? REGIONS[0];
  const rift = EVENTS.find((e) => e.id.startsWith("rift-") && e.regions.includes(region));
  const siege = ARTIFACT_SIEGE.regions.includes(region) ? ARTIFACT_SIEGE : undefined;

  const items = [
    rift && { label: "Rift", at: nextStart(rift, region, now) },
    siege && { label: "Siege", at: nextStart(siege, region, now) },
    { label: "Daily", at: nextReset("daily", tracked, now) },
    { label: "Weekly", at: nextReset("weekly", tracked, now) },
  ].filter((x) => !!x);

  return (
    <div className="overlay-timers num" data-tauri-drag-region>
      {items.map((i) => (
        <span key={i.label} data-tauri-drag-region title={new Date(i.at).toLocaleString()}>
          <span className="dim">{i.label}</span> {short(i.at - now)}
        </span>
      ))}
    </div>
  );
}
