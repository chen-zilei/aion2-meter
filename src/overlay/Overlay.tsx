import { api, useMeter } from "../shared/api";
import { Bars } from "../shared/Bars";
import { fmtCompact, fmtDuration } from "../shared/format";
import { OverlayTimers } from "../app/timers/OverlayTimers";

/** Rows the overlay shows before it only adds yours. */
const OVERLAY_ROWS = 8;

export function Overlay() {
  const update = useMeter();
  if (!update) return null;
  const { current: snap, settings } = update;
  const locked = settings.overlayLocked;
  const selfIdx = snap ? snap.actors.findIndex((a) => a.isSelf) : -1;

  return (
    <div className={`overlay ${locked ? "locked" : ""}`} style={{ ["--overlay-bg" as string]: `rgba(10, 13, 19, ${settings.overlayOpacity})` }}>
      <div className="overlay-head" data-tauri-drag-region>
        <span data-tauri-drag-region className="overlay-title">
          {snap ? `${snap.mainTarget || "Encounter"} · ${fmtDuration(snap.durationS)}` : "AION 2 Meter"}
        </span>
        <span data-tauri-drag-region className="num overlay-dps">{snap ? `${fmtCompact(snap.partyDps)}/s` : ""}</span>
        {!locked && (
          <span className="overlay-buttons">
            <button title="New encounter" onClick={() => api.resetEncounter()}>⟲</button>
            <button title="Lock (click-through). Ctrl+Shift+L unlocks." onClick={() => api.setSettings({ ...settings, overlayLocked: true })}>🔒</button>
          </span>
        )}
      </div>
      {snap && snap.actors.length > 0 ? (
        <div className="overlay-rows">
          <Bars actors={snap.actors.slice(0, OVERLAY_ROWS)} dense />
          {selfIdx >= OVERLAY_ROWS && (
            <Bars actors={[snap.actors[selfIdx]]} dense firstRank={selfIdx + 1} topDamage={snap.actors[0].damage} />
          )}
        </div>
      ) : (
        <div className="overlay-empty">Waiting for combat</div>
      )}
      <OverlayTimers />
    </div>
  );
}
