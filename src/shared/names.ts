import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

/** Mirrors `gamedata::Status` in src-tauri/src/gamedata.rs. */
export type NameTables =
  | ({ state: "ready" } & NameCounts)
  | ({ state: "downloading" } & NameCounts)
  | ({ state: "failed"; message: string } & NameCounts);

interface NameCounts {
  skills: number;
  npcs: number;
  folder: string;
  source: string;
}

interface Lookup {
  skills: Record<string, string>;
  npcs: Record<string, string>;
}

// Names never change while the tables stay the same, so each code is asked for once (null = the tables don't know it).
const skills = new Map<number, string | null>();
const npcs = new Map<number, string | null>();
let generation = 0;
const subscribers = new Set<() => void>();
const notify = () => subscribers.forEach((f) => f());

listen<NameTables>("meter://names", (e) => {
  if (e.payload.state !== "downloading") {
    skills.clear();
    npcs.clear();
    generation++;
    notify();
  }
}).catch(() => {});

async function fetchMissing(skillIds: number[], npcIds: number[]) {
  const wantSkills = skillIds.filter((id) => !skills.has(id));
  const wantNpcs = npcIds.filter((id) => !npcs.has(id));
  if (!wantSkills.length && !wantNpcs.length) return;
  const gen = generation;
  // Mark as pending so overlapping renders don't ask twice.
  wantSkills.forEach((id) => skills.set(id, null));
  wantNpcs.forEach((id) => npcs.set(id, null));
  try {
    const found = await invoke<Lookup>("lookup_names", { skills: wantSkills, npcs: wantNpcs });
    if (gen !== generation) return;
    for (const [id, name] of Object.entries(found.skills)) skills.set(Number(id), name);
    for (const [id, name] of Object.entries(found.npcs)) npcs.set(Number(id), name);
    notify();
  } catch {
    wantSkills.forEach((id) => skills.delete(id));
    wantNpcs.forEach((id) => npcs.delete(id));
  }
}

/** Resolves skill and NPC codes to names, falling back to the code until (or unless) the tables know it. */
export function useNames(skillIds: number[], npcIds: number[] = []) {
  const [, rerender] = useState(0);
  useEffect(() => {
    const f = () => rerender((n) => n + 1);
    subscribers.add(f);
    return () => {
      subscribers.delete(f);
    };
  }, []);
  const key = `${skillIds.join(",")}|${npcIds.join(",")}`;
  useEffect(() => {
    fetchMissing(skillIds, npcIds);
  }, [key, generation]);
  return {
    skill: (id: number) => skills.get(id) ?? `Skill ${id}`,
    npc: (id: number) => npcs.get(id) ?? `NPC ${id}`,
  };
}

export const nameTablesApi = {
  status: () => invoke<NameTables>("name_tables"),
  download: () => invoke<void>("download_name_tables"),
};

/** The name tables' state, kept current while a download runs. */
export function useNameTables(): NameTables | null {
  const [status, setStatus] = useState<NameTables | null>(null);
  useEffect(() => {
    nameTablesApi.status().then(setStatus).catch(() => {});
    const off = listen<NameTables>("meter://names", (e) => setStatus(e.payload));
    return () => {
      off.then((f) => f());
    };
  }, []);
  return status;
}
