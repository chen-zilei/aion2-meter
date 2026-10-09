import type { ActorStats } from "./types";
import { actorColor, fmtCompact, fmtPct } from "./format";

/** The classic meter: one bar per actor, scaled to the top damage dealer. */
export function Bars({
  actors,
  dense = false,
  selected,
  onSelect,
}: {
  actors: ActorStats[];
  dense?: boolean;
  selected?: number | null;
  onSelect?: (id: number) => void;
}) {
  const top = actors[0]?.damage || 1;
  return (
    <ol className={`bars ${dense ? "dense" : ""}`}>
      {actors.map((a, i) => (
        <li
          key={a.id}
          className={`bar ${selected === a.id ? "selected" : ""} ${onSelect ? "clickable" : ""}`}
          onClick={() => onSelect?.(a.id)}
        >
          <div className="fill" style={{ width: `${(a.damage / top) * 100}%`, background: actorColor(a.id, a.isSelf) }} />
          <span className="rank">{i + 1}</span>
          <span className="name">{a.name}</span>
          <span className="num">{fmtCompact(a.dps)}/s</span>
          {!dense && <span className="num dim">{fmtCompact(a.damage)}</span>}
          <span className="num dim">{fmtPct(a.share)}</span>
        </li>
      ))}
    </ol>
  );
}
