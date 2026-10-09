import { useState } from "react";
import { useMeter } from "../shared/api";
import { StatusBanner } from "./StatusBanner";
import { LivePage } from "./LivePage";
import { HistoryPage } from "./HistoryPage";
import { SettingsPage } from "./SettingsPage";
import { SetupPage } from "./SetupPage";
import { TrackerPage } from "./tracker/TrackerPage";
import { TimersPage } from "./timers/TimersPage";

const PAGES = [
  { id: "live", label: "Live" },
  { id: "history", label: "History" },
  { id: "tracker", label: "Dailies & weeklies" },
  { id: "timers", label: "Timers" },
  { id: "settings", label: "Settings" },
  { id: "setup", label: "Setup & help" },
] as const;
type PageId = (typeof PAGES)[number]["id"];

export function App() {
  const update = useMeter();
  const [page, setPage] = useState<PageId>("live");

  return (
    <div className="app">
      <nav className="sidebar">
        <div className="brand">
          <span className="logo">A2</span>
          <div>
            <div className="brand-name">AION 2 Meter</div>
            <div className="dim small">{update?.selfName ?? "not identified yet"}</div>
          </div>
        </div>
        {PAGES.map((p) => (
          <button key={p.id} className={`nav ${page === p.id ? "active" : ""}`} onClick={() => setPage(p.id)}>
            {p.label}
            {p.id === "history" && update?.historyLen ? <span className="badge">{update.historyLen}</span> : null}
          </button>
        ))}
        <div className="spacer" />
        <div className="dim small hotkeys">
          <div><kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>L</kbd> lock overlay</div>
          <div><kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>O</kbd> show/hide overlay</div>
          <div><kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>R</kbd> new encounter</div>
        </div>
      </nav>
      <main className="content">
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
