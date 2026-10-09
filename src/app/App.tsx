import { useState } from "react";
import { useMeter } from "../shared/api";
import { StatusBanner, StatusPill } from "./StatusBanner";
import { UpdateBanner } from "./UpdateBanner";
import { LivePage } from "./LivePage";
import { HistoryPage } from "./HistoryPage";
import { SettingsPage } from "./SettingsPage";
import { SetupPage } from "./SetupPage";
import { TrackerPage } from "./tracker/TrackerPage";
import { TimersPage } from "./timers/TimersPage";
import { Icon } from "./icons";

const GROUPS = [
  { label: "Combat", pages: [{ id: "live", label: "Live" }, { id: "history", label: "History" }] },
  { label: "Planner", pages: [{ id: "tracker", label: "Dailies & weeklies" }, { id: "timers", label: "Timers" }] },
  { label: "App", pages: [{ id: "settings", label: "Settings" }, { id: "setup", label: "Setup & help" }] },
] as const;
type PageId = (typeof GROUPS)[number]["pages"][number]["id"];

export function App() {
  const update = useMeter();
  const [page, setPage] = useState<PageId>("live");
  const self = update?.selfName;

  return (
    <div className="app">
      <nav className="sidebar" aria-label="Pages">
        <div className="profile">
          <span className="avatar">{self ? self[0].toUpperCase() : "A2"}</span>
          <div className="profile-text">
            <div className="profile-name">{self ?? "AION 2 Meter"}</div>
            <div className="dim small">{self ? "AION 2 Meter" : "Not identified yet"}</div>
          </div>
        </div>
        {update && <StatusPill status={update.status} demo={update.settings.demo} />}
        {GROUPS.map((g) => (
          <div key={g.label} className="nav-group">
            <div className="nav-label">{g.label}</div>
            {g.pages.map((p) => (
              <button key={p.id} className={`nav ${page === p.id ? "active" : ""}`} onClick={() => setPage(p.id)}>
                <Icon name={p.id} />
                <span className="nav-text">{p.label}</span>
                {p.id === "history" && update?.historyLen ? <span className="badge">{update.historyLen}</span> : null}
              </button>
            ))}
          </div>
        ))}
        <div className="spacer" />
        <details className="hotkeys">
          <summary>
            <Icon name="keyboard" />
            Shortcuts
          </summary>
          <div><kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>L</kbd> lock overlay</div>
          <div><kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>O</kbd> show/hide overlay</div>
          <div><kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>R</kbd> new encounter</div>
        </details>
      </nav>
      <main className="content">
        <UpdateBanner />
        {update && <StatusBanner status={update.status} demo={update.settings.demo} onHelp={() => setPage("setup")} />}
        {page === "live" && <LivePage update={update} />}
        {page === "history" && <HistoryPage historyLen={update?.historyLen ?? 0} />}
        {page === "tracker" && <TrackerPage />}
        {page === "timers" && <TimersPage />}
        {page === "settings" && update && <SettingsPage settings={update.settings} />}
        {page === "setup" && <SetupPage />}
      </main>
    </div>
  );
}
