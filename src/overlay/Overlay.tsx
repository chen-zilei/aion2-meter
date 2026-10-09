import { useState } from "react";
import { api, useMeter } from "../shared/api";
import { Bars } from "../shared/Bars";
import { fmtCompact, fmtDuration } from "../shared/format";
import { OverlayTimers } from "../app/timers/OverlayTimers";
import { METRICS, type Metric, metricTotal, metricValue, ranked } from "../shared/metrics";
import type { Update } from "../shared/types";

function savedMetric(): Metric {
  try {
    const m = localStorage.getItem("overlay-metric");
    return METRICS.some((x) => x.id === m) ? (m as Metric) : "damage";
  } catch {
    return "damage";
  }
}

/** Rows the overlay shows before it only adds yours. */
const OVERLAY_ROWS = 8;

export function Overlay() {
  const update = useMeter();
  const [metric, setMetric] = useState<Metric>(savedMetric);
  if (!update) return null;
  const { current: snap, settings } = update;
  const locked = settings.overlayLocked;
  const rows = snap ? ranked(snap.actors, metric) : [];
  const total = metricTotal(rows, metric);
  const selfIdx = rows.findIndex((a) => a.isSelf);
  const info = METRICS.find((m) => m.id === metric)!;
  const cycle = () => {
    const next = METRICS[(METRICS.indexOf(info) + 1) % METRICS.length].id;
    setMetric(next);
    try {
      localStorage.setItem("overlay-metric", next);
    } catch {
      // Not remembered; it still switches.
    }
  };
  const headline = !snap
    ? ""
    : metric === "damage"
      ? `${fmtCompact(snap.partyDps)}/s`
      : metric === "healing"
        ? `${fmtCompact(total / snap.durationS)}/s`
        : fmtCompact(total);

  return (
    <div className={`overlay ${locked ? "locked" : ""}`} style={{ ["--overlay-bg" as string]: `rgba(10, 13, 19, ${settings.overlayOpacity})` }}>
      <div className="overlay-head" data-tauri-drag-region>
        <span data-tauri-drag-region className="overlay-title">
          {snap ? `${snap.mainTarget || "Encounter"} · ${fmtDuration(snap.durationS)}` : "AION 2 Meter"}
        </span>
        <Ping update={update} />
        <span data-tauri-drag-region className="num overlay-dps">{headline}</span>
        {!locked && (
          <span className="overlay-buttons">
            <button title="Switch between damage, healing and damage taken" onClick={cycle}>{info.short}</button>
            <button title="New encounter" onClick={() => api.resetEncounter()}>⟲</button>
            <button title="Lock (click-through). Ctrl+Shift+L unlocks." onClick={() => api.setSettings({ ...settings, overlayLocked: true })}>🔒</button>
          </span>
        )}
      </div>
      {rows.length > 0 ? (
        <div className="overlay-rows">
          <Bars actors={rows.slice(0, OVERLAY_ROWS)} dense metric={metric} total={total} />
          {selfIdx >= OVERLAY_ROWS && (
            <Bars actors={[rows[selfIdx]]} dense firstRank={selfIdx + 1} metric={metric} total={total} topDamage={metricValue(rows[0], metric)} />
          )}
        </div>
      ) : (
        <div className="overlay-empty">{snap && metric !== "damage" ? `No ${info.label.toLowerCase()} yet` : "Waiting for combat"}</div>
      )}
      <OverlayTimers />
    </div>
  );
}

/** Ping to the game server. Shown while capturing (or in the demo); a dash when there is no reading yet. */
function Ping({ update }: { update: Update }) {
  const { pingMs, status, settings } = update;
  if (pingMs == null && !(status.state === "capturing" && !settings.demo)) return null;
  const level = pingMs == null ? "none" : pingMs < 80 ? "good" : pingMs < 150 ? "fair" : "bad";
  return (
    <span data-tauri-drag-region className={`num overlay-ping ${level}`} title="Ping to the game server, timed from the game's own traffic">
      {pingMs == null ? "– ms" : `${pingMs} ms`}
    </span>
  );
}
