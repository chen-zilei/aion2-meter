import { useEffect, useRef, useState } from "react";
import { api } from "../shared/api";
import { DEFAULT_HOTKEYS, HOTKEY_ACTIONS, isModifierKey, Keys, keysText, modifiersOf, sameShortcut, shortcutFrom } from "../shared/hotkeys";
import { nameTablesApi, useNameTables } from "../shared/names";
import type { Hotkeys, Recordings, Settings } from "../shared/types";

export function SettingsPage({ settings, hotkeysFailed }: { settings: Settings; hotkeysFailed: (keyof Hotkeys)[] }) {
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
          hint={`The overlay ignores the mouse so clicks go to the game.${
            settings.hotkeys.lockOverlay ? ` ${keysText(settings.hotkeys.lockOverlay)} toggles it from in game.` : ""
          }`}
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
        <Toggle
          label="Party only"
          hint="Show only you and your party. Solo, that's just you."
          checked={settings.partyOnly}
          onChange={(v) => save({ partyOnly: v })}
        />
        <Toggle
          label="Only my fights"
          hint="Ignore fights nearby that you and your party aren't in, for the open world. Anyone helping on your targets still counts."
          checked={settings.onlyMyFights}
          onChange={(v) => save({ onlyMyFights: v })}
        />
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
      <ShortcutsCard hotkeys={settings.hotkeys} failed={hotkeysFailed} />
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
        <RecordPackets settings={settings} onChange={(v) => save({ recordPackets: v })} />
      </div>
    </section>
  );
}

function ShortcutsCard({ hotkeys, failed }: { hotkeys: Hotkeys; failed: (keyof Hotkeys)[] }) {
  const [recording, setRecording] = useState<keyof Hotkeys | null>(null);
  const [held, setHeld] = useState<string[]>([]);
  const [message, setMessage] = useState<{ id: keyof Hotkeys; text: string } | null>(null);
  // Read through a ref so the four-a-second updates don't restart recording.
  const current = useRef(hotkeys);
  current.current = hotkeys;

  const bind = (id: keyof Hotkeys, keys: string | null) =>
    api.setHotkey(id, keys).then(() => setMessage(null), (e) => setMessage({ id, text: String(e) }));

  useEffect(() => {
    if (!recording) return;
    const id = recording;
    setMessage(null);
    api.pauseHotkeys(true).catch(() => {});
    const finish = (keys: string) => {
      setRecording(null);
      bind(id, keys);
    };
    const onDown = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      const mods = modifiersOf(e);
      setHeld(mods);
      if (isModifierKey(e)) return;
      if (!mods.length && e.code === "Escape") return setRecording(null);
      if (!mods.length && (e.code === "Backspace" || e.code === "Delete")) return finish("");
      const made = shortcutFrom(e);
      if ("error" in made) return setMessage({ id, text: made.error });
      const other = HOTKEY_ACTIONS.find((a) => a.id !== id && sameShortcut(current.current[a.id], made.keys));
      if (other) {
        return setMessage({ id, text: `${keysText(made.keys)} is already used for ${other.label.toLowerCase()}. Press another, or Esc to cancel.` });
      }
      finish(made.keys);
    };
    const onUp = (e: KeyboardEvent) => {
      e.preventDefault();
      setHeld(modifiersOf(e));
    };
    const cancel = () => setRecording(null);
    window.addEventListener("keydown", onDown, true);
    window.addEventListener("keyup", onUp, true);
    window.addEventListener("blur", cancel);
    return () => {
      window.removeEventListener("keydown", onDown, true);
      window.removeEventListener("keyup", onUp, true);
      window.removeEventListener("blur", cancel);
      setHeld([]);
      api.pauseHotkeys(false).catch(() => {});
    };
  }, [recording]);

  const changed = HOTKEY_ACTIONS.some((a) => hotkeys[a.id] !== DEFAULT_HOTKEYS[a.id]);
  return (
    <div className="card form">
      <div className="card-head">
        <h2>Shortcuts</h2>
        {changed && (
          <button className="link" onClick={() => api.resetHotkeys().then(() => setMessage(null))}>
            Reset all
          </button>
        )}
      </div>
      <p className="dim small">
        These work while the game has focus, and the game doesn't see keys bound here. Click a shortcut and press the new
        keys. Esc cancels, Backspace clears it.
      </p>
      {HOTKEY_ACTIONS.map((a) => {
        const on = recording === a.id;
        return (
          <div key={a.id} className="row">
            <span>
              {a.label}
              {a.hint && <span className="dim small block">{a.hint}</span>}
              {failed.includes(a.id) && !on && (
                <span className="warn-text small block">Windows or another app is already using this shortcut. Pick another.</span>
              )}
              {message?.id === a.id && <span className="warn-text small block">{message.text}</span>}
            </span>
            <span className="hotkey-controls">
              {hotkeys[a.id] !== DEFAULT_HOTKEYS[a.id] && !on && (
                <button className="link" onClick={() => bind(a.id, null)}>
                  Reset
                </button>
              )}
              <button
                className={`hotkey ${on ? "recording" : ""}`}
                title={on ? "Press the new shortcut" : "Click to change"}
                onClick={() => setRecording(on ? null : a.id)}
                onBlur={() => on && setRecording(null)}
              >
                {on ? held.length ? <Keys keys={[...held, "…"].join("+")} /> : "Press keys…" : <Keys keys={hotkeys[a.id]} />}
              </button>
            </span>
          </div>
        );
      })}
    </div>
  );
}

function RecordPackets({ settings, onChange }: { settings: Settings; onChange: (v: boolean) => void }) {
  const [info, setInfo] = useState<Recordings | null>(null);
  useEffect(() => {
    const load = () => api.recordingsInfo().then(setInfo).catch(() => {});
    load();
    const timer = setInterval(load, 5000);
    return () => clearInterval(timer);
  }, [settings.recordPackets]);
  const gb = (n: number) => `${(n / 1024 ** 3).toFixed(n < 1024 ** 3 ? 2 : 1)} GB`;
  return (
    <>
      <Toggle
        label="Record raw packets"
        hint="Saves the game's traffic to disk, so a hit the meter missed or counted wrong can be decoded again after a fix. Nothing is sent anywhere."
        checked={settings.recordPackets}
        onChange={onChange}
      />
      {info && (settings.recordPackets || info.files > 0) && (
        <div className="row">
          <span className="dim small">
            {info.files} {info.files === 1 ? "file" : "files"}, {gb(info.bytes)} of {gb(info.maxBytes)} in <code>{info.folder}</code>.
            The oldest are deleted past that.
            {settings.recordPackets && !info.active && (
              <span className="block">Recording stopped because a file couldn't be written. Is the disk full? Turn it off and on to retry.</span>
            )}
          </span>
          <button onClick={() => api.openRecordingsFolder()}>Open folder</button>
        </div>
      )}
    </>
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
