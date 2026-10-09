import { api } from "../shared/api";
import type { CaptureStatus } from "../shared/types";

export function StatusBanner({ status, demo, onHelp }: { status: CaptureStatus; demo: boolean; onHelp: () => void }) {
  if (demo) return <div className="banner info">Demo mode: showing made-up fights. Turn it off in Settings.</div>;
  switch (status.state) {
    case "capturing":
      return null; // The sidebar pill says it; a banner is only for things that need you.
    case "waiting":
      return (
        <div className="banner info">
          Watching {status.adapters} network adapters. Waiting for AION 2 traffic. If you are already in game,
          change zones once so the meter can sync.
        </div>
      );
    case "npcapMissing":
      return (
        <div className="banner warn">
          Npcap is not installed, so the meter can't see game traffic.{" "}
          <button className="link" onClick={onHelp}>How to install it</button>{" "}
          <button className="link" onClick={() => api.retryCapture()}>Retry</button>
        </div>
      );
    case "error":
      return (
        <div className="banner warn">
          {status.message} <button className="link" onClick={() => api.retryCapture()}>Retry</button>
        </div>
      );
    case "off":
      return <div className="banner info">Live capture is off in this build.</div>;
  }
}

/** One-line capture state under the profile in the sidebar. */
export function StatusPill({ status, demo }: { status: CaptureStatus; demo: boolean }) {
  const [tone, text] = demo
    ? ["info", "Demo mode"]
    : status.state === "capturing"
      ? ["ok", `Capturing on ${status.adapter}`]
      : status.state === "waiting"
        ? ["info", "Waiting for game traffic"]
        : status.state === "off"
          ? ["info", "Capture off"]
          : ["warn", status.state === "npcapMissing" ? "Npcap missing" : "Capture error"];
  return (
    <div className={`pill ${tone}`} title={text}>
      <span className="dot" />
      <span className="pill-text">{text}</span>
    </div>
  );
}
