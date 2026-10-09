import { fmtCompact, fmtPct, fmtWhole } from "../shared/format";
import { useNames } from "../shared/names";
import type { ActorStats } from "../shared/types";

/** Crit rates at or above this stand out in the table. */
const HIGH_CRIT = 0.45;

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
          <th className="r">Max</th>
        </tr>
      </thead>
      <tbody>
        {actor.skills.map((s) => {
          const share = actor.damage ? s.damage / actor.damage : 0;
          const crit = s.hits ? s.crits / s.hits : 0;
          return (
          <tr key={s.skill}>
            <td className="skill-name" title={`${s.skill}`}>{names.skill(s.skill)}</td>
            <td title={`${fmtWhole(s.damage)} damage`}>
              <div className="share">
                <div className="share-track"><div className="share-fill" style={{ width: `${(share / (topShare || 1)) * 100}%` }} /></div>
                <span className="share-pct">{fmtPct(share)}</span>
              </div>
            </td>
            <td className="r">{fmtCompact(s.damage / durationS)}</td>
            <td className="r dim">{s.hits}</td>
            <td className={`r ${crit >= HIGH_CRIT ? "high-crit" : "dim"}`}>{fmtPct(crit)}</td>
            <td className="r dim">{fmtCompact(s.maxHit)}</td>
          </tr>
          );
        })}
      </tbody>
    </table>
  );
}
