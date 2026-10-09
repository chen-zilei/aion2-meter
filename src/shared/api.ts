import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import type { Settings, Snapshot, Update } from "./types";

export const api = {
  getUpdate: () => invoke<Update>("get_update"),
  getHistory: () => invoke<Snapshot[]>("get_history"),
  resetEncounter: () => invoke<void>("reset_encounter"),
  setSettings: (settings: Settings) => invoke<Settings>("set_settings", { settings }),
  retryCapture: () => invoke<void>("retry_capture"),
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
