import { api, useMeter } from "../shared/api";
import { Bars } from "../shared/Bars";
import { fmtCompact, fmtDuration } from "../shared/format";

export function Overlay() {
  const update = useMeter();
  if (!update) return null;
  const { current: snap, settings } = update;
  const locked = settings.overlayLocked;

  return (
    <div className={`overlay ${locked ? "locked" : ""}`} style={{ ["--overlay-bg" as string]: `rgba(14, 17, 24, ${settings.overlayOpacity})` }}>
      <div className="overlay-head" data-tauri-drag-region>
        <span data-tauri-drag-region className="overlay-title">
          {snap ? `${snap.mainTarget || "Encounter"} · ${fmtDuration(snap.durationS)}` : "AION 2 Meter"}
        </span>
        <span data-tauri-drag-region className="num">{snap ? `${fmtCompact(snap.partyDps)}/s` : ""}</span>
        {!locked && (
          <span className="overlay-buttons">
            <button title="New encounter" onClick={() => api.resetEncounter()}>⟲</button>
            <button title="Lock (click-through). Ctrl+Shift+L unlocks." onClick={() => api.setSettings({ ...settings, overlayLocked: true })}>🔒</button>
          </span>
        )}
      </div>
      {snap && snap.actors.length > 0 ? (
        <Bars actors={snap.actors.slice(0, 8)} dense />
      ) : (
        <div className="overlay-empty">Waiting for combat</div>
      )}
    </div>
  );
}
