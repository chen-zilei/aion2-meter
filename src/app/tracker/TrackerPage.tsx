import { useEffect, useState } from "react";
import {
  ACTIVITIES,
  REGIONS,
  STACKS,
  nextReset,
  periodStart,
  phaseFromNext,
  projectStack,
  type Activity,
  type Period,
  type Region,
  type Stack,
} from "./activities";
import "./tracker.css";

/** Saved in the app's local storage, so each person who uses the app has their own list. */
interface TrackerState {
  region: string;
  characters: string[];
  active: string;
  hidden: string[];
  custom: Activity[];
  /** Times done this period, keyed by activity id (server-wide) or "character/id". */
  progress: Record<Period, { start: number; done: Record<string, number> }>;
  /** Amount of each stack as last entered, and when; keyed like progress. Never cleared by a reset. */
  stacks: Record<string, { value: number; at: number }>;
  membership: boolean;
  /** Refill timing set by hand ("next refill in"), keyed like stacks; else refills line up with the reset. */
  phases: Record<string, number>;
}

const STORAGE_KEY = "aion2-meter.tracker";
const PERIODS: Period[] = ["daily", "weekly"];
/** Counts above this get a number stepper instead of a row of checkboxes. */
const MAX_PIPS = 14;

const SECTIONS: { title: string; period: Period; shop: boolean }[] = [
  { title: "Daily", period: "daily", shop: false },
  { title: "Weekly", period: "weekly", shop: false },
  { title: "Weekly shop purchases", period: "weekly", shop: true },
];

function initialState(): TrackerState {
  const fresh: TrackerState = {
    region: "global",
    characters: ["Main"],
    active: "Main",
    hidden: [],
    custom: [],
    progress: { daily: { start: 0, done: {} }, weekly: { start: 0, done: {} } },
    stacks: {},
    membership: false,
    phases: {},
  };
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    return saved ? { ...fresh, ...JSON.parse(saved) } : fresh;
  } catch {
    return fresh;
  }
}

/** Clears a period's check-offs once its reset time has passed. */
function rollOver(state: TrackerState, now: number): TrackerState {
  const region = REGIONS.find((r) => r.id === state.region) ?? REGIONS[0];
  let progress = state.progress;
  for (const period of PERIODS) {
    const start = periodStart(period, region, now);
    if (start > progress[period].start) progress = { ...progress, [period]: { start, done: {} } };
  }
  return progress === state.progress ? state : { ...state, progress };
}

function useNow(intervalMs: number) {
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), intervalMs);
    return () => clearInterval(id);
  }, [intervalMs]);
  return now;
}

function countdown(ms: number) {
  const mins = Math.max(0, Math.floor(ms / 60_000));
  const d = Math.floor(mins / 1440), h = Math.floor((mins % 1440) / 60), m = mins % 60;
  return d ? `${d}d ${h}h` : h ? `${h}h ${m}m` : `${m}m`;
}

export function TrackerPage() {
  const now = useNow(5_000);
  const [state, setState] = useState(() => rollOver(initialState(), Date.now()));
  const [showHidden, setShowHidden] = useState(false);

  useEffect(() => setState((s) => rollOver(s, now)), [now, state.region]);
  useEffect(() => {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
    } catch {
      // Storage full or unavailable: the list still works for this session.
    }
  }, [state]);

  const region = REGIONS.find((r) => r.id === state.region) ?? REGIONS[0];
  const all = [...ACTIVITIES, ...state.custom];
  const keyOf = (a: Activity | Stack) => (a.scope !== "character" ? a.id : `${state.active}/${a.id}`);

  const setDone = (a: Activity, n: number) =>
    setState((s) => {
      const p = s.progress[a.period];
      return { ...s, progress: { ...s.progress, [a.period]: { ...p, done: { ...p.done, [keyOf(a)]: n } } } };
    });

  const toggleHidden = (id: string) =>
    setState((s) => ({ ...s, hidden: s.hidden.includes(id) ? s.hidden.filter((h) => h !== id) : [...s.hidden, id] }));

  const addCharacter = () => {
    const name = prompt("Character name")?.trim();
    if (!name || state.characters.includes(name) || name.includes("/")) return;
    setState((s) => ({ ...s, characters: [...s.characters, name], active: name }));
  };

  const removeCharacter = () => {
    if (state.characters.length < 2 || !confirm(`Remove ${state.active} and its check-offs?`)) return;
    setState((s) => {
      const characters = s.characters.filter((c) => c !== s.active);
      const strip = <T,>(done: Record<string, T>) =>
        Object.fromEntries(Object.entries(done).filter(([k]) => !k.startsWith(`${s.active}/`)));
      return {
        ...s,
        characters,
        active: characters[0],
        stacks: strip(s.stacks),
        progress: {
          daily: { ...s.progress.daily, done: strip(s.progress.daily.done) },
          weekly: { ...s.progress.weekly, done: strip(s.progress.weekly.done) },
        },
      };
    });
  };

  const removeCustom = (id: string) => setState((s) => ({ ...s, custom: s.custom.filter((c) => c.id !== id) }));

  const hiddenCount = [...all, ...STACKS].filter((a) => state.hidden.includes(a.id)).length;

  return (
    <section className="tracker">
      <header className="page-head">
        <h1>Dailies & weeklies</h1>
        <div className="tracker-controls">
          <select value={state.active} onChange={(e) => setState((s) => ({ ...s, active: e.target.value }))} title="Character">
            {state.characters.map((c) => <option key={c}>{c}</option>)}
          </select>
          <button onClick={addCharacter} title="Add a character">+</button>
          {state.characters.length > 1 && <button onClick={removeCharacter} title="Remove this character">−</button>}
          <select value={state.region} onChange={(e) => setState((s) => ({ ...s, region: e.target.value }))} title="Server region">
            {REGIONS.map((r) => <option key={r.id} value={r.id}>{r.label}</option>)}
          </select>
        </div>
      </header>

      <StackCard
        stacks={STACKS.filter((st) => showHidden || !state.hidden.includes(st.id))}
        entered={state.stacks}
        keyOf={keyOf}
        membership={state.membership}
        region={region}
        now={now}
        hidden={state.hidden}
        onToggleHidden={toggleHidden}
        onMembership={(membership) => setState((s) => ({ ...s, membership }))}
        phases={state.phases}
        onPhase={(st, phase) =>
          setState((s) => {
            const phases = { ...s.phases };
            if (phase === null) delete phases[keyOf(st)];
            else phases[keyOf(st)] = phase;
            return { ...s, phases };
          })
        }
        onEnter={(st, value, at) => setState((s) => ({ ...s, stacks: { ...s.stacks, [keyOf(st)]: { value, at } } }))}
      />

      {SECTIONS.map(({ title, period, shop }) => {
        const items = all.filter(
          (a) => a.period === period && !!a.shop === shop && (showHidden || !state.hidden.includes(a.id)),
        );
        const visible = items.filter((a) => !state.hidden.includes(a.id));
        const finished = visible.filter((a) => (state.progress[period].done[keyOf(a)] ?? 0) >= a.count).length;
        const resetAt = nextReset(period, region, now);
        return (
          <div className="card" key={title}>
            <div className="tracker-head">
              <h2>{title} · {finished}/{visible.length}</h2>
              <span className="dim small" title={new Date(resetAt).toLocaleString()}>
                resets in {countdown(resetAt - now)}
              </span>
            </div>
            <ul className="tracker-list">
              {items.map((a) => {
                const done = Math.min(state.progress[period].done[keyOf(a)] ?? 0, a.count);
                const hidden = state.hidden.includes(a.id);
                return (
                  <li key={a.id} className={`${done >= a.count ? "done" : ""} ${hidden ? "hidden-item" : ""}`}>
                    {a.count > MAX_PIPS ? (
                      <div className="pips stepper">
                        <button onClick={() => setDone(a, Math.max(0, done - 1))} aria-label={`${a.name} minus one`}>−</button>
                        <input
                          type="number"
                          min={0}
                          max={a.count}
                          value={done}
                          onChange={(e) => setDone(a, Math.min(a.count, Math.max(0, Number(e.target.value) || 0)))}
                          aria-label={`${a.name} bought`}
                        />
                        <button onClick={() => setDone(a, Math.min(a.count, done + 1))} aria-label={`${a.name} plus one`}>+</button>
                      </div>
                    ) : (
                      <div className="pips">
                        {Array.from({ length: a.count }, (_, i) => (
                          <input
                            key={i}
                            type="checkbox"
                            className="switch"
                            checked={i < done}
                            onChange={() => setDone(a, i < done ? i : i + 1)}
                            aria-label={`${a.name} ${i + 1} of ${a.count}`}
                          />
                        ))}
                      </div>
                    )}
                    <div className="tracker-text">
                      <div>
                        {a.name}
                        {a.count > 1 && <span className="dim small"> {done}/{a.count}</span>}
                        {a.scope === "server" && <span className="tag">server-wide</span>}
                        {a.scope === "account" && <span className="tag">account-wide</span>}
                        {a.disputed && <span className="tag warn" title={a.disputed}>sources differ</span>}
                      </div>
                      {(a.shop || a.note) && (
                        <div className="dim small">
                          {a.shop && <b>{a.shop}. </b>}
                          {a.note}
                        </div>
                      )}
                      {a.disputed && <div className="disputed small">{a.disputed}</div>}
                    </div>
                    <button className="tracker-x" onClick={() => toggleHidden(a.id)} title={hidden ? "Show again" : "Hide (I don't do this)"}>
                      {hidden ? "show" : "hide"}
                    </button>
                    {a.custom && (
                      <button className="tracker-x" onClick={() => removeCustom(a.id)} title="Delete this task">delete</button>
                    )}
                  </li>
                );
              })}
            </ul>
          </div>
        );
      })}

      <AddCustom onAdd={(a) => setState((s) => ({ ...s, custom: [...s.custom, a] }))} />

      <p className="dim small">
        Check-offs are saved on this PC and clear themselves at the reset. Server- and account-wide items are shared by all
        your characters; the rest are per character.{" "}
        {hiddenCount > 0 && (
          <button className="link" onClick={() => setShowHidden((v) => !v)}>
            {showHidden ? "Hide" : "Show"} {hiddenCount} hidden
          </button>
        )}
      </p>
    </section>
  );
}

function AddCustom({ onAdd }: { onAdd: (a: Activity) => void }) {
  const [name, setName] = useState("");
  const [period, setPeriod] = useState<Period>("daily");
  const [count, setCount] = useState(1);

  const add = () => {
    if (!name.trim()) return;
    onAdd({ id: `custom-${Date.now()}`, name: name.trim(), period, scope: "character", count, custom: true });
    setName("");
    setCount(1);
  };

  return (
    <div className="card tracker-add">
      <h2>Add your own</h2>
      <div className="tracker-controls">
        <input placeholder="e.g. Craft potions" value={name} onChange={(e) => setName(e.target.value)} onKeyDown={(e) => e.key === "Enter" && add()} />
        <select value={period} onChange={(e) => setPeriod(e.target.value as Period)}>
          <option value="daily">Daily</option>
          <option value="weekly">Weekly</option>
        </select>
        <label className="dim small">
          times{" "}
          <input type="number" min={1} max={20} value={count} onChange={(e) => setCount(Math.min(20, Math.max(1, Number(e.target.value) || 1)))} />
        </label>
        <button onClick={add}>Add</button>
      </div>
    </div>
  );
}

function StackCard(props: {
  stacks: Stack[];
  entered: TrackerState["stacks"];
  keyOf: (st: Stack) => string;
  membership: boolean;
  region: Region;
  now: number;
  hidden: string[];
  onToggleHidden: (id: string) => void;
  onMembership: (v: boolean) => void;
  phases: Record<string, number>;
  onPhase: (st: Stack, phase: number | null) => void;
  onEnter: (st: Stack, value: number, at: number) => void;
}) {
  const { stacks, entered, keyOf, membership, region, now, hidden } = props;
  return (
    <div className="card">
      <div className="tracker-head">
        <h2>Don't overcap</h2>
        <label className="dim small stack-member">
          <input type="checkbox" className="switch" checked={membership} onChange={(e) => props.onMembership(e.target.checked)} />
          Membership caps
        </label>
      </div>
      <p className="dim small stack-intro">
        These refill on a timer and stop refilling at the cap. Type in what you have once; it counts up on its own,
        and the minus button takes off one use when you spend some.
      </p>
      <ul className="tracker-list">
        {stacks.map((st) => {
          const cap = membership && st.memberCap ? st.memberCap : st.cap;
          const saved = entered[keyOf(st)];
          const phase = props.phases[keyOf(st)];
          const proj = saved ? projectStack(st, cap, saved, region, now, phase) : null;
          const setNext = () => {
            const answer = prompt(
              `When is the next +${st.gain} ${st.unit} in game? Minutes, or h:mm (e.g. 30 or 1:45). Leave empty to go back to the default.`,
            );
            if (answer === null) return;
            const m = answer.trim().match(/^(?:(\d+):)?(\d+)$/);
            if (!answer.trim()) props.onPhase(st, null);
            else if (m) props.onPhase(st, phaseFromNext(st, Date.now(), ((Number(m[1] ?? 0) * 60) + Number(m[2])) * 60_000));
          };
          const pct = proj ? Math.min(100, (proj.value / cap) * 100) : 0;
          const full = !!proj?.full;
          const level = full ? "full" : pct >= 75 ? "high" : "";
          const isHidden = hidden.includes(st.id);
          const refill = st.every === "daily" ? "at each daily reset" : `every ${st.every} hours`;
          const set = (value: number) => props.onEnter(st, Math.max(0, value), Date.now());
          return (
            <li key={st.id} className={isHidden ? "hidden-item" : ""}>
              <div className="pips stepper stack-input">
                <AmountInput value={proj?.value} onCommit={set} label={`${st.name} you have now`} />
                <span className="dim small">/ {cap}</span>
                <button
                  disabled={!proj || proj.value < st.spend}
                  onClick={() => proj && set(proj.value - st.spend)}
                  title={`Used ${st.spend}`}
                  aria-label={`${st.name} spend ${st.spend}`}
                >
                  −{st.spend}
                </button>
              </div>
              <div className="tracker-text">
                <div>
                  {st.name}
                  {st.scope === "server" && <span className="tag">server-wide</span>}
                  {st.disputed && <span className="tag warn" title={st.disputed}>sources differ</span>}
                  <span className={`stack-when small ${level}`}>
                    {!proj
                      ? "enter how many you have"
                      : full
                        ? "full: refills are being wasted"
                        : `+${st.gain} in ${countdown(proj.nextAt - now)} · full in ${countdown(proj.fullAt - now)}`}
                    {st.every !== "daily" && (
                      <button className="link stack-sync" onClick={setNext} title="Match the game's refill timer">
                        {phase === undefined ? "set timer" : "timer set ✓"}
                      </button>
                    )}
                  </span>
                </div>
                <div className={`stack-bar ${level}`}>
                  <span style={{ width: `${pct}%` }} />
                </div>
                <div className="dim small">
                  +{st.gain} {st.gain === 1 ? st.unit.replace(/s$/, "") : st.unit} {refill}. {st.note}
                </div>
                {st.disputed && <div className="disputed small">{st.disputed}</div>}
              </div>
              <button className="tracker-x" onClick={() => props.onToggleHidden(st.id)} title={isHidden ? "Show again" : "Hide (I don't do this)"}>
                {isHidden ? "show" : "hide"}
              </button>
            </li>
          );
        })}
      </ul>
      <p className="dim small">Timed refills are lined up with the daily reset (on Global, Odyle at 07:00, 10:00, 13:00 UTC and so
        on). If the game's timer says otherwise, use "set timer" to match it.</p>
    </div>
  );
}

/** Number box that lets you clear it and type freely; saves each valid number as you type. */
function AmountInput({ value, onCommit, label }: { value?: number; onCommit: (v: number) => void; label: string }) {
  const [draft, setDraft] = useState<string | null>(null);
  return (
    <input
      type="number"
      min={0}
      placeholder="?"
      value={draft ?? value ?? ""}
      onFocus={(e) => {
        setDraft(value === undefined ? "" : String(value));
        e.target.select();
      }}
      onChange={(e) => {
        setDraft(e.target.value);
        if (e.target.value !== "" && Number.isFinite(Number(e.target.value))) onCommit(Math.floor(Number(e.target.value)));
      }}
      onBlur={() => setDraft(null)}
      onKeyDown={(e) => e.key === "Enter" && e.currentTarget.blur()}
      aria-label={label}
    />
  );
}
