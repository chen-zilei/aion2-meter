import { REGIONS, nextReset } from "../tracker/activities";
import { EVENTS, nextStart } from "./schedule";
import { countdown, useNow, useSharedRegion } from "./clock";

/** One-line countdowns for the overlay: next rift, daily reset and weekly reset. */
export function OverlayTimers() {
  const now = useNow(1000);
  const region = useSharedRegion();
  const tracked = REGIONS.find((r) => r.id === region) ?? REGIONS[0];
  const rift = EVENTS.find((e) => e.id.startsWith("rift-") && e.regions.includes(region));

  const items = [
    rift && { label: "Rift", at: nextStart(rift, region, now) },
    { label: "Daily", at: nextReset("daily", tracked, now) },
    { label: "Weekly", at: nextReset("weekly", tracked, now) },
  ].filter((x) => !!x);

  return (
    <div className="overlay-timers num" data-tauri-drag-region>
      {items.map((i) => (
        <span key={i.label} data-tauri-drag-region title={new Date(i.at).toLocaleString()}>
          <span className="dim">{i.label}</span> {countdown(i.at - now)}
        </span>
      ))}
    </div>
  );
}
