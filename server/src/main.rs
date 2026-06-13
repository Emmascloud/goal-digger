//! Goal Digger demo server.
//!
//! Reuses the EXACT plugin engine (path-included from ../../app/src) and serves the
//! UI same-origin so the React dashboard fetches real Dixon-Coles + Monte-Carlo
//! numbers with no CORS, no AomiFrame, nothing heavy.
//!
//!   GET  /                 -> the camel-theme UI
//!   GET  /api/board        -> all 8 fixtures priced by the real engine
//!   POST /api/simulate     -> one match: {home, away, neutral, knockout, *_adj}
//!   GET  /api/edge         -> model vs live Polymarket price (Gamma) ?slug&outcome&model_prob
//!   GET  /api/tournament   -> title probabilities ?teams=A,B,C,...
//!   POST /api/chat         -> agentic Claude loop with 6 engine tools (requires ANTHROPIC_API_KEY)

#[path = "../../app/src/sim.rs"]
mod sim;
#[path = "../../app/src/data.rs"]
mod data;
mod chat;

use serde_json::{json, Value};
use sim::{Adjustments, MatchSetup};
use std::io::Read;
use std::path::{Path, PathBuf};
use tiny_http::{Header, Method, Response, Server};

const ADDR: &str = "127.0.0.1:8787";

/// Cached live-board snapshot, refreshed in the background so the endpoint is instant
/// no matter how slow the underlying API calls are.
static BOARD: std::sync::LazyLock<std::sync::Mutex<Value>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(json!({ "matches": [], "warming": true })));

/// One board fixture with the engine inputs that mirror the UI's RAW_MATCHES.
struct Fixture {
    id: &'static str,
    home: &'static str,
    away: &'static str,
    neutral: bool,
    host_elo_bonus: f64,
    // net adjustment multipliers translated from the UI's stated reasons
    ha: f64,
    hd: f64,
    aa: f64,
    ad: f64,
}

fn fixtures() -> Vec<Fixture> {
    vec![
        // Spain: keeper-out (atk x1.10) + crowd (x1.04); Germany: suspensions (atk x0.94)
        Fixture { id: "esp-ger", home: "Spain", away: "Germany", neutral: true, host_elo_bonus: 0.0, ha: 1.144, hd: 1.0, aa: 0.94, ad: 1.0 },
        // Argentina captain knock (atk x0.97); Mexico low fatigue (def x1.05)
        Fixture { id: "arg-mex", home: "Argentina", away: "Mexico", neutral: true, host_elo_bonus: 0.0, ha: 0.97, hd: 1.0, aa: 1.0, ad: 1.05 },
        // England winger returns (atk x1.08); France left-back doubtful (def x0.96)
        Fixture { id: "fra-eng", home: "France", away: "England", neutral: true, host_elo_bonus: 0.0, ha: 1.0, hd: 0.96, aa: 1.08, ad: 1.0 },
        // Brazil playmaker back (atk x1.07), humid risk (def x0.97)
        Fixture { id: "bra-ned", home: "Brazil", away: "Netherlands", neutral: true, host_elo_bonus: 0.0, ha: 1.07, hd: 0.97, aa: 1.0, ad: 1.0 },
        // Uruguay CB suspended (def x0.93)
        Fixture { id: "por-uru", home: "Portugal", away: "Uruguay", neutral: true, host_elo_bonus: 0.0, ha: 1.0, hd: 1.0, aa: 1.0, ad: 0.93 },
        // Croatia tired legs (atk x0.95); Morocco strong press (def x1.06)
        Fixture { id: "cro-mar", home: "Croatia", away: "Morocco", neutral: true, host_elo_bonus: 0.0, ha: 0.95, hd: 1.0, aa: 1.0, ad: 1.06 },
        // USA host nation (home + crowd atk x1.06)
        Fixture { id: "usa-col", home: "USA", away: "Colombia", neutral: false, host_elo_bonus: 40.0, ha: 1.06, hd: 1.0, aa: 1.0, ad: 1.0 },
        // Senegal physical edge (atk x1.05)
        Fixture { id: "jpn-sen", home: "Japan", away: "Senegal", neutral: true, host_elo_bonus: 0.0, ha: 1.0, hd: 1.0, aa: 1.05, ad: 1.0 },
    ]
}

fn price_fixture(f: &Fixture) -> Result<Value, String> {
    let home = data::team_strength(f.home)?;
    let away = data::team_strength(f.away)?;
    let setup = MatchSetup {
        home,
        away,
        neutral: f.neutral,
        host_elo_bonus: f.host_elo_bonus,
        knockout: true,
        home_adj: Adjustments { attack: f.ha, defense: f.hd },
        away_adj: Adjustments { attack: f.aa, defense: f.ad },
        seed: Some(0x60A1_D16E),
        sims: Some(50_000),
    };
    let o = sim::simulate(&setup);
    Ok(json!({
        "id": f.id,
        "lambda_home": o.lambda_home,
        "lambda_away": o.lambda_away,
        "p_home_win": o.p_home_win,
        "p_draw": o.p_draw,
        "p_away_win": o.p_away_win,
        "p_home_advance": o.p_home_advance,
        "p_over_2_5": o.p_over_2_5,
        "p_btts": o.p_btts,
        "top_scorelines": o.top_scorelines,
    }))
}

// ─── Polymarket outright slugs for all 16 fixture teams ──────────────────────

const SLUG_MAP: &[(&str, &str)] = &[
    ("Spain",       "will-spain-win-the-2026-fifa-world-cup-963"),
    ("Germany",     "will-germany-win-the-2026-fifa-world-cup-467"),
    ("Argentina",   "will-argentina-win-the-2026-fifa-world-cup-245"),
    ("Mexico",      "will-mexico-win-the-2026-fifa-world-cup-529"),
    ("France",      "will-france-win-the-2026-fifa-world-cup-924"),
    ("England",     "will-england-win-the-2026-fifa-world-cup-937"),
    ("Brazil",      "will-brazil-win-the-2026-fifa-world-cup-183"),
    ("Netherlands", "will-netherlands-win-the-2026-fifa-world-cup-739"),
    ("Portugal",    "will-portugal-win-the-2026-fifa-world-cup-912"),
    ("Uruguay",     "will-uruguay-win-the-2026-fifa-world-cup-932"),
    ("Croatia",     "will-croatia-win-the-2026-fifa-world-cup"),
    ("Morocco",     "will-morocco-win-the-2026-fifa-world-cup-464"),
    ("USA",         "will-usa-win-the-2026-fifa-world-cup-467"),
    ("Colombia",    "will-colombia-win-the-2026-fifa-world-cup-734"),
    ("Japan",       "will-japan-win-the-2026-fifa-world-cup-112"),
    ("Senegal",     "will-senegal-win-the-2026-fifa-world-cup"),
];

/// GET /api/prices — live Polymarket outright prices for all fixture teams.
fn prices() -> Value {
    let outrights: Vec<Value> = SLUG_MAP
        .iter()
        .filter_map(|(team, slug)| {
            let market = data::gamma_market(slug).ok()?;
            let price = market
                .get("outcomes")
                .and_then(|o| o.as_array())
                .and_then(|arr| {
                    arr.iter().find(|r| {
                        r.get("outcome")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_lowercase() == "yes")
                            .unwrap_or(false)
                    })
                })
                .and_then(|r| r.get("price").and_then(|p| p.as_f64()))?;
            Some(json!({ "team": team, "slug": slug, "market_price": price }))
        })
        .collect();
    json!({ "outrights": outrights })
}

pub(crate) fn board() -> Value {
    let rows: Vec<Value> = fixtures()
        .iter()
        .filter_map(|f| price_fixture(f).ok())
        .collect();
    json!({ "source": "goal-digger-engine", "model": "dixon-coles+elo+xg/monte-carlo", "matches": rows })
}

pub(crate) fn simulate_body(body: &str) -> Result<Value, String> {
    let v: Value = serde_json::from_str(body).map_err(|e| format!("bad json: {e}"))?;
    let home = data::team_strength(v.get("home").and_then(|x| x.as_str()).unwrap_or(""))?;
    let away = data::team_strength(v.get("away").and_then(|x| x.as_str()).unwrap_or(""))?;
    let g = |k: &str, d: f64| v.get(k).and_then(|x| x.as_f64()).unwrap_or(d);
    let setup = MatchSetup {
        home,
        away,
        neutral: v.get("neutral").and_then(|x| x.as_bool()).unwrap_or(true),
        host_elo_bonus: g("host_elo_bonus", 0.0),
        knockout: v.get("knockout").and_then(|x| x.as_bool()).unwrap_or(false),
        home_adj: Adjustments { attack: g("home_attack_adj", 1.0), defense: g("home_defense_adj", 1.0) },
        away_adj: Adjustments { attack: g("away_attack_adj", 1.0), defense: g("away_defense_adj", 1.0) },
        seed: v.get("seed").and_then(|x| x.as_u64()),
        sims: v.get("sims").and_then(|x| x.as_u64()).map(|n| n as usize),
    };
    Ok(serde_json::to_value(sim::simulate(&setup)).unwrap())
}

pub(crate) fn edge(q: &Query) -> Result<Value, String> {
    let slug = q.get("slug").ok_or("missing slug")?;
    let outcome = q.get("outcome").ok_or("missing outcome")?;
    let model_prob: f64 = q.get("model_prob").and_then(|s| s.parse().ok()).ok_or("missing model_prob")?;
    let market = data::gamma_market(slug)?;
    let want = outcome.trim().to_lowercase();
    let price = market
        .get("outcomes")
        .and_then(|o| o.as_array())
        .and_then(|arr| arr.iter().find(|r| r.get("outcome").and_then(|v| v.as_str()).map(|s| s.trim().to_lowercase() == want).unwrap_or(false)))
        .and_then(|r| r.get("price").and_then(|p| p.as_f64()))
        .ok_or_else(|| format!("outcome '{outcome}' not found"))?;
    let e = model_prob - price;
    Ok(json!({ "slug": slug, "outcome": outcome, "market_price": price, "model_prob": model_prob,
        "edge": (e * 10000.0).round() / 10000.0, "verdict": if e >= 0.04 { "VALUE_BUY" } else if e <= -0.04 { "OVERPRICED" } else { "FAIR" } }))
}

pub(crate) fn tournament(q: &Query) -> Result<Value, String> {
    let names: Vec<String> = q.get("teams").ok_or("missing teams")?.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    let mut strengths = Vec::new();
    for n in &names {
        strengths.push(data::team_strength(n)?);
    }
    let champs = sim::simulate_tournament(&strengths, 20_000, 0x60A1)?;
    let table: Vec<Value> = champs.into_iter().map(|(t, p)| json!({ "team": t, "title_probability": p })).collect();
    Ok(json!({ "championship": table }))
}

/// Live lineup-driven adjustments for a real WC fixture (needs API_FOOTBALL_KEY).
/// Demonstrates the model reading the confirmed starting XI and adjusting itself.
pub(crate) fn live_adjust(q: &Query) -> Result<Value, String> {
    let home = q.get("home").ok_or("missing home")?;
    let away = q.get("away").ok_or("missing away")?;
    let fid = data::wc_fixture_id(home, away)
        .ok_or_else(|| format!("no WC fixture found for {home} vs {away}"))?;
    let (ha, hd, hr) = data::live_adjustment(home, fid);
    let (aa, ad, ar) = data::live_adjustment(away, fid);
    Ok(json!({
        "source": "api-football/lineups",
        "fixture_id": fid,
        "home": { "team": home, "attack_adj": ha, "defense_adj": hd, "reasons": hr },
        "away": { "team": away, "attack_adj": aa, "defense_adj": ad, "reasons": ar }
    }))
}

/// The live board: real in-play + upcoming WC fixtures, priced by the engine with
/// curated live-lineup adjustments, plus live scores. Needs API_FOOTBALL_KEY.
pub(crate) fn live_board() -> Value {
    if std::env::var("API_FOOTBALL_KEY").is_err() {
        return json!({ "error": "API_FOOTBALL_KEY not set", "matches": [] });
    }
    let mut fixtures: Vec<Value> = vec![];
    if let Ok(v) = data::wc_fixtures_live() {
        if let Some(a) = v.get("response").and_then(|r| r.as_array()) {
            fixtures.extend(a.iter().cloned());
        }
    }
    if let Ok(v) = data::wc_fixtures_next(12) {
        if let Some(a) = v.get("response").and_then(|r| r.as_array()) {
            fixtures.extend(a.iter().cloned());
        }
    }

    let is_host = |t: &str| matches!(data::team_code(t).as_str(), "USA" | "MEX" | "CAN");
    let mut seen = std::collections::HashSet::new();
    let mut matches = vec![];
    for f in fixtures {
        let fid = f["fixture"]["id"].as_u64().unwrap_or(0) as u32;
        if fid == 0 || !seen.insert(fid) {
            continue;
        }
        let home = f["teams"]["home"]["name"].as_str().unwrap_or("").to_string();
        let away = f["teams"]["away"]["name"].as_str().unwrap_or("").to_string();
        let (hs, as_) = match (data::team_strength(&home), data::team_strength(&away)) {
            (Ok(h), Ok(a)) => (h, a),
            _ => continue,
        };
        let (ha, hd, hr) = data::live_adjustment(&home, fid);
        let (aa, ad, ar) = data::live_adjustment(&away, fid);
        let setup = MatchSetup {
            home: hs,
            away: as_,
            neutral: !is_host(&home),
            host_elo_bonus: if is_host(&home) { 40.0 } else { 0.0 },
            knockout: false,
            home_adj: Adjustments { attack: ha, defense: hd },
            away_adj: Adjustments { attack: aa, defense: ad },
            seed: Some(fid as u64),
            sims: Some(50_000),
        };
        let o = sim::simulate(&setup);
        let mut adjustments = vec![];
        for r in &hr {
            adjustments.push(json!({ "team": home, "reason": r }));
        }
        for r in &ar {
            adjustments.push(json!({ "team": away, "reason": r }));
        }
        let mut row = json!({
            "id": format!("{}-{}", data::team_code(&home), data::team_code(&away)).to_lowercase(),
            "fixture_id": fid,
            "comp": f["league"]["round"].as_str().unwrap_or("Group Stage"),
            "kickoff": f["fixture"]["date"].as_str().unwrap_or(""),
            "status": f["fixture"]["status"]["short"].as_str().unwrap_or("NS"),
            "elapsed": f["fixture"]["status"]["elapsed"],
            "venue": f["fixture"]["venue"]["name"].as_str().unwrap_or(""),
            "score": { "home": f["goals"]["home"], "away": f["goals"]["away"] },
            "home": { "code": data::team_code(&home), "name": home.clone() },
            "away": { "code": data::team_code(&away), "name": away.clone() },
            "lambda_home": o.lambda_home, "lambda_away": o.lambda_away,
            "p_home_win": o.p_home_win, "p_draw": o.p_draw, "p_away_win": o.p_away_win,
            "p_home_advance": o.p_home_advance,
            "p_over_2_5": o.p_over_2_5, "p_btts": o.p_btts,
            "top_scorelines": o.top_scorelines,
            "adjustments": adjustments
        });
        // Real Polymarket 3-way prices for this match -> crowd + per-outcome edge.
        if let Some((ph, pd, pa)) = data::match_market(&home, &away) {
            let r2 = |x: f64| (x * 10000.0).round() / 10000.0;
            row["crowd"] = json!({ "home": ph, "draw": pd, "away": pa });
            row["edge"] = json!({
                "home": r2(o.p_home_win - ph),
                "draw": r2(o.p_draw - pd),
                "away": r2(o.p_away_win - pa)
            });
        }
        matches.push(row);
        if matches.len() >= 12 {
            break;
        }
    }
    json!({ "source": "goal-digger live (api-football + dixon-coles/monte-carlo)", "matches": matches })
}

// ─── tiny HTTP plumbing ──────────────────────────────────────────────────────

pub(crate) struct Query(Vec<(String, String)>);
impl Query {
    pub(crate) fn parse(url: &str) -> Self {
        let q = url.splitn(2, '?').nth(1).unwrap_or("");
        let pairs = q
            .split('&')
            .filter(|s| !s.is_empty())
            .map(|p| {
                let mut it = p.splitn(2, '=');
                (urldecode(it.next().unwrap_or("")), urldecode(it.next().unwrap_or("")))
            })
            .collect();
        Query(pairs)
    }
    fn get(&self, k: &str) -> Option<&str> {
        self.0.iter().find(|(key, _)| key == k).map(|(_, v)| v.as_str())
    }
}

pub(crate) fn urldecode_noop(s: &str) -> &str {
    s
}

fn urldecode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'%' if i + 2 < b.len() => {
                if let Ok(n) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                    out.push(n);
                    i += 3;
                    continue;
                }
                out.push(b[i]);
                i += 1;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn json_response(v: Value) -> Response<std::io::Cursor<Vec<u8>>> {
    let body = serde_json::to_vec(&v).unwrap();
    Response::from_data(body)
        .with_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
        .with_header(Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap())
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("jsx") => "text/babel; charset=utf-8",
        Some("png") => "image/png",
        Some("json") => "application/json",
        _ => "application/octet-stream",
    }
}

fn ui_dir() -> PathBuf {
    // server/ is a sibling of ui/
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("ui")
}

fn serve_static(url: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    let raw = url.splitn(2, '?').next().unwrap_or("/");
    let rel = urldecode(raw);
    let rel = if rel == "/" { "goal_digger2.html".to_string() } else { rel.trim_start_matches('/').to_string() };
    let path = ui_dir().join(&rel);
    // contain within ui/
    if !path.starts_with(ui_dir()) || !path.is_file() {
        return Response::from_string("not found").with_status_code(404);
    }
    let bytes = std::fs::read(&path).unwrap_or_default();
    Response::from_data(bytes).with_header(Header::from_bytes(&b"Content-Type"[..], content_type(&path).as_bytes()).unwrap())
}

fn main() {
    let server = Server::http(ADDR).expect("bind");
    println!("Goal Digger engine server on http://{ADDR}  (serving {})", ui_dir().display());
    // Compute the board in the background and refresh it every 60s. The endpoint
    // serves the latest snapshot instantly instead of blocking on ~60 API calls.
    if std::env::var("API_FOOTBALL_KEY").is_ok() {
        std::thread::spawn(|| loop {
            let v = live_board();
            let n = v.get("matches").and_then(|m| m.as_array()).map(|a| a.len()).unwrap_or(0);
            if let Ok(mut g) = BOARD.lock() {
                *g = v;
            }
            println!("[board] snapshot refreshed ({n} matches)");
            std::thread::sleep(std::time::Duration::from_secs(60));
        });
    }
    for mut req in server.incoming_requests() {
        let url = req.url().to_string();
        let path = url.splitn(2, '?').next().unwrap_or("/").to_string();
        let is_api = path.starts_with("/api/");

        if is_api {
            let q = Query::parse(&url);
            let result: Result<Value, String> = match (req.method(), path.as_str()) {
                (Method::Get, "/api/board") => Ok(board()),
                (Method::Post, "/api/simulate") => {
                    let mut body = String::new();
                    let _ = req.as_reader().read_to_string(&mut body);
                    simulate_body(&body)
                }
                (Method::Get, "/api/edge") => edge(&q),
                (Method::Get, "/api/tournament") => tournament(&q),
                (Method::Get, "/api/live-adjust") => live_adjust(&q),
                (Method::Get, "/api/live-board") => {
                    Ok(BOARD.lock().map(|g| g.clone()).unwrap_or_else(|_| json!({ "matches": [] })))
                }
                (Method::Get, "/api/prices") => Ok(prices()),
                (Method::Post, "/api/chat") => {
                    let mut body = String::new();
                    let _ = req.as_reader().read_to_string(&mut body);
                    chat::handle(&body)
                }
                _ => Err("unknown endpoint".into()),
            };
            let payload = match result {
                Ok(v) => v,
                Err(e) => json!({ "error": e }),
            };
            let _ = req.respond(json_response(payload));
        } else {
            let _ = req.respond(serve_static(&url));
        }
    }
}
