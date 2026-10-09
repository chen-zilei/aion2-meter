import { invoke } from "@tauri-apps/api/core";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { useEffect, useState } from "react";

const DISMISSED_KEY = "aion2meter.dismissedRelease";

/** Checks for a signed update once on launch and offers to install it, until the user dismisses that version. */
export function UpdateBanner() {
  const [update, setUpdate] = useState<Update | null>(null);
  const [progress, setProgress] = useState<string | null>(null);

  useEffect(() => {
    check()
      .then((u) => {
        if (u && localStorage.getItem(DISMISSED_KEY) !== u.version) setUpdate(u);
      })
      .catch(() => {});
  }, []);

  if (!update) return null;
  const notes = `https://github.com/chen-zilei/aion2-meter/releases/tag/v${update.version}`;

  const install = async () => {
    let total = 0;
    let done = 0;
    setProgress("Downloading…");
    try {
      await update.downloadAndInstall((e) => {
        if (e.event === "Started") total = e.data.contentLength ?? 0;
        if (e.event === "Progress") {
          done += e.data.chunkLength;
          if (total) setProgress(`Downloading… ${Math.round((done / total) * 100)}%`);
        }
        if (e.event === "Finished") setProgress("Installing…");
      });
      await relaunch();
    } catch (err) {
      setProgress(`Update failed: ${err}`);
    }
  };
  const dismiss = () => {
    localStorage.setItem(DISMISSED_KEY, update.version);
    setUpdate(null);
  };

  return (
    <div className="banner ok" style={{ marginBottom: 8 }}>
      Version {update.version} is out.{" "}
      {progress ?? (
        <>
          <button className="link" onClick={install}>Update and restart</button>{" "}
          <button className="link" onClick={() => invoke("open_release_page", { url: notes }).catch(() => {})}>
            What's new
          </button>{" "}
          <button className="link" onClick={dismiss}>Not now</button>
        </>
      )}
    </div>
  );
}
