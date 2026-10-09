import { useLayoutEffect, useMemo, useRef, useState } from "react";
import { actorColor, fmtCompact, fmtDuration } from "../shared/format";
import type { ActorStats, Snapshot } from "../shared/types";

/** DPS at each second is the average over this many seconds up to it, so bursts of skills read as a trend. */
const SMOOTH_S = 5;
const HEIGHT = 220;
/** Neutral, so it never matches a player's colour. */
const PARTY_COLOR = "var(--text-2)";
const PAD = { top: 12, right: 12, bottom: 24, left: 52 };

interface Series {
  id: number | "party";
  name: string;
  color: string;
  values: number[];
}

/** Party DPS and each player's DPS over the fight. */
export function DpsChart({ snap, selected, onSelect }: { snap: Snapshot; selected?: number; onSelect: (id: number) => void }) {
  const wrap = useRef<HTMLDivElement>(null);
  const width = useWidth(wrap);
  const [hover, setHover] = useState<number | null>(null);
  // Without the party line the players fill the chart, which makes their lines easier to compare.
  const [showParty, setShowParty] = useState(true);

  const { party, players, seconds } = useMemo(() => buildSeries(snap), [snap]);
  const max = niceMax(Math.max(1, ...(showParty ? party.values : players.flatMap((p) => p.values))));
  const plotW = Math.max(1, width - PAD.left - PAD.right);
  const plotH = HEIGHT - PAD.top - PAD.bottom;
  const x = (i: number) => PAD.left + (seconds > 1 ? (i / (seconds - 1)) * plotW : plotW / 2);
  const y = (v: number) => PAD.top + plotH - (v / max) * plotH;
  const path = (values: number[]) => values.map((v, i) => `${i ? "L" : "M"}${x(i).toFixed(1)},${y(v).toFixed(1)}`).join("");
  const area = `${path(party.values)}L${x(party.values.length - 1).toFixed(1)},${y(0)}L${x(0).toFixed(1)},${y(0)}Z`;
  const yTicks = [0, max / 2, max];
  const xTicks = timeTicks(seconds);
  // The selected line goes last so it is drawn on top.
  const ordered = [...players.filter((p) => p.id !== selected), ...players.filter((p) => p.id === selected)];

  function onMove(e: React.PointerEvent<SVGSVGElement>) {
    const rect = e.currentTarget.getBoundingClientRect();
    const i = Math.round(((e.clientX - rect.left - PAD.left) / plotW) * (seconds - 1));
    setHover(Math.min(seconds - 1, Math.max(0, i)));
  }

  const hovered = hover == null ? [] : [...(showParty ? [party] : []), ...players].filter((s) => s.values[hover] > 0 || s === party).sort((a, b) => b.values[hover] - a.values[hover]);

  return (
    <div className="card">
      <div className="card-head">
        <h2>DPS over time</h2>
        <span className="dim small">{SMOOTH_S}-second average</span>
      </div>
      <div className="chart-legend">
        <button className={`legend-item ${showParty ? "" : "off"}`} onClick={() => setShowParty(!showParty)} title={showParty ? "Hide the party line" : "Show the party line"}>
          <i style={{ background: PARTY_COLOR }} />
          Party
        </button>
        {players.map((p) => (
          <button key={p.id} className={`legend-item ${p.id === selected ? "on" : ""}`} onClick={() => onSelect(p.id as number)}>
            <i style={{ background: p.color }} />
            {p.name}
          </button>
        ))}
      </div>
      <div className="chart" ref={wrap}>
        {seconds < 2 ? (
          <p className="dim small">The chart fills in after a couple of seconds of fighting.</p>
        ) : (
          <svg width={width} height={HEIGHT} onPointerMove={onMove} onPointerLeave={() => setHover(null)} role="img" aria-label="Party and player DPS over the fight">
            {yTicks.map((t) => (
              <g key={t}>
                <line className="grid" x1={PAD.left} x2={PAD.left + plotW} y1={y(t)} y2={y(t)} />
                <text className="axis" x={PAD.left - 8} y={y(t) + 4} textAnchor="end">{fmtCompact(t)}</text>
              </g>
            ))}
            {xTicks.map((t) => (
              <text key={t} className="axis" x={x(t)} y={HEIGHT - 6} textAnchor="middle">{fmtDuration(t)}</text>
            ))}
            {showParty && (
              <>
                <path d={area} fill={PARTY_COLOR} opacity={0.07} />
                <path d={path(party.values)} className="line" stroke={PARTY_COLOR} />
              </>
            )}
            {ordered.map((p) => (
              <path key={p.id} d={path(p.values)} className={`line ${p.id === selected ? "selected" : "faint"}`} stroke={p.color} />
            ))}
            {hover != null && (
              <g>
                <line className="crosshair" x1={x(hover)} x2={x(hover)} y1={PAD.top} y2={PAD.top + plotH} />
                {hovered.map((s) => (
                  <circle key={s.id} cx={x(hover)} cy={y(s.values[hover])} r={s === party || s.id === selected ? 4 : 3} fill={s.color} stroke="var(--panel)" strokeWidth={2} />
                ))}
              </g>
            )}
          </svg>
        )}
        {hover != null && (
          <div className="chart-tip" style={{ left: Math.min(x(hover) + 12, width - 190) }}>
            <div className="dim small">{fmtDuration(hover)}</div>
            {hovered.slice(0, 7).map((s) => (
              <div key={s.id} className={`tip-row ${s.id === selected ? "on" : ""}`}>
                <i style={{ background: s.color }} />
                <span className="tip-name">{s.name}</span>
                <span className="num">{fmtCompact(s.values[hover])}/s</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function buildSeries(snap: Snapshot) {
  const dealers = snap.actors.filter((a) => a.damage > 0).sort((a, b) => b.damage - a.damage);
  // A live fight runs on past its last hit, so the chart reaches the present.
  const seconds = Math.max(Math.round(snap.durationS) + 1, ...dealers.map((a) => a.timeline.length));
  const raw = (a: ActorStats) => Array.from({ length: seconds }, (_, i) => a.timeline[i] ?? 0);
  const partyRaw = new Array<number>(seconds).fill(0);
  for (const a of dealers.filter((a) => a.isPlayer)) raw(a).forEach((v, i) => (partyRaw[i] += v));
  const players: Series[] = dealers.map((a) => ({ id: a.id, name: a.name, color: actorColor(a.id, a.isSelf), values: smooth(raw(a)) }));
  const party: Series = { id: "party", name: "Party", color: PARTY_COLOR, values: smooth(partyRaw) };
  return { party, players, seconds };
}

/** Trailing average over [`SMOOTH_S`] seconds (fewer at the start of the fight). */
function smooth(perSecond: number[]) {
  let sum = 0;
  return perSecond.map((v, i) => {
    sum += v - (i >= SMOOTH_S ? perSecond[i - SMOOTH_S] : 0);
    return sum / Math.min(i + 1, SMOOTH_S);
  });
}

/** Rounds up to 1, 2 or 5 times a power of ten, so the gridlines land on round numbers. */
function niceMax(v: number) {
  const p = 10 ** Math.floor(Math.log10(v));
  return ([1, 2, 5, 10].find((m) => m * p >= v) ?? 10) * p;
}

/** Second marks at a round step that gives at most about six labels. */
function timeTicks(seconds: number) {
  const step = [5, 10, 15, 30, 60, 120, 300, 600].find((s) => seconds / s <= 6) ?? 1200;
  const ticks = [];
  for (let t = 0; t < seconds; t += step) ticks.push(t);
  return ticks;
}

function useWidth(ref: React.RefObject<HTMLDivElement | null>) {
  const [width, setWidth] = useState(600);
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const ro = new ResizeObserver(([e]) => setWidth(Math.floor(e.contentRect.width)));
    ro.observe(el);
    return () => ro.disconnect();
  }, [ref]);
  return width;
}
