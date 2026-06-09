//! Data layer.
//!
//! Two sources, by cadence:
//!   - Team strength (Elo + xG) is slow-moving: bundled from `data/teams.json`,
//!     produced daily by the Python prep step (soccerdata -> FBref + Elo).
//!   - Match state (fixtures, injuries, lineups) is live: API-FOOTBALL at runtime,
//!     gated on the API_FOOTBALL_KEY env var.
//!   - Market prices: Polymarket Gamma API, public, no key.

use crate::sim::TeamStrength;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::LazyLock;

const TEAMS_JSON: &str = include_str!("../data/teams.json");
const GAMMA_BASE: &str = "https://gamma-api.polymarket.com";
const APIFOOTBALL_BASE: &str = "https://v3.football.api-sports.io";
/// World Cup league id and season in API-FOOTBALL.
const WC_LEAGUE_ID: u32 = 1;
const WC_SEASON: u32 = 2026;

#[derive(Clone, Debug, Deserialize)]
struct TeamRow {
    name: String,
    elo: f64,
    xg_for: f64,
    xg_against: f64,
    #[serde(default)]
    aliases: Vec<String>,
}

static TEAMS: LazyLock<HashMap<String, TeamRow>> = LazyLock::new(|| {
    let rows: Vec<TeamRow> = serde_json::from_str(TEAMS_JSON).unwrap_or_default();
    let mut map = HashMap::new();
    for row in rows {
        map.insert(norm(&row.name), row.clone());
        for a in &row.aliases {
            map.insert(norm(a), row.clone());
        }
    }
    map
});

fn norm(s: &str) -> String {
    s.trim().to_lowercase()
}

/// Look up a team's bundled strength by name or alias.
pub fn team_strength(name: &str) -> Result<TeamStrength, String> {
    TEAMS
        .get(&norm(name))
        .map(|r| TeamStrength {
            name: r.name.clone(),
            elo: r.elo,
            xg_for: r.xg_for,
            xg_against: r.xg_against,
        })
        .ok_or_else(|| format!("[goal-digger] unknown team '{name}'. Check data/teams.json coverage."))
}

/// Every team we have strength data for (for tournament sims and listing).
#[allow(dead_code)]
pub fn all_team_names() -> Vec<String> {
    let mut names: Vec<String> = TEAMS.values().map(|r| r.name.clone()).collect();
    names.sort();
    names.dedup();
    names
}

// ─── API-FOOTBALL (live, key-gated) ──────────────────────────────────────────

fn api_football_key() -> Result<String, String> {
    std::env::var("API_FOOTBALL_KEY").map_err(|_| {
        "[goal-digger] API_FOOTBALL_KEY not set. Live fixtures/injuries unavailable; \
         strength-only simulation still works."
            .to_string()
    })
}

fn af_get(path: &str, query: &[(&str, String)]) -> Result<Value, String> {
    let key = api_football_key()?;
    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(format!("{APIFOOTBALL_BASE}{path}"))
        .header("x-apisports-key", key)
        .query(query)
        .send()
        .map_err(|e| format!("[goal-digger] api-football request failed: {e}"))?
        .json::<Value>()
        .map_err(|e| format!("[goal-digger] api-football parse failed: {e}"))?;
    Ok(resp)
}

pub fn fixtures(date: Option<String>) -> Result<Value, String> {
    let mut q = vec![
        ("league", WC_LEAGUE_ID.to_string()),
        ("season", WC_SEASON.to_string()),
    ];
    if let Some(d) = date {
        q.push(("date", d));
    }
    af_get("/fixtures", &q)
}

pub fn injuries(team_id: u32) -> Result<Value, String> {
    af_get(
        "/injuries",
        &[
            ("league", WC_LEAGUE_ID.to_string()),
            ("season", WC_SEASON.to_string()),
            ("team", team_id.to_string()),
        ],
    )
}

// ─── Polymarket Gamma (public, no key) ───────────────────────────────────────

/// Fetch a market by slug and return its outcomes with current prices.
pub fn gamma_market(slug: &str) -> Result<Value, String> {
    let client = reqwest::blocking::Client::new();
    let arr = client
        .get(format!("{GAMMA_BASE}/markets"))
        .query(&[("slug", slug)])
        .send()
        .map_err(|e| format!("[goal-digger] gamma request failed: {e}"))?
        .json::<Value>()
        .map_err(|e| format!("[goal-digger] gamma parse failed: {e}"))?;

    let market = arr
        .as_array()
        .and_then(|a| a.first())
        .cloned()
        .ok_or_else(|| format!("[goal-digger] no Polymarket market for slug '{slug}'"))?;

    let outcomes = parse_str_array(market.get("outcomes"));
    let prices = parse_str_array(market.get("outcomePrices"));
    let pairs: Vec<Value> = outcomes
        .iter()
        .zip(prices.iter())
        .map(|(o, p)| json!({ "outcome": o, "price": p.parse::<f64>().unwrap_or(0.0) }))
        .collect();

    Ok(json!({
        "slug": slug,
        "question": market.get("question"),
        "outcomes": pairs,
        "clobTokenIds": parse_str_array(market.get("clobTokenIds")),
    }))
}

/// Gamma returns `outcomes`/`outcomePrices`/`clobTokenIds` as stringified JSON arrays.
fn parse_str_array(v: Option<&Value>) -> Vec<String> {
    match v {
        Some(Value::String(s)) => serde_json::from_str::<Vec<String>>(s).unwrap_or_default(),
        Some(Value::Array(a)) => a
            .iter()
            .map(|x| x.as_str().unwrap_or_default().to_string())
            .collect(),
        _ => Vec::new(),
    }
}
