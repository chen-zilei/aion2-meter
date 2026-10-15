import { fmtCompact, fmtPct, fmtWhole } from "../shared/format";
import { useNames } from "../shared/names";
import type { ActorStats } from "../shared/types";

/** Crit rates at or above this stand out in the table. */
const HIGH_CRIT = 0.45;

/** Back attacks as a share of the hits that carried a direction; a dash when none did (DoTs, for one). */
function fmtBack({ backHits, frontHits }: { backHits: number; frontHits: number }) {
  const directed = backHits + frontHits;
  return directed ? fmtPct(backHits / directed) : "–";
}

/** Back and front attack rates for the whole fight, above the skill table. */
export function Positioning({ actor }: { actor: ActorStats }) {
  const directed = actor.backHits + actor.frontHits;
  if (!directed) return null;
  return (
    <div className="positioning" title={`${directed} of ${actor.hits} hits carried a direction; DoT ticks never do.`}>
      <span><span className="dim">Back attacks</span> <span className="num">{fmtPct(actor.backHits / directed)}</span></span>
      <span><span className="dim">Front attacks</span> <span className="num">{fmtPct(actor.frontHits / directed)}</span></span>
    </div>
  );
}

export function SkillTable({ actor, durationS }: { actor: ActorStats; durationS: number }) {
  const names = useNames(actor.skills.map((s) => s.skill));
  const topShare = actor.damage && actor.skills[0] ? actor.skills[0].damage / actor.damage : 1;
  return (
    <table className="table">
      <thead>
        <tr>
          <th>Skill</th>
          <th className="share-col">Share</th>
          <th className="r">DPS</th>
          <th className="r">Hits</th>
          <th className="r">Crit</th>
          <th className="r" title="Share of this skill's hits that landed from behind the target">Back</th>
          <th className="r">Max</th>
        </tr>
      </thead>
      <tbody>
        {actor.skills.map((s) => {
          const share = actor.damage ? s.damage / actor.damage : 0;
          const crit = s.hits ? s.crits / s.hits : 0;
          return (
          <tr key={s.skill}>
            <td className="skill-name" title={`${s.skill}`}>
              {names.skill(s.skill)}
              {s.summon && <span className="badge" title="Dealt by their summon">summon</span>}
            </td>
            <td title={`${fmtWhole(s.damage)} damage`}>
              <div className="share">
                <div className="share-track"><div className="share-fill" style={{ width: `${(share / (topShare || 1)) * 100}%` }} /></div>
                <span className="share-pct">{fmtPct(share)}</span>
              </div>
            </td>
            <td className="r">{fmtCompact(s.damage / durationS)}</td>
            <td className="r dim">{s.hits}</td>
            <td className={`r ${crit >= HIGH_CRIT ? "high-crit" : "dim"}`}>{fmtPct(crit)}</td>
            <td className="r dim">{fmtBack(s)}</td>
            <td className="r dim">{fmtCompact(s.maxHit)}</td>
          </tr>
          );
        })}
      </tbody>
    </table>
  );
}
