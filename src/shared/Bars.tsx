import type { ActorStats } from "./types";
import { actorColor, fmtCompact, fmtPct } from "./format";

/** The classic meter: one bar per actor, scaled to the top damage dealer. */
export function Bars({
  actors,
  dense = false,
  selected,
  onSelect,
  firstRank = 1,
  topDamage,
}: {
  actors: ActorStats[];
  dense?: boolean;
  selected?: number | null;
  onSelect?: (id: number) => void;
  /** Rank of the first row, for a list that continues another one. */
  firstRank?: number;
  /** Damage that fills a whole bar; defaults to the first row's. */
  topDamage?: number;
}) {
  const top = topDamage || actors[0]?.damage || 1;
  return (
    <ol className={`bars ${dense ? "dense" : ""}`}>
      {actors.map((a, i) => (
        <li
          key={a.id}
          className={`bar ${a.isSelf ? "self" : ""} ${selected === a.id ? "selected" : ""} ${onSelect ? "clickable" : ""}`}
          style={{ ["--c" as string]: actorColor(a.id, a.isSelf) }}
          onClick={() => onSelect?.(a.id)}
        >
          <div className="fill" style={{ width: `${(a.damage / top) * 100}%` }} />
          <span className="rank">{firstRank + i}</span>
          <span className="name">{a.name}</span>
          <span className="num dps">{fmtCompact(a.dps)}/s</span>
          {!dense && <span className="num dim">{fmtCompact(a.damage)}</span>}
          <span className="num dim">{fmtPct(a.share)}</span>
        </li>
      ))}
    </ol>
  );
}
