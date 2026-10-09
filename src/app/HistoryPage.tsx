import { useEffect, useState } from "react";
import { api } from "../shared/api";
import { fmtCompact, fmtDuration, fmtTime } from "../shared/format";
import type { Snapshot } from "../shared/types";
import { EncounterView } from "./LivePage";

export function HistoryPage({ historyLen }: { historyLen: number }) {
  const [items, setItems] = useState<Snapshot[]>([]);
  const [openId, setOpenId] = useState<number | null>(null);

  useEffect(() => {
    api.getHistory().then(setItems).catch(() => {});
  }, [historyLen]);

  const open = items.find((s) => s.id === openId);
  return (
    <section>
      <header className="page-head">
        <h1>History</h1>
        {open && <button onClick={() => setOpenId(null)}>Back to list</button>}
      </header>
      {open ? (
        <EncounterView snap={open} />
      ) : items.length === 0 ? (
        <div className="card empty"><p className="dim">Finished encounters show up here.</p></div>
      ) : (
        <div className="card">
          <table className="table clickable-rows">
            <thead>
              <tr>
                <th>Time</th>
                <th>Target</th>
                <th className="r">Length</th>
                <th className="r">Party DPS</th>
                <th>Top</th>
              </tr>
            </thead>
            <tbody>
              {items.map((s) => (
                <tr key={s.id} onClick={() => setOpenId(s.id)}>
                  <td>{fmtTime(s.startedMs)}</td>
                  <td>{s.mainTarget}</td>
                  <td className="r">{fmtDuration(s.durationS)}</td>
                  <td className="r">{fmtCompact(s.partyDps)}</td>
                  <td>{s.actors[0]?.name}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}
