import { fmtCompact, fmtPct, fmtWhole } from "../shared/format";
import { useNames } from "../shared/names";
import type { ActorStats } from "../shared/types";

export function SkillTable({ actor, durationS }: { actor: ActorStats; durationS: number }) {
  const names = useNames(actor.skills.map((s) => s.skill));
  return (
    <table className="table">
      <thead>
        <tr>
          <th>Skill</th>
          <th className="r">Damage</th>
          <th className="r">DPS</th>
          <th className="r">Share</th>
          <th className="r">Hits</th>
          <th className="r">Crit</th>
          <th className="r">Max</th>
        </tr>
      </thead>
      <tbody>
        {actor.skills.map((s) => (
          <tr key={s.skill}>
            <td title={`${s.skill}`}>{names.skill(s.skill)}</td>
            <td className="r">{fmtWhole(s.damage)}</td>
            <td className="r">{fmtCompact(s.damage / durationS)}</td>
            <td className="r">{fmtPct(actor.damage ? s.damage / actor.damage : 0)}</td>
            <td className="r">{s.hits}</td>
            <td className="r">{fmtPct(s.hits ? s.crits / s.hits : 0)}</td>
            <td className="r">{fmtCompact(s.maxHit)}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
