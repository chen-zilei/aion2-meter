import { useState } from "react";
import { api } from "../shared/api";
import { Bars } from "../shared/Bars";
import { fmtCompact, fmtDuration, fmtWhole } from "../shared/format";
import type { Snapshot, Update } from "../shared/types";
import { SkillTable } from "./SkillTable";
import { Icon } from "./icons";

export function LivePage({ update }: { update: Update | null }) {
  const snap = update?.current ?? null;
  return (
    <section>
      {snap ? <EncounterView snap={snap} live /> : <Empty />}
    </section>
  );
}

export function EncounterView({ snap, live = false }: { snap: Snapshot; live?: boolean }) {
  const [selected, setSelected] = useState<number | null>(null);
  const actor = snap.actors.find((a) => a.id === selected) ?? snap.actors.find((a) => a.isSelf) ?? snap.actors[0];
  return (
    <>
      <section className="hero">
        <div className="hero-title">
          <div className={`hero-state ${snap.active ? "active" : ""}`}>
            <span className="dot" />
            {snap.active ? "In combat" : "Finished"}
          </div>
          <h1>{snap.mainTarget || "Encounter"}</h1>
          <div className="dim">{plural(snap.actors.filter((a) => a.isPlayer).length, "player")}</div>
        </div>
        <div className="hero-stats">
          <Stat label="Party DPS" value={fmtCompact(snap.partyDps)} accent />
          <Stat label="Duration" value={fmtDuration(snap.durationS)} />
          <Stat label="Total damage" value={fmtCompact(snap.totalDamage)} title={fmtWhole(snap.totalDamage)} />
        </div>
        {live && (
          <button className="hero-button" onClick={() => api.resetEncounter()}>
            <Icon name="reset" size={14} />
            New encounter
          </button>
        )}
      </section>
      <div className="split">
        <div className="card">
          <div className="card-head">
            <h2>Damage</h2>
            <span className="dim small">DPS · total · share</span>
          </div>
          <Bars actors={snap.actors} selected={actor?.id} onSelect={setSelected} />
        </div>
        <div className="card">
          <div className="card-head">
            <h2>{actor ? `${actor.name}'s skills` : "Skills"}</h2>
            <span className="dim small">Click a bar to switch player</span>
          </div>
          {actor && <div className="table-wrap"><SkillTable actor={actor} durationS={snap.durationS} /></div>}
        </div>
      </div>
    </>
  );
}

function Stat({ label, value, title, accent = false }: { label: string; value: string; title?: string; accent?: boolean }) {
  return (
    <div className="stat" title={title}>
      <div className="dim small">{label}</div>
      <div className={`stat-value num ${accent ? "accent" : ""}`}>{value}</div>
    </div>
  );
}

const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;

function Empty() {
  return (
    <div className="card empty">
      <h2>No fights yet</h2>
      <p className="dim">Hit something in game and it shows up here. To try the app without the game, turn on demo mode in Settings.</p>
    </div>
  );
}
