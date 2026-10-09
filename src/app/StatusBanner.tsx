import { api } from "../shared/api";
import type { CaptureStatus } from "../shared/types";

export function StatusBanner({ status, demo, onHelp }: { status: CaptureStatus; demo: boolean; onHelp: () => void }) {
  if (demo) return <div className="banner info">Demo mode: showing made-up fights. Turn it off in Settings.</div>;
  switch (status.state) {
    case "capturing":
      return <div className="banner ok">Capturing game traffic on {status.adapter}</div>;
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
