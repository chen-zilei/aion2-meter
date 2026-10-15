//! Crafting recipes and market prices for the Market page.
//!
//! Recipes come from gamers4.life's crafting graph. Like the name tables in [`crate::gamedata`], that is NCSOFT's game
//! data, so it is never shipped here: the app downloads it into its data folder on first use and from the page's
//! refresh button. Prices come from aion2.exchange, which takes an hourly snapshot of each server's market. Both are
//! hobby projects, so prices are fetched only when the page asks, and kept for [`PRICE_TTL`] per server.

use anyhow::Context;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub const RECIPES_URL: &str = "https://gamers4.life/aion-2/database/crafting-graph.json";
const API: &str = "https://aion2.exchange/api";
/// Snapshots are hourly; this keeps a busy page to a handful of requests an hour.
const PRICE_TTL: Duration = Duration::from_secs(10 * 60);
/// Even a refresh pressed by hand reuses prices this fresh.
const MIN_REFRESH: Duration = Duration::from_secs(60);
/// The API pages its item list at most this many at a time.
const PAGE: usize = 1000;

/// Recent prices per item id, oldest first.
pub type Trends = HashMap<String, Vec<f64>>;

pub struct Market {
    dir: PathBuf,
    agent: ureq::Agent,
    recipes: Mutex<Option<RecipeBook>>,
    /// Held while the recipe file is downloaded, so two callers never write it at once.
    downloading: Mutex<()>,
    servers: Mutex<Option<ServerList>>,
    prices: Mutex<HashMap<u32, (Instant, PriceBook)>>,
    trends: Mutex<HashMap<(u32, String), (Instant, Trends)>>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RecipeBook {
    recipes: Vec<Recipe>,
    /// Every item a recipe uses or makes, keyed by item id.
    items: HashMap<String, ItemInfo>,
    /// When the file on disk was downloaded, in ms.
    downloaded_at: u64,
    source: &'static str,
    folder: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Recipe {
    id: String,
    profession: String,
    /// "light" (Elyos) or "dark" (Asmodian): each faction has its own copy of most recipes.
    side: String,
    level: u32,
    /// Kinah the crafting station charges.
    fee: f64,
    inputs: Vec<Input>,
    output: String,
    output_qty: u32,
    /// What a lucky craft makes instead of `output`.
    combo: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Input {
    item: String,
    qty: u32,
    /// The recipe that makes this material, when it can be crafted.
    recipe: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ItemInfo {
    name: String,
    grade: u32,
    category: String,
}

/// The parts of crafting-graph.json the page uses; everything else in the file is ignored.
#[derive(Deserialize)]
struct Graph {
    items: HashMap<String, RawItem>,
    recipes: HashMap<String, RawRecipe>,
}

#[derive(Deserialize)]
struct RawItem {
    #[serde(default)]
    n: String,
    #[serde(default)]
    g: u32,
    #[serde(default)]
    mc: String,
}

#[derive(Deserialize)]
struct RawRecipe {
    #[serde(default)]
    p: String,
    #[serde(default)]
    qr: String,
    #[serde(default)]
    ml: u32,
    #[serde(default)]
    gold: f64,
    #[serde(default, rename = "in")]
    inputs: Vec<(String, u32, Option<String>)>,
    out: String,
    #[serde(default = "one")]
    outq: u32,
    #[serde(default)]
    combo: Option<String>,
}

fn one() -> u32 {
    1
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct ServerList {
    default: u32,
    servers: Vec<Server>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct Server {
    server_id: u32,
    name: String,
    #[serde(default)]
    region: String,
    #[serde(default)]
    region_name: String,
    #[serde(default)]
    faction: String,
    #[serde(default)]
    has_data: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PriceBook {
    server: u32,
    /// When this app fetched the prices, in ms.
    fetched_at: u64,
    /// When aion2.exchange took the newest snapshot among them (ISO 8601).
    captured_at: Option<String>,
    /// Keyed by item id. Items the site doesn't track are missing.
    prices: HashMap<String, Price>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Price {
    /// Cheapest listing; `None` when nothing is listed right now.
    price: Option<f64>,
    listings: u32,
    /// Sales in the last 24 hours; `None` on servers whose sales the site doesn't track.
    units_sold: Option<f64>,
    sold_avg_price: Option<f64>,
    change_pct: Option<f64>,
}

#[derive(Deserialize)]
struct ItemPage {
    total: usize,
    items: Vec<ApiItem>,
}

#[derive(Deserialize)]
struct ApiItem {
    item_key: u64,
    price: Option<f64>,
    #[serde(default)]
    listings: Option<u32>,
    captured_at: Option<String>,
    units_sold: Option<f64>,
    sold_avg_price: Option<f64>,
    change_pct: Option<f64>,
}

#[derive(Deserialize)]
struct Sparklines {
    sparklines: Vec<Sparkline>,
}

#[derive(Deserialize)]
struct Sparkline {
    item_key: u64,
    points: Vec<SparkPoint>,
}

#[derive(Deserialize)]
struct SparkPoint {
    price: Option<f64>,
}

impl Market {
    pub fn new(dir: PathBuf) -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(20))
            .user_agent(&format!("aion2-meter/{}", env!("CARGO_PKG_VERSION")))
            .build();
        Self {
            dir,
            agent,
            recipes: Mutex::new(None),
            downloading: Mutex::new(()),
            servers: Mutex::new(None),
            prices: Mutex::new(HashMap::new()),
            trends: Mutex::new(HashMap::new()),
        }
    }

    fn recipe_path(&self) -> PathBuf {
        self.dir.join("crafting-graph.json")
    }

    /// The recipes, downloading them first when there are none on disk yet or `refresh` is set.
    pub fn recipes(&self, refresh: bool) -> anyhow::Result<RecipeBook> {
        let _guard = self.downloading.lock();
        if !refresh {
            if let Some(book) = self.recipes.lock().clone() {
                return Ok(book);
            }
        }
        let path = self.recipe_path();
        if refresh || !path.exists() {
            self.download_recipes()?;
        }
        let book = load_recipes(&path)?;
        *self.recipes.lock() = Some(book.clone());
        Ok(book)
    }

    fn download_recipes(&self) -> anyhow::Result<()> {
        let body = self
            .agent
            .get(RECIPES_URL)
            .call()
            .map_err(describe)?
            .into_string()?;
        // Refuse anything that isn't the graph rather than overwrite a good file with an error page.
        let parsed: serde_json::Value =
            serde_json::from_str(&body).context("the recipe file is not JSON")?;
        anyhow::ensure!(
            parsed.get("items").is_some_and(|v| v.is_object())
                && parsed.get("recipes").is_some_and(|v| v.is_object()),
            "the recipe file has no items or recipes"
        );
        std::fs::create_dir_all(&self.dir)?;
        let path = self.recipe_path();
        let tmp = path.with_extension("json.part");
        std::fs::write(&tmp, body)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    pub fn servers(&self) -> anyhow::Result<ServerList> {
        if let Some(list) = self.servers.lock().clone() {
            return Ok(list);
        }
        let list: ServerList = json(self.agent.get(&format!("{API}/servers")).call())?;
        *self.servers.lock() = Some(list.clone());
        Ok(list)
    }

    /// Every price the site has for a server: its whole list fits in a page or two, fewer calls than asking by id.
    pub fn prices(&self, server: u32, refresh: bool) -> anyhow::Result<PriceBook> {
        if let Some((at, book)) = self.prices.lock().get(&server) {
            let ttl = if refresh { MIN_REFRESH } else { PRICE_TTL };
            if at.elapsed() < ttl {
                return Ok(book.clone());
            }
        }
        let mut prices = HashMap::new();
        let mut captured_at: Option<String> = None;
        let mut offset = 0;
        loop {
            let page: ItemPage = json(
                self.agent
                    .get(&format!("{API}/items"))
                    .query("server", &server.to_string())
                    .query("window", "24h")
                    .query("sort", "key")
                    .query("limit", &PAGE.to_string())
                    .query("offset", &offset.to_string())
                    .call(),
            )?;
            let got = page.items.len();
            for item in page.items {
                if item.captured_at > captured_at {
                    captured_at.clone_from(&item.captured_at);
                }
                prices.insert(
                    item.item_key.to_string(),
                    Price {
                        price: item.price,
                        listings: item.listings.unwrap_or(0),
                        units_sold: item.units_sold,
                        sold_avg_price: item.sold_avg_price,
                        change_pct: item.change_pct,
                    },
                );
            }
            offset += got;
            // Ten pages is far more than any server lists; the cap only guards against a runaway loop.
            if got == 0 || offset >= page.total || offset >= PAGE * 10 {
                break;
            }
        }
        let book = PriceBook {
            server,
            fetched_at: crate::now_ms(),
            captured_at,
            prices,
        };
        self.prices
            .lock()
            .insert(server, (Instant::now(), book.clone()));
        Ok(book)
    }

    /// Seven days of prices for up to 100 items, oldest first, in 4-hour steps.
    pub fn trends(&self, server: u32, mut keys: Vec<String>) -> anyhow::Result<Trends> {
        keys.retain(|k| !k.is_empty() && k.bytes().all(|b| b.is_ascii_digit()));
        keys.sort();
        keys.dedup();
        keys.truncate(100);
        if keys.is_empty() {
            return Ok(HashMap::new());
        }
        let joined = keys.join(",");
        let cache_key = (server, joined.clone());
        if let Some((at, lines)) = self.trends.lock().get(&cache_key) {
            if at.elapsed() < PRICE_TTL {
                return Ok(lines.clone());
            }
        }
        let resp: Sparklines = json(
            self.agent
                .get(&format!("{API}/sparklines"))
                .query("server", &server.to_string())
                .query("window", "7d")
                .query("keys", &joined)
                .call(),
        )?;
        let lines: Trends = resp
            .sparklines
            .into_iter()
            .map(|s| {
                (
                    s.item_key.to_string(),
                    s.points.into_iter().filter_map(|p| p.price).collect(),
                )
            })
            .collect();
        self.trends
            .lock()
            .insert(cache_key, (Instant::now(), lines.clone()));
        Ok(lines)
    }
}

fn load_recipes(path: &PathBuf) -> anyhow::Result<RecipeBook> {
    let text = std::fs::read_to_string(path).context("reading the recipe file")?;
    let downloaded_at = std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_millis() as u64);
    let mut book = parse_recipes(&text)?;
    book.downloaded_at = downloaded_at;
    book.folder = path
        .parent()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    Ok(book)
}

fn parse_recipes(text: &str) -> anyhow::Result<RecipeBook> {
    let graph: Graph =
        serde_json::from_str(text).context("the recipe file is not in the expected shape")?;
    let mut recipes: Vec<Recipe> = graph
        .recipes
        .into_iter()
        // Some entries list no materials; there is nothing to price for those.
        .filter(|(_, r)| !r.inputs.is_empty())
        .map(|(id, r)| Recipe {
            id,
            profession: r.p,
            side: r.qr,
            level: r.ml,
            fee: r.gold,
            inputs: r
                .inputs
                .into_iter()
                .map(|(item, qty, recipe)| Input { item, qty, recipe })
                .collect(),
            output: r.out,
            output_qty: r.outq.max(1),
            combo: r.combo,
        })
        .collect();
    recipes.sort_by(|a, b| a.id.cmp(&b.id));
    let mut items = HashMap::new();
    for r in &recipes {
        for id in std::iter::once(&r.output)
            .chain(&r.combo)
            .chain(r.inputs.iter().map(|i| &i.item))
        {
            if let Some(raw) = graph.items.get(id) {
                items.entry(id.clone()).or_insert_with(|| ItemInfo {
                    name: raw.n.clone(),
                    grade: raw.g,
                    category: raw.mc.clone(),
                });
            }
        }
    }
    Ok(RecipeBook {
        recipes,
        items,
        downloaded_at: 0,
        source: RECIPES_URL,
        folder: String::new(),
    })
}

/// Reads an aion2.exchange answer.
fn json<T: serde::de::DeserializeOwned>(
    resp: Result<ureq::Response, ureq::Error>,
) -> anyhow::Result<T> {
    let body = resp.map_err(describe)?.into_string()?;
    serde_json::from_str(&body).context("aion2.exchange sent something this app can't read")
}

/// Turns a failed request into something a player can act on.
fn describe(e: ureq::Error) -> anyhow::Error {
    match e {
        ureq::Error::Status(code, resp) => anyhow::anyhow!(
            "{} answered with error {code}",
            resp.get_url().split('/').nth(2).unwrap_or("the site")
        ),
        ureq::Error::Transport(t) => {
            anyhow::anyhow!("couldn't reach the site; are you offline? ({t})")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_graph_and_keeps_only_recipes_with_materials() {
        let text = r#"{
            "items": {
                "1": {"n": "Sword", "g": 41, "sc": "sword", "mc": "weapon", "ic": "x"},
                "2": {"n": "Ore", "g": 11, "sc": "ore", "mc": "material"},
                "3": {"n": "Shiny Sword", "g": 51, "mc": "weapon"}
            },
            "recipes": {
                "10": {"p": "blacksmithing", "qr": "dark", "ml": 5, "gold": 300, "in": [["2", 4, "20"]], "out": "1", "outq": 1, "cp": 0, "combo": "3"},
                "11": {"p": "blacksmithing", "qr": "dark", "ml": 5, "gold": 0, "in": [], "out": "1", "outq": 1, "combo": null}
            },
            "byProduct": {}
        }"#;
        let book = parse_recipes(text).unwrap();
        assert_eq!(book.recipes.len(), 1);
        let r = &book.recipes[0];
        assert_eq!(
            (r.output.as_str(), r.combo.as_deref(), r.fee),
            ("1", Some("3"), 300.0)
        );
        assert_eq!(
            (r.inputs[0].qty, r.inputs[0].recipe.as_deref()),
            (4, Some("20"))
        );
        assert_eq!(book.items.len(), 3);
        assert_eq!(book.items["2"].name, "Ore");
    }

    #[test]
    fn refuses_something_else() {
        assert!(parse_recipes("<html>Not found</html>").is_err());
        assert!(parse_recipes(r#"{"error": "rate limited"}"#).is_err());
    }
}
