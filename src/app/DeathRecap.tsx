import { useState } from "react";
import { fmtCompact, fmtDuration, fmtWhole } from "../shared/format";
import { useNames } from "../shared/names";
import type { DeathRecap } from "../shared/types";

/** Player deaths in the fight, each opening to the hits and heals that led up to it. */
export function DeathRecaps({ deaths }: { deaths: DeathRecap[] }) {
  // The latest death starts open; a click opens another or closes it.
  const [open, setOpen] = useState<number | null>(null);
  const shown = open ?? deaths.length - 1;
  const names = useNames(deaths.flatMap((d) => d.events.map((e) => e.skill)));
  return (
    <div className="card">
      <div className="card-head">
        <h2>Deaths</h2>
        <span className="dim small">Last 10 seconds before each death</span>
      </div>
      <ul className="deaths">
        {deaths.map((d, i) => {
          const blow = [...d.events].reverse().find((e) => !e.heal);
          const taken = d.events.filter((e) => !e.heal).reduce((sum, e) => sum + e.amount, 0);
          const healed = d.events.filter((e) => e.heal).reduce((sum, e) => sum + e.amount, 0);
          const isOpen = i === shown;
          return (
            <li key={`${d.id}-${d.atS}`} className={isOpen ? "open" : ""}>
              <button className="death-head" onClick={() => setOpen(isOpen ? -1 : i)} aria-expanded={isOpen}>
                <span className="num dim">{fmtDuration(d.atS)}</span>
                <span className={`death-name ${d.isSelf ? "self" : ""}`}>{d.name}</span>
                <span className="dim death-blow">
                  {blow ? <>killed by {blow.source} · {names.skill(blow.skill)} · <span className="num">{fmtCompact(blow.amount)}</span></> : "no hits seen"}
                </span>
                <span className="dim small num">{fmtCompact(taken)} taken · {fmtCompact(healed)} healed</span>
              </button>
              {isOpen && d.events.length > 0 && (
                <table className="table recap">
                  <tbody>
                    {d.events.map((e, j) => (
                      <tr key={j} className={e.heal ? "heal" : ""}>
                        <td className="r num dim">{e.beforeMs >= 50 ? `−${(e.beforeMs / 1000).toFixed(1)}s` : "0.0s"}</td>
                        <td>{e.source}</td>
                        <td className="skill-name" title={`${e.skill}`}>{names.skill(e.skill)}</td>
                        <td className="r num amount" title={fmtWhole(e.amount)}>
                          {e.heal ? "+" : "−"}{fmtCompact(e.amount)}{e.crit ? " crit" : ""}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </li>
          );
        })}
      </ul>
    </div>
  );
}
