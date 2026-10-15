import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import type { Hotkeys, Recordings, Settings, Snapshot, Update } from "./types";

export const api = {
  getUpdate: () => invoke<Update>("get_update"),
  getHistory: () => invoke<Snapshot[]>("get_history"),
  resetEncounter: () => invoke<void>("reset_encounter"),
  setSettings: (settings: Settings) => invoke<Settings>("set_settings", { settings }),
  /** `keys` null restores the default; "" unbinds. Rejects with a message when it clashes or is taken. */
  setHotkey: (action: keyof Hotkeys, keys: string | null) => invoke<void>("set_hotkey", { action, keys }),
  resetHotkeys: () => invoke<void>("reset_hotkeys"),
  /** Frees the shortcuts while one is being recorded, so pressing a current one reaches the page. */
  pauseHotkeys: (paused: boolean) => invoke<void>("pause_hotkeys", { paused }),
  retryCapture: () => invoke<void>("retry_capture"),
  recordingsInfo: () => invoke<Recordings>("recordings_info"),
  openRecordingsFolder: () => invoke<void>("open_recordings_folder"),
};

/** Latest state from the backend; refreshed four times a second. */
export function useMeter(): Update | null {
  const [update, setUpdate] = useState<Update | null>(null);
  useEffect(() => {
    api.getUpdate().then(setUpdate).catch(() => {});
    const off = listen<Update>("meter://update", (e) => setUpdate(e.payload));
    return () => {
      off.then((f) => f());
    };
  }, []);
  return update;
}
