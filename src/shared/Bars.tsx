import type { ActorStats } from "./types";
import { actorColor, fmtCompact, fmtPct } from "./format";
import { type Metric, metricRate, metricValue } from "./metrics";

/** The classic meter: one bar per actor, scaled to the top row. Pass actors already ranked for `metric`. */
export function Bars({
  actors,
  dense = false,
  selected,
  onSelect,
  firstRank = 1,
  topDamage,
  metric = "damage",
  total,
}: {
  actors: ActorStats[];
  dense?: boolean;
  selected?: number | null;
  onSelect?: (id: number) => void;
  /** Rank of the first row, for a list that continues another one. */
  firstRank?: number;
  /** Value that fills a whole bar; defaults to the first row's. */
  topDamage?: number;
  metric?: Metric;
  /** Everyone's total for `metric`, for the share column; defaults to the damage share. */
  total?: number;
}) {
  const top = topDamage || (actors[0] && metricValue(actors[0], metric)) || 1;
  return (
    <ol className={`bars ${dense ? "dense" : ""}`}>
      {actors.map((a, i) => (
        <li
          key={a.id}
          className={`bar ${a.isSelf ? "self" : ""} ${selected === a.id ? "selected" : ""} ${onSelect ? "clickable" : ""}`}
          style={{ ["--c" as string]: actorColor(a.id, a.isSelf) }}
          onClick={() => onSelect?.(a.id)}
        >
          <div className="fill" style={{ width: `${(metricValue(a, metric) / top) * 100}%` }} />
          <span className="rank">{firstRank + i}</span>
          <span className="name">{a.name}</span>
          <Value a={a} metric={metric} dense={dense} />
          <span className="num dim">{fmtPct(total ? metricValue(a, metric) / total : a.share)}</span>
        </li>
      ))}
    </ol>
  );
}

function Value({ a, metric, dense }: { a: ActorStats; metric: Metric; dense: boolean }) {
  const rate = metricRate(a, metric);
  const value = metricValue(a, metric);
  if (rate == null) return <span className="num dps">{fmtCompact(value)}</span>;
  return (
    <>
      <span className="num dps">{fmtCompact(rate)}/s</span>
      {!dense && <span className="num dim">{fmtCompact(value)}</span>}
    </>
  );
}
