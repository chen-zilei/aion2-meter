import { invoke } from "@tauri-apps/api/core";

/** Mirrors the types in src-tauri/src/market.rs. */
export interface RecipeBook {
  recipes: Recipe[];
  items: Record<string, ItemInfo>;
  downloadedAt: number;
  source: string;
  folder: string;
}

export interface Recipe {
  id: string;
  profession: string;
  /** "light" (Elyos) or "dark" (Asmodian). */
  side: string;
  level: number;
  fee: number;
  inputs: { item: string; qty: number; recipe: string | null }[];
  output: string;
  outputQty: number;
  combo: string | null;
}

export interface ItemInfo {
  name: string;
  grade: number;
  category: string;
}

export interface ServerList {
  default: number;
  servers: Server[];
}

export interface Server {
  serverId: number;
  name: string;
  region: string;
  regionName: string;
  faction: string;
  hasData: boolean;
}

export interface PriceBook {
  server: number;
  fetchedAt: number;
  capturedAt: string | null;
  prices: Record<string, Price>;
}

export interface Price {
  price: number | null;
  listings: number;
  unitsSold: number | null;
  soldAvgPrice: number | null;
  changePct: number | null;
}

export const marketApi = {
  recipes: (refresh: boolean) => invoke<RecipeBook>("market_recipes", { refresh }),
  servers: () => invoke<ServerList>("market_servers"),
  prices: (server: number, refresh: boolean) => invoke<PriceBook>("market_prices", { server, refresh }),
  openSource: (which: "prices" | "recipes") => invoke<void>("market_open_source", { which }).catch(() => {}),
  trends: (server: number, keys: string[]) => invoke<Record<string, number[]>>("market_trends", { server, keys }),
};
