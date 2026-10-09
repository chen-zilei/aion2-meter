import { useEffect, useState } from "react";
import { api } from "../shared/api";
import { nameTablesApi, useNameTables } from "../shared/names";
import type { Settings } from "../shared/types";

export function SettingsPage({ settings }: { settings: Settings }) {
  const [port, setPort] = useState(settings.gamePorts.join(", "));
  useEffect(() => setPort(settings.gamePorts.join(", ")), [settings.gamePorts]);

  const save = (patch: Partial<Settings>) => api.setSettings({ ...settings, ...patch });

  return (
    <section>
      <header className="page-head"><h1>Settings</h1></header>
      <div className="card form">
        <h2>Overlay</h2>
        <Toggle label="Show overlay" checked={settings.overlayVisible} onChange={(v) => save({ overlayVisible: v })} />
        <Toggle
          label="Lock overlay (click-through)"
          hint="The overlay ignores the mouse so clicks go to the game. Ctrl+Shift+L toggles it from in game."
          checked={settings.overlayLocked}
          onChange={(v) => save({ overlayLocked: v })}
        />
        <label className="row">
          <span>Background opacity</span>
          <input
            type="range"
            min={0.2}
            max={1}
            step={0.05}
            value={settings.overlayOpacity}
            onChange={(e) => save({ overlayOpacity: Number(e.target.value) })}
          />
        </label>
        <p className="dim small">To use a second monitor, unlock the overlay and drag it there by its title strip.</p>
      </div>
      <div className="card form">
        <h2>Meter</h2>
        <Toggle label="Players only" hint="Hide monsters and unidentified entities." checked={settings.playersOnly} onChange={(v) => save({ playersOnly: v })} />
        <label className="row">
          <span>End encounter after (seconds without damage)</span>
          <input
            type="number"
            min={2}
            max={120}
            value={settings.idleTimeoutS}
            onChange={(e) => save({ idleTimeoutS: Math.max(2, Number(e.target.value) || 8) })}
          />
        </label>
        <Toggle label="Demo mode" hint="Made-up fights, for trying the app without the game." checked={settings.demo} onChange={(v) => save({ demo: v })} />
      </div>
      <NameTablesCard />
      <div className="card form">
        <h2>Advanced</h2>
        <label className="row">
          <span>Game server port(s)</span>
          <input
            value={port}
            onChange={(e) => setPort(e.target.value)}
            onBlur={() => {
              const ports = port.split(/[ ,]+/).map(Number).filter((p) => p > 0 && p < 65536);
              if (ports.length) save({ gamePorts: ports });
            }}
          />
        </label>
        <p className="dim small">Only change this if a game patch moves the server port.</p>
      </div>
    </section>
  );
}

function NameTablesCard() {
  const tables = useNameTables();
  if (!tables) return null;
  const loaded = `${tables.skills.toLocaleString()} skills and ${tables.npcs.toLocaleString()} monsters loaded.`;
  return (
    <div className="card form">
      <h2>Skill and monster names</h2>
      <p className="dim small">
        The game sends skills and monsters as numbers. These tables turn them into names. They are downloaded from{" "}
        <code>{tables.source}</code> (GPL-3.0) and saved in{" "}
        <code>{tables.folder}</code>.
      </p>
      <div className="row">
        <span>
          {tables.state === "downloading" ? "Downloading…" : loaded}
          {tables.state === "failed" && <span className="dim small block">Download failed: {tables.message}</span>}
        </span>
        <button disabled={tables.state === "downloading"} onClick={() => nameTablesApi.download()}>
          {tables.skills ? "Download again" : "Download"}
        </button>
      </div>
    </div>
  );
}

function Toggle({ label, hint, checked, onChange }: { label: string; hint?: string; checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <label className="row">
      <span>
        {label}
        {hint && <span className="dim small block">{hint}</span>}
      </span>
      <input type="checkbox" className="switch" checked={checked} onChange={(e) => onChange(e.target.checked)} />
    </label>
  );
}
