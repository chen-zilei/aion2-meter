import { Fragment, useEffect, useMemo, useState } from "react";
import { fmtWhole } from "../../shared/format";
import { marketApi, type Price, type PriceBook, type Recipe, type RecipeBook, type Server, type ServerList } from "./api";
import "./market.css";

/** Saved in the app's local storage, like the tracker's choices. */
interface MarketPrefs {
  server: number;
  /** Market tax in percent. */
  tax: number;
  hideUnpriced: boolean;
  sort: SortKey;
  desc: boolean;
}

type SortKey = "profit" | "margin" | "price" | "name";

const STORAGE_KEY = "aion2-meter.market";
/** Lumiel, Asia, Asmodian. */
const DEFAULT_SERVER = 12504;
const PAGE_ROWS = 150;
const SIDE: Record<string, string> = { Elyos: "light", Asmodian: "dark" };

function loadPrefs(): MarketPrefs {
  const fresh: MarketPrefs = { server: DEFAULT_SERVER, tax: 10, hideUnpriced: true, sort: "profit", desc: true };
  try {
    return { ...fresh, ...JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "{}") };
  } catch {
    return fresh;
  }
}

interface Row {
  recipe: Recipe;
  name: string;
  /** One unit of the output, cheapest listing. */
  unitPrice: number | null;
  listings: number;
  unitsSold: number | null;
  /** Materials plus the crafting fee; null while any material has no price. */
  cost: number | null;
  /** Items (output or materials) with no price on this server. */
  unpriced: string[];
  profit: number | null;
  margin: number | null;
}

const priceOf = (p: Price | undefined) => (p?.price != null && p.price > 0 ? p.price : null);

function buildRow(recipe: Recipe, book: RecipeBook, prices: Record<string, Price>, tax: number): Row {
  const out = prices[recipe.output];
  const unitPrice = priceOf(out);
  const unpriced = unitPrice == null ? [recipe.output] : [];
  let mats = 0;
  for (const i of recipe.inputs) {
    const p = priceOf(prices[i.item]);
    if (p == null) unpriced.push(i.item);
    else mats += p * i.qty;
  }
  const matsPriced = recipe.inputs.every((i) => priceOf(prices[i.item]) != null);
  const cost = matsPriced ? mats + recipe.fee : null;
  const sale = unitPrice != null ? unitPrice * recipe.outputQty : null;
  const profit = sale != null && cost != null ? sale * (1 - tax / 100) - cost : null;
  return {
    recipe,
    name: book.items[recipe.output]?.name ?? `#${recipe.output}`,
    unitPrice,
    listings: out?.listings ?? 0,
    unitsSold: out?.unitsSold ?? null,
    cost,
    unpriced,
    profit,
    margin: profit != null && sale ? profit / sale : null,
  };
}

function compare(a: Row, b: Row, sort: SortKey, desc: boolean) {
  if (sort === "name") return desc ? b.name.localeCompare(a.name) : a.name.localeCompare(b.name);
  const pick = (r: Row) => (sort === "profit" ? r.profit : sort === "margin" ? r.margin : r.unitPrice);
  const x = pick(a), y = pick(b);
  // Rows we can't work out always go last, whichever way the column is sorted.
  if (x == null || y == null) return x == null ? (y == null ? a.name.localeCompare(b.name) : 1) : -1;
  return desc ? y - x : x - y;
}

const kinah = (n: number | null) => (n == null ? "—" : fmtWhole(n));
const signed = (n: number) => (n > 0 ? "+" : n < 0 ? "−" : "") + fmtWhole(Math.abs(n));

export function MarketPage() {
  const [prefs, setPrefs] = useState(loadPrefs);
  const [book, setBook] = useState<RecipeBook | null>(null);
  const [bookError, setBookError] = useState<string | null>(null);
  const [bookBusy, setBookBusy] = useState(false);
  const [servers, setServers] = useState<ServerList | null>(null);
  const [prices, setPrices] = useState<PriceBook | null>(null);
  const [priceError, setPriceError] = useState<string | null>(null);
  const [priceBusy, setPriceBusy] = useState(false);
  const [search, setSearch] = useState("");
  const [open, setOpen] = useState<string | null>(null);
  const [limit, setLimit] = useState(PAGE_ROWS);

  useEffect(() => {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(prefs));
    } catch {
      // Storage unavailable: the choices still hold for this session.
    }
  }, [prefs]);

  const loadRecipes = (refresh: boolean) => {
    setBookBusy(true);
    setBookError(null);
    marketApi
      .recipes(refresh)
      .then(setBook)
      .catch((e) => setBookError(String(e)))
      .finally(() => setBookBusy(false));
  };

  useEffect(() => {
    loadRecipes(false);
    marketApi.servers().then(setServers).catch(() => {});
  }, []);

  const loadPrices = (refresh: boolean) => {
    const server = prefs.server;
    setPriceBusy(true);
    setPriceError(null);
    marketApi
      .prices(server, refresh)
      .then((p) => setPrices((cur) => (p.server === server ? p : cur)))
      .catch((e) => setPriceError(String(e)))
      .finally(() => setPriceBusy(false));
  };

  // Only while the page is open: the backend keeps prices for ten minutes, so coming back to the tab is free.
  useEffect(() => loadPrices(false), [prefs.server]);

  const server = servers?.servers.find((s) => s.serverId === prefs.server);
  const side = server ? SIDE[server.faction] : undefined;
  const shownPrices = prices?.server === prefs.server ? prices : null;
  const salesTracked = shownPrices ? Object.values(shownPrices.prices).some((p) => p.unitsSold != null) : true;

  const rows = useMemo(() => {
    if (!book || !shownPrices) return [];
    const q = search.trim().toLowerCase();
    return book.recipes
      .filter((r) => !side || r.side === side)
      .map((r) => buildRow(r, book, shownPrices.prices, prefs.tax))
      .filter((r) => !q || r.name.toLowerCase().includes(q) || r.recipe.profession.includes(q))
      .sort((a, b) => compare(a, b, prefs.sort, prefs.desc));
  }, [book, shownPrices, side, search, prefs.tax, prefs.sort, prefs.desc]);

  const complete = rows.filter((r) => !r.unpriced.length);
  const visible = prefs.hideUnpriced ? complete : rows;
  const hiddenCount = rows.length - complete.length;

  const sortBy = (sort: SortKey) =>
    setPrefs((p) => ({ ...p, sort, desc: p.sort === sort ? !p.desc : sort !== "name" }));
  const arrow = (key: SortKey) => (prefs.sort === key ? (prefs.desc ? " ↓" : " ↑") : "");

  return (
    <section className="market">
      <header className="page-head">
        <h1>Market</h1>
        <div className="market-controls">
          <ServerPicker servers={servers} value={prefs.server} onChange={(s) => { setPrefs((p) => ({ ...p, server: s })); setOpen(null); }} />
          <button disabled={priceBusy} onClick={() => loadPrices(true)} title="Fetch the latest snapshot (aion2.exchange takes one an hour)">
            {priceBusy ? "Loading…" : "Refresh prices"}
          </button>
        </div>
      </header>

      {bookError && (
        <div className="banner warn">
          Couldn't load crafting recipes: {bookError}{" "}
          <button className="link" onClick={() => loadRecipes(false)}>Try again</button>
        </div>
      )}
      {priceError && (
        <div className="banner warn">
          Couldn't load prices: {priceError}
          {shownPrices && " Showing the last prices fetched."}{" "}
          <button className="link" onClick={() => loadPrices(true)}>Try again</button>
        </div>
      )}

      <div className="card market-filters">
        <input
          className="market-search"
          placeholder="Search recipes"
          value={search}
          onChange={(e) => { setSearch(e.target.value); setLimit(PAGE_ROWS); }}
        />
        <label className="market-check">
          <input type="checkbox" className="switch" checked={prefs.hideUnpriced} onChange={(e) => setPrefs((p) => ({ ...p, hideUnpriced: e.target.checked }))} />
          Hide recipes with unpriced items
          {hiddenCount > 0 && <span className="dim small">({fmtWhole(hiddenCount)})</span>}
        </label>
        <label className="market-check" title="Not confirmed in-game yet; set it to what the broker actually takes.">
          Market tax
          <input
            type="number"
            min={0}
            max={50}
            step={0.5}
            value={prefs.tax}
            onChange={(e) => setPrefs((p) => ({ ...p, tax: Math.min(50, Math.max(0, Number(e.target.value) || 0)) }))}
          />
          % <span className="tag warn">unconfirmed</span>
        </label>
      </div>

      {!book || !shownPrices ? (
        <div className="card empty dim">
          {!book && bookBusy
            ? "Loading crafting recipes… the first time, they are downloaded from gamers4.life."
            : !shownPrices && priceBusy
              ? "Loading prices from aion2.exchange…"
              : bookError || priceError
                ? "Nothing to show until the data above loads."
                : "Loading…"}
        </div>
      ) : (
        <div className="card">
          <div className="card-head">
            <h2>
              Crafting profit · {fmtWhole(visible.length)} recipes
              {server && <span className="dim small"> · {server.faction} recipes</span>}
            </h2>
            {!salesTracked && <span className="dim small">Sales aren't tracked on this server yet.</span>}
          </div>
          <div className="table-wrap">
            <table className="table market-table">
              <thead>
                <tr>
                  <th><button className="th-sort" onClick={() => sortBy("name")}>Recipe{arrow("name")}</button></th>
                  <th className="r"><button className="th-sort" onClick={() => sortBy("price")}>Price{arrow("price")}</button></th>
                  <th className="r">Listed</th>
                  <th className="r" title="Units sold in the last 24 hours">Sold 24h</th>
                  <th className="r" title="Materials at their cheapest listing, plus the crafting fee">Cost</th>
                  <th className="r"><button className="th-sort" onClick={() => sortBy("profit")} title="Sale price after tax, minus cost">Profit{arrow("profit")}</button></th>
                  <th className="r"><button className="th-sort" onClick={() => sortBy("margin")} title="Profit as a share of the sale price">Margin{arrow("margin")}</button></th>
                </tr>
              </thead>
              <tbody>
                {visible.slice(0, limit).map((row) => (
                  <Fragment key={row.recipe.id}>
                    <tr className={`market-row ${open === row.recipe.id ? "open" : ""}`} onClick={() => setOpen(open === row.recipe.id ? null : row.recipe.id)}>
                      <td className="market-name">
                        <span className="caret">{open === row.recipe.id ? "▾" : "▸"}</span>
                        {row.name}
                        {row.recipe.outputQty > 1 && <span className="dim"> ×{row.recipe.outputQty}</span>}
                        {row.recipe.combo && <span className="tag" title="A lucky craft can make a better item; profit counts the normal result.">combo</span>}
                        {row.unpriced.length > 0 && (
                          <span className="tag warn" title="No listing on this server, so profit can't be worked out.">
                            {row.unpriced.length} unpriced
                          </span>
                        )}
                        <span className="dim small market-sub">
                          {row.recipe.profession} · lv {row.recipe.level}
                        </span>
                      </td>
                      <td className="r">{kinah(row.unitPrice)}</td>
                      <td className="r dim">{row.listings || "—"}</td>
                      <td className="r dim" title={row.unitsSold == null ? "Sales aren't tracked on this server yet" : undefined}>
                        {row.unitsSold == null ? "—" : fmtWhole(row.unitsSold)}
                      </td>
                      <td className="r">{kinah(row.cost)}</td>
                      <td className={`r market-profit ${row.profit == null ? "" : row.profit >= 0 ? "gain" : "loss"}`}>
                        {row.profit == null ? "—" : signed(row.profit)}
                      </td>
                      <td className="r">{row.margin == null ? "—" : `${(row.margin * 100).toFixed(1)}%`}</td>
                    </tr>
                    {open === row.recipe.id && (
                      <tr className="market-detail">
                        <td colSpan={7}>
                          <Materials row={row} book={book} prices={shownPrices} />
                        </td>
                      </tr>
                    )}
                  </Fragment>
                ))}
              </tbody>
            </table>
          </div>
          {visible.length === 0 && (
            <p className="dim small">
              {search ? "No recipe matches that search." : "No recipe has a price for its result and every material on this server."}
            </p>
          )}
          {visible.length > limit && (
            <button className="market-more" onClick={() => setLimit((l) => l + PAGE_ROWS)}>
              Show {fmtWhole(Math.min(PAGE_ROWS, visible.length - limit))} more
            </button>
          )}
        </div>
      )}

      <footer className="market-footer dim small">
        <span>
          Prices: <button className="link" onClick={() => marketApi.openSource("prices")}>aion2.exchange</button> · Recipes:{" "}
          <button className="link" onClick={() => marketApi.openSource("recipes")}>gamers4.life</button>
        </span>
        {shownPrices && (
          <span title={`Fetched ${new Date(shownPrices.fetchedAt).toLocaleString()}`}>
            Snapshot {shownPrices.capturedAt ? new Date(shownPrices.capturedAt).toLocaleString() : "time unknown"}
          </span>
        )}
        {book && (
          <span>
            Recipes from {new Date(book.downloadedAt).toLocaleDateString()}{" "}
            <button className="link" disabled={bookBusy} onClick={() => loadRecipes(true)}>
              {bookBusy ? "downloading…" : "download again"}
            </button>
          </span>
        )}
      </footer>
    </section>
  );
}

function ServerPicker({ servers, value, onChange }: { servers: ServerList | null; value: number; onChange: (id: number) => void }) {
  const groups = useMemo(() => {
    const map = new Map<string, Server[]>();
    for (const s of servers?.servers ?? []) {
      const key = `${s.regionName || s.region} · ${s.faction}`;
      map.set(key, [...(map.get(key) ?? []), s]);
    }
    return [...map];
  }, [servers]);
  return (
    <select value={value} onChange={(e) => onChange(Number(e.target.value))} title="Server">
      {!servers && <option value={value}>{value === DEFAULT_SERVER ? "Lumiel (Asia, Asmodian)" : `Server ${value}`}</option>}
      {groups.map(([label, list]) => (
        <optgroup key={label} label={label}>
          {list.map((s) => (
            <option key={s.serverId} value={s.serverId} disabled={!s.hasData}>
              {s.name} ({s.region})
              {!s.hasData && " (no data)"}
            </option>
          ))}
        </optgroup>
      ))}
    </select>
  );
}

function Materials({ row, book, prices }: { row: Row; book: RecipeBook; prices: PriceBook }) {
  const { recipe } = row;
  const [trends, setTrends] = useState<Record<string, number[]> | null>(null);
  useEffect(() => {
    marketApi
      .trends(prices.server, [recipe.output, ...recipe.inputs.map((i) => i.item)])
      .then(setTrends)
      .catch(() => setTrends({}));
  }, [prices.server, recipe.id]);

  const name = (id: string) => book.items[id]?.name ?? `#${id}`;
  const combo = recipe.combo ? priceOf(prices.prices[recipe.combo]) : null;

  return (
    <div className="market-materials">
      <table className="table">
        <thead>
          <tr>
            <th>Material</th>
            <th className="r">Qty</th>
            <th className="r">Unit price</th>
            <th className="r">Subtotal</th>
            <th className="r">Listed</th>
            <th>7 days</th>
          </tr>
        </thead>
        <tbody>
          {recipe.inputs.map((i) => {
            const p = prices.prices[i.item];
            const unit = priceOf(p);
            return (
              <tr key={i.item}>
                <td>
                  {name(i.item)}
                  {i.recipe && <span className="tag" title="This material can be crafted too">craftable</span>}
                  {unit == null && <span className="tag warn">no price</span>}
                </td>
                <td className="r">{fmtWhole(i.qty)}</td>
                <td className="r">{kinah(unit)}</td>
                <td className="r">{unit == null ? "—" : fmtWhole(unit * i.qty)}</td>
                <td className="r dim">{p?.listings || "—"}</td>
                <td><Spark points={trends?.[i.item]} loading={!trends} /></td>
              </tr>
            );
          })}
          {recipe.fee > 0 && (
            <tr>
              <td className="dim">Crafting fee</td>
              <td />
              <td />
              <td className="r">{fmtWhole(recipe.fee)}</td>
              <td />
              <td />
            </tr>
          )}
          <tr className="market-result">
            <td>
              Makes {name(recipe.output)}
              {recipe.outputQty > 1 && ` ×${recipe.outputQty}`}
            </td>
            <td />
            <td className="r">{kinah(row.unitPrice)}</td>
            <td className="r">{row.unitPrice == null ? "—" : fmtWhole(row.unitPrice * recipe.outputQty)}</td>
            <td className="r dim">{row.listings || "—"}</td>
            <td><Spark points={trends?.[recipe.output]} loading={!trends} /></td>
          </tr>
        </tbody>
      </table>
      {recipe.combo && (
        <p className="dim small">
          A lucky (combo) craft makes <b>{name(recipe.combo)}</b> instead
          {combo != null ? `, listed at ${fmtWhole(combo)}` : ", which has no price on this server"}. Profit above counts the
          normal result.
        </p>
      )}
    </div>
  );
}

function Spark({ points, loading }: { points?: number[]; loading: boolean }) {
  if (!points || points.length < 2) return <span className="dim small">{loading ? "…" : "—"}</span>;
  const w = 90, h = 20;
  const min = Math.min(...points), max = Math.max(...points);
  const span = max - min || 1;
  const d = points.map((p, i) => `${(i / (points.length - 1)) * w},${h - 2 - ((p - min) / span) * (h - 4)}`).join(" ");
  const up = points[points.length - 1] >= points[0];
  return (
    <svg className={`spark ${up ? "up" : "down"}`} width={w} height={h} viewBox={`0 0 ${w} ${h}`} aria-label={`${fmtWhole(points[0])} to ${fmtWhole(points[points.length - 1])}`}>
      <title>{`${fmtWhole(min)} – ${fmtWhole(max)} over 7 days`}</title>
      <polyline points={d} fill="none" strokeWidth="1.5" strokeLinejoin="round" />
    </svg>
  );
}
