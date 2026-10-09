import { useState } from "react";
import { api } from "../shared/api";
import { Bars } from "../shared/Bars";
import { fmtCompact, fmtDuration, fmtWhole } from "../shared/format";
import type { Snapshot, Update } from "../shared/types";
import { SkillTable } from "./SkillTable";

export function LivePage({ update }: { update: Update | null }) {
  const snap = update?.current ?? null;
  return (
    <section>
      <header className="page-head">
        <h1>Live</h1>
        <button onClick={() => api.resetEncounter()}>New encounter</button>
      </header>
      {snap ? <EncounterView snap={snap} /> : <Empty />}
    </section>
  );
}

export function EncounterView({ snap }: { snap: Snapshot }) {
  const [selected, setSelected] = useState<number | null>(null);
  const actor = snap.actors.find((a) => a.id === selected) ?? snap.actors.find((a) => a.isSelf) ?? snap.actors[0];
  return (
    <>
      <div className="stats">
        <Stat label="Target" value={snap.mainTarget || "-"} />
        <Stat label="Duration" value={fmtDuration(snap.durationS)} />
        <Stat label="Party DPS" value={fmtCompact(snap.partyDps)} />
        <Stat label="Total damage" value={fmtWhole(snap.totalDamage)} />
        <Stat label="State" value={snap.active ? "In combat" : "Finished"} />
      </div>
      <div className="split">
        <div className="card">
          <h2>Damage</h2>
          <Bars actors={snap.actors} selected={actor?.id} onSelect={setSelected} />
        </div>
        <div className="card">
          <h2>{actor ? `${actor.name}: skills` : "Skills"}</h2>
          {actor && <SkillTable actor={actor} durationS={snap.durationS} />}
        </div>
      </div>
    </>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="stat">
      <div className="dim small">{label}</div>
      <div className="stat-value">{value}</div>
    </div>
  );
}

function Empty() {
  return (
    <div className="card empty">
      <h2>No fights yet</h2>
      <p className="dim">Hit something in game and it shows up here. To try the app without the game, turn on demo mode in Settings.</p>
    </div>
  );
}
