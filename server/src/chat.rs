//! /api/chat — synchronous Anthropic agentic loop for the tiny_http server.
//!
//! chat.rs was originally written for axum/async (Emma's starter backend).
//! This version is rewritten for tiny_http compatibility: blocking reqwest,
//! direct engine calls instead of HTTP re-entry (which would deadlock the
//! single-threaded server), same agentic loop logic.
//!
//! Set ANTHROPIC_API_KEY before running. Without it the endpoint returns a
//! clear JSON error; the rest of the server runs fine.

use reqwest::blocking::Client;
use serde_json::{json, Value};

// ─── Tool definitions ────────────────────────────────────────────────────────

fn tools() -> Value {
    json!([
        {
            "name": "get_board",
            "description": "Get all 8 World Cup fixtures priced by the live Dixon-Coles + Monte Carlo engine. Returns win/draw/loss probabilities, advance probability, over/under 2.5, BTTS, and top scorelines for each match.",
            "input_schema": { "type": "object", "properties": {}, "required": [] }
        },
        {
            "name": "simulate_match",
            "description": "Run 50,000 Monte Carlo simulations for a specific match. Returns the full probability distribution: win/draw/loss, knockout advance probability, over/under 2.5, BTTS, expected goals, and top scorelines. Pass *_adj multipliers (clamped 0.6–1.4) to encode injuries, suspensions, or fitness news.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "home": { "type": "string", "description": "Home team name" },
                    "away": { "type": "string", "description": "Away team name" },
                    "neutral": { "type": "boolean", "description": "Neutral venue? (default true)" },
                    "knockout": { "type": "boolean", "description": "Knockout stage — no draw allowed" },
                    "host_elo_bonus": { "type": "number", "description": "Extra Elo for a host-nation crowd (e.g. 40 for USA)" },
                    "home_attack_adj": { "type": "number", "description": "Home attack multiplier, 0.6–1.4" },
                    "home_defense_adj": { "type": "number", "description": "Home defense multiplier, 0.6–1.4" },
                    "away_attack_adj": { "type": "number", "description": "Away attack multiplier, 0.6–1.4" },
                    "away_defense_adj": { "type": "number", "description": "Away defense multiplier, 0.6–1.4" }
                },
                "required": ["home", "away"]
            }
        },
        {
            "name": "get_edge",
            "description": "Compare a model probability to the live Polymarket (Gamma) price for one market outcome. Returns the edge in probability points, a verdict (VALUE_BUY / FAIR / OVERPRICED), and a quarter-Kelly stake suggestion.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "slug": { "type": "string", "description": "Polymarket market slug, e.g. 'will-spain-win-the-2026-fifa-world-cup-963'" },
                    "outcome": { "type": "string", "description": "Outcome to check: 'Yes' for binary markets, or a team name" },
                    "model_prob": { "type": "number", "description": "Model probability (0–1) from simulate_match or simulate_tournament" }
                },
                "required": ["slug", "outcome", "model_prob"]
            }
        },
        {
            "name": "get_tournament_odds",
            "description": "Get 2026 World Cup tournament-winner probabilities for a list of teams, simulated through all remaining bracket fixtures.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "teams": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Team names to include in the bracket simulation"
                    }
                },
                "required": ["teams"]
            }
        },
        {
            "name": "find_best_edges",
            "description": "Compute ranked edges across ALL outright 2026 World Cup winner markets. Runs a full tournament simulation, fetches live Polymarket prices for every team, and returns a sorted list of value buys and overpriced markets. Use this as your first call when the user asks about value, edges, or where to bet. No need to call get_edge separately — edges are pre-computed and ranked.",
            "input_schema": { "type": "object", "properties": {}, "required": [] }
        },
        {
            "name": "explain_model",
            "description": "Simulate a match and explain in plain language what drove the expected-goals split — which adjustments were applied and why the probabilities landed where they did.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "home": { "type": "string" },
                    "away": { "type": "string" }
                },
                "required": ["home", "away"]
            }
        }
    ])
}

// ─── System prompt ────────────────────────────────────────────────────────────

fn system_prompt(context: Option<&Value>) -> String {
    let board_block = context
        .filter(|v| !v.is_null())
        .map(|c| format!(
            "\n\nLive board snapshot (from engine, loaded at session start):\n{}",
            serde_json::to_string_pretty(c).unwrap_or_default()
        ))
        .unwrap_or_default();

    format!(
        "You are GoalDigger, an AI trading analyst for the 2026 FIFA World Cup on \
         Polymarket. Your job: find where the crowd has a price wrong, explain why in \
         plain language, and help the user decide on a stake.\n\n\
         Rules:\n\
         1. Simulate, never guess. For any match question call simulate_match first.\n\
         2. Find the edge. Call get_edge with your model probability and the Polymarket \
            slug. Only call something a value bet when edge clears 4 points.\n\
         3. Lead with the number, then the reasoning. Crowd price → your number → gap \
            → suggested stake.\n\
         4. When a bet is fair or overpriced, say so plainly. Never force a trade.\n\
         5. Probabilities are estimates. Never promise a win.\n\
         6. Be concise. One sentence per fact. No filler.\n\n\
         Polymarket outright-winner slugs (outcome: \"Yes\" for all):\n\
         Spain       → will-spain-win-the-2026-fifa-world-cup-963\n\
         Germany     → will-germany-win-the-2026-fifa-world-cup-467\n\
         Argentina   → will-argentina-win-the-2026-fifa-world-cup-245\n\
         Mexico      → will-mexico-win-the-2026-fifa-world-cup-529\n\
         France      → will-france-win-the-2026-fifa-world-cup-924\n\
         England     → will-england-win-the-2026-fifa-world-cup-937\n\
         Brazil      → will-brazil-win-the-2026-fifa-world-cup-183\n\
         Netherlands → will-netherlands-win-the-2026-fifa-world-cup-739\n\
         Portugal    → will-portugal-win-the-2026-fifa-world-cup-912\n\
         Uruguay     → will-uruguay-win-the-2026-fifa-world-cup-932\n\
         Croatia     → will-croatia-win-the-2026-fifa-world-cup\n\
         Morocco     → will-morocco-win-the-2026-fifa-world-cup-464\n\
         USA         → will-usa-win-the-2026-fifa-world-cup-467\n\
         Colombia    → will-colombia-win-the-2026-fifa-world-cup-734\n\
         Japan       → will-japan-win-the-2026-fifa-world-cup-112\n\
         Senegal     → will-senegal-win-the-2026-fifa-world-cup{board_block}"
    )
}

// ─── Tool execution — calls engine functions directly, no HTTP re-entry ───────

fn execute_tool(name: &str, input: &Value) -> Value {
    match name {
        "get_board" => crate::board(),

        "find_best_edges" => {
            // All fixture teams with confirmed live Polymarket outright slugs.
            const OUTRIGHT_SLUGS: &[(&str, &str)] = &[
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

            // Tournament simulation gives model title probabilities for all teams.
            let teams_csv = OUTRIGHT_SLUGS.iter().map(|(t, _)| *t).collect::<Vec<_>>().join(",");
            let q = crate::Query::parse(&format!("?teams={}", teams_csv));
            let tournament_result = crate::tournament(&q).unwrap_or_else(|e| json!({ "error": e }));

            let mut model_probs: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
            if let Some(champ) = tournament_result.get("championship").and_then(|v| v.as_array()) {
                for entry in champ {
                    if let (Some(team), Some(prob)) = (
                        entry.get("team").and_then(|v| v.as_str()),
                        entry.get("title_probability").and_then(|v| v.as_f64()),
                    ) {
                        model_probs.insert(team.to_string(), prob);
                    }
                }
            }

            // Fetch live Polymarket price and compute edge for each team.
            let mut edges: Vec<Value> = OUTRIGHT_SLUGS.iter().filter_map(|(team, slug)| {
                let model_prob = *model_probs.get(*team)?;
                let qs = format!("?slug={}&outcome=Yes&model_prob={}", slug, model_prob);
                let q = crate::Query::parse(&qs);
                match crate::edge(&q) {
                    Ok(mut e) => { e["team"] = json!(team); Some(e) }
                    Err(_) => None,
                }
            }).collect();

            // Sort by edge descending (best value buys first).
            edges.sort_by(|a, b| {
                let ea = a.get("edge").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let eb = b.get("edge").and_then(|v| v.as_f64()).unwrap_or(0.0);
                eb.partial_cmp(&ea).unwrap_or(std::cmp::Ordering::Equal)
            });

            json!({
                "outright_edges": edges,
                "note": "Model probability = tournament title probability from 20k-sim bracket. Market price = live Polymarket. Outcome is 'Yes' on each binary market."
            })
        }

        "simulate_match" | "explain_model" => {
            let body = serde_json::to_string(input).unwrap_or_default();
            crate::simulate_body(&body).unwrap_or_else(|e| json!({ "error": e }))
        }

        "get_edge" => {
            let slug = input["slug"].as_str().unwrap_or("");
            let outcome = input["outcome"].as_str().unwrap_or("");
            let model_prob = input["model_prob"].as_f64().unwrap_or(0.5);
            // Construct a fake query string so we can reuse the existing edge() function.
            let qs = format!(
                "?slug={}&outcome={}&model_prob={}",
                crate::urldecode_noop(slug),
                crate::urldecode_noop(outcome),
                model_prob
            );
            let q = crate::Query::parse(&qs);
            crate::edge(&q).unwrap_or_else(|e| json!({ "error": e }))
        }

        "get_tournament_odds" => {
            let teams = input["teams"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .unwrap_or_default();
            let qs = format!("?teams={}", teams);
            let q = crate::Query::parse(&qs);
            crate::tournament(&q).unwrap_or_else(|e| json!({ "error": e }))
        }

        other => json!({ "error": format!("unknown tool: {other}") }),
    }
}

// ─── Anthropic Messages API call (blocking) ───────────────────────────────────

fn call_claude(http: &Client, api_key: &str, body: &Value) -> Result<Value, String> {
    let resp = http
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(body)
        .send()
        .map_err(|e| format!("Anthropic request failed: {e}"))?;

    let status = resp.status();
    let parsed: Value = resp
        .json()
        .map_err(|e| format!("Anthropic response parse failed: {e}"))?;

    if !status.is_success() {
        let msg = parsed["error"]["message"]
            .as_str()
            .unwrap_or("unknown error")
            .to_string();
        return Err(format!("Anthropic {status}: {msg}"));
    }

    Ok(parsed)
}

// ─── Main handler — called from main.rs for POST /api/chat ───────────────────

pub fn handle(body: &str) -> Result<Value, String> {
    let req: Value = serde_json::from_str(body).map_err(|e| format!("bad json: {e}"))?;

    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| "ANTHROPIC_API_KEY not set".to_string())?;

    let context = req.get("context").filter(|v| !v.is_null());
    let sys = system_prompt(context);

    let initial_messages = req["messages"]
        .as_array()
        .ok_or("messages must be an array")?
        .clone();

    if initial_messages.is_empty() {
        return Err("messages array is empty".into());
    }

    let http = Client::new();
    let mut messages: Vec<Value> = initial_messages;
    let mut tool_log: Vec<Value> = vec![];

    let mut payload = json!({
        "model": "claude-sonnet-4-6",
        "max_tokens": 1024,
        "system": sys,
        "tools": tools(),
        "messages": messages
    });

    let mut claude_resp = call_claude(&http, &api_key, &payload)?;

    // Agentic loop: keep running until Claude stops calling tools.
    loop {
        let stop_reason = claude_resp["stop_reason"].as_str().unwrap_or("");
        if stop_reason != "tool_use" {
            break;
        }

        let content_blocks: Vec<Value> = claude_resp["content"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        let mut tool_results: Vec<Value> = vec![];

        for block in &content_blocks {
            if block["type"].as_str() != Some("tool_use") {
                continue;
            }
            let id = block["id"].as_str().unwrap_or("").to_string();
            let name = block["name"].as_str().unwrap_or("").to_string();
            let input = block["input"].clone();

            let result = execute_tool(&name, &input);
            tool_log.push(json!({ "name": name, "input": input, "result": result }));
            tool_results.push(json!({
                "type": "tool_result",
                "tool_use_id": id,
                "content": result.to_string()
            }));
        }

        // Extend conversation history with the assistant's tool-call turn and results.
        messages.push(json!({ "role": "assistant", "content": content_blocks }));
        messages.push(json!({ "role": "user", "content": tool_results }));

        payload = json!({
            "model": "claude-sonnet-4-6",
            "max_tokens": 1024,
            "system": sys,
            "tools": tools(),
            "messages": messages
        });

        claude_resp = call_claude(&http, &api_key, &payload)?;
    }

    let reply = claude_resp["content"]
        .as_array()
        .and_then(|arr| {
            arr.iter()
                .find(|b| b["type"].as_str() == Some("text"))
                .and_then(|b| b["text"].as_str())
                .map(String::from)
        })
        .unwrap_or_else(|| "No response.".into());

    Ok(json!({ "reply": reply, "tool_calls": tool_log }))
}
