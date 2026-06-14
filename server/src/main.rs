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
//!   POST /api/chat         -> Aomi agent (Claude + 7 tools over the engine)
//!   POST /api/conditional  -> conditional / player-prop markets

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

/// Conditional / player-prop markets, e.g. "Portugal to score if Ronaldo is
/// subbed off". Builds a MatchSetup the same way as simulate_body(), loads
/// both rosters via data::players_for(), parses `condition`/`outcome` into
/// sim::EventCondition / sim::EventOutcome, and runs simulate_conditional().
pub(crate) fn conditional(body: &str) -> Result<Value, String> {
    let v: Value = serde_json::from_str(body).map_err(|e| format!("bad json: {e}"))?;

    let home_name = v.get("home").and_then(|x| x.as_str()).ok_or("missing home")?;
    let away_name = v.get("away").and_then(|x| x.as_str()).ok_or("missing away")?;

    let home = data::team_strength(home_name)?;
    let away = data::team_strength(away_name)?;

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

    let condition: sim::EventCondition = serde_json::from_value(
        v.get("condition").cloned().ok_or("missing condition")?,
    )
    .map_err(|e| format!("bad condition: {e}"))?;

    let outcome: sim::EventOutcome = serde_json::from_value(
        v.get("outcome").cloned().ok_or("missing outcome")?,
    )
    .map_err(|e| format!("bad outcome: {e}"))?;

    let home_players = data::players_for(home_name);
    let away_players = data::players_for(away_name);

    let result = sim::simulate_conditional(&setup, &home_players, &away_players, &condition, &outcome);
    Ok(serde_json::to_value(result).unwrap())
}

// ─── tiny HTTP plumbing ──────────────────────────────────────────────────────

pub(crate) struct Query(pub(crate) Vec<(String, String)>);
impl Query {
    fn parse(url: &str) -> Self {
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
                (Method::Post, "/api/chat") => {
                    let mut body = String::new();
                    let _ = req.as_reader().read_to_string(&mut body);
                    chat::chat(&body)
                }
                (Method::Post, "/api/conditional") => {
                    let mut body = String::new();
                    let _ = req.as_reader().read_to_string(&mut body);
                    conditional(&body)
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
