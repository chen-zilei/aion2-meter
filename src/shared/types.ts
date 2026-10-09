// Mirrors the Rust types sent over `meter://update` (see src-tauri/src/lib.rs and crates/meter-core/src/combat.rs).

export interface SkillStats {
  skill: number;
  damage: number;
  hits: number;
  crits: number;
  maxHit: number;
}

export interface ActorStats {
  id: number;
  name: string;
  isSelf: boolean;
  isPlayer: boolean;
  damage: number;
  dps: number;
  share: number;
  hits: number;
  crits: number;
  skills: SkillStats[];
}

export interface Snapshot {
  id: number;
  active: boolean;
  startedMs: number;
  durationS: number;
  totalDamage: number;
  partyDps: number;
  mainTarget: string;
  actors: ActorStats[];
}

export type CaptureStatus =
  | { state: "npcapMissing" }
  | { state: "off" }
  | { state: "waiting"; adapters: number }
  | { state: "capturing"; adapter: string }
  | { state: "error"; message: string };

export interface Settings {
  gamePorts: number[];
  idleTimeoutS: number;
  demo: boolean;
  overlayVisible: boolean;
  overlayLocked: boolean;
  overlayOpacity: number;
  playersOnly: boolean;
  onlyMyFights: boolean;
}

export interface Update {
  status: CaptureStatus;
  current: Snapshot | null;
  historyLen: number;
  selfName: string | null;
  /** Round trip to the game server in ms, measured from captured traffic; null without a recent reading. */
  pingMs: number | null;
  settings: Settings;
}
