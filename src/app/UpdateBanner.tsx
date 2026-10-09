import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

type Release = { version: string; url: string };

const DISMISSED_KEY = "aion2meter.dismissedRelease";

/** Checks GitHub once on launch and offers the newer release, until the user dismisses that version. */
export function UpdateBanner() {
  const [release, setRelease] = useState<Release | null>(null);

  useEffect(() => {
    invoke<Release | null>("check_release")
      .then((r) => {
        if (r && localStorage.getItem(DISMISSED_KEY) !== r.version) setRelease(r);
      })
      .catch(() => {});
  }, []);

  if (!release) return null;
  const dismiss = () => {
    localStorage.setItem(DISMISSED_KEY, release.version);
    setRelease(null);
  };
  return (
    <div className="banner ok" style={{ marginBottom: 8 }}>
      Version {release.version} is out.{" "}
      <button className="link" onClick={() => invoke("open_release_page", { url: release.url }).catch(() => {})}>
        Download it
      </button>{" "}
      <button className="link" onClick={dismiss}>Dismiss</button>
    </div>
  );
}
