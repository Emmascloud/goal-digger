//! /api/chat handler for the tiny_http Goal Digger server.
//!
//! Calls the local engine functions DIRECTLY (board, simulate_body, edge,
//! tournament, conditional) — same process, no HTTP round-trip. Uses `ureq`
//! for the Anthropic API call (sync, matches the rest of this codebase).
//!
//! Add to Cargo.toml:
//!   ureq = { version = "2", features = ["json"] }
//!
//! In main.rs:
//!   1. mod chat;
//!   2. mark these `pub(crate)` (board/simulate_body/edge/tournament/Query
//!      already done in earlier PR):
//!        - fn conditional   <- new, added in this PR
//!   3. route table includes /api/board, /api/simulate, /api/edge,
//!      /api/tournament, /api/chat, /api/conditional
//!   4. set ANTHROPIC_API_KEY before running

use crate::{board, conditional, edge, simulate_body, tournament, Query};
use serde_json::{json, Value};

const MODEL: &str = "claude-sonnet-4-20250514";

fn tools() -> Value {
    json!([
        {
            "name": "get_board",
            "description": "Get all 8 World Cup fixtures priced by the live Dixon-Coles + Monte Carlo engine. Returns win/draw/loss probabilities, advance probability, over/under 2.5, BTTS, and top scorelines for each match.",
            "input_schema": { "type": "object", "properties": {}, "required": [] }
        },
        {
            "name": "simulate_match",
            "description": "Run a fresh 50,000-iteration Monte Carlo simulation for any matchup, with optional attack/defense adjustments.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "home": { "type": "string" },
                    "away": { "type": "string" },
                    "neutral": { "type": "boolean" },
                    "knockout": { "type": "boolean" },
                    "host_elo_bonus": { "type": "number" },
                    "home_attack_adj": { "type": "number" },
                    "home_defense_adj": { "type": "number" },
                    "away_attack_adj": { "type": "number" },
                    "away_defense_adj": { "type": "number" },
                    "sims": { "type": "integer" }
                },
                "required": ["home", "away"]
            }
        },
        {
            "name": "get_edge",
            "description": "Compare a model probability against the live Polymarket (Gamma) price for a given market slug + outcome. Returns edge and a VALUE_BUY / OVERPRICED / FAIR verdict.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "slug": { "type": "string" },
                    "outcome": { "type": "string" },
                    "model_prob": { "type": "number" }
                },
                "required": ["slug", "outcome", "model_prob"]
            }
        },
        {
            "name": "get_tournament_odds",
            "description": "Run a tournament-level simulation across the given teams and return championship probabilities.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "teams": { "type": "array", "items": { "type": "string" } }
                },
                "required": ["teams"]
            }
        },
        {
            "name": "find_best_edges",
            "description": "Scan the 8 board fixtures and return the matches where the model's home-win probability diverges most from 50/50.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "top_n": { "type": "integer", "description": "default 3" }
                },
                "required": []
            }
        },
        {
            "name": "explain_model",
            "description": "Re-simulate a specific board matchup and return lambda_home/lambda_away plus the full probability breakdown.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "home": { "type": "string" },
                    "away": { "type": "string" }
                },
                "required": ["home", "away"]
            }
        },
        {
            "name": "get_conditional_odds",
            "description": "Compute the probability of a conditional 'if X then Y' market — e.g. 'Portugal to score if Ronaldo gets subbed off', or anytime-scorer player props. Returns p_condition (how often the condition itself occurs), p_outcome_given_condition (the conditional probability — this is the model_prob to feed into get_edge), and p_outcome_unconditional (baseline for comparison). Player names must match data/players.json (e.g. 'Ronaldo', 'Mbappe', 'Messi') — if a player isn't in the roster data, p_condition/probabilities involving them come back as 0.0.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "home": { "type": "string", "description": "Home team name" },
                    "away": { "type": "string", "description": "Away team name" },
                    "neutral": { "type": "boolean", "description": "default true" },
                    "knockout": { "type": "boolean", "description": "default false" },
                    "condition": {
                        "type": "object",
                        "description": "One of: {\"type\":\"PlayerSubbedOff\",\"player\":\"<name>\"}, {\"type\":\"PlayerScores\",\"player\":\"<name>\"}, {\"type\":\"TeamLeadsAtMinute\",\"side\":\"Home\"|\"Away\",\"minute\":<0-90>}"
                    },
                    "outcome": {
                        "type": "object",
                        "description": "One of: {\"type\":\"TeamScoresAfter\",\"side\":\"Home\"|\"Away\"}, {\"type\":\"PlayerScoresAnytime\",\"player\":\"<name>\"}, {\"type\":\"TeamScoresAnytime\",\"side\":\"Home\"|\"Away\"}"
                    },
                    "sims": { "type": "integer", "description": "default 50000, max 200000" }
                },
                "required": ["home", "away", "condition", "outcome"]
            }
        }
    ])
}

fn execute_tool(name: &str, input: &Value) -> Value {
    match name {
        "get_board" => board(),

        "simulate_match" => simulate_body(&input.to_string())
            .unwrap_or_else(|e| json!({ "error": e })),

        "get_edge" => {
            let q = Query(vec![
                ("slug".into(), input["slug"].as_str().unwrap_or("").into()),
                ("outcome".into(), input["outcome"].as_str().unwrap_or("").into()),
                ("model_prob".into(), input["model_prob"].as_f64().unwrap_or(0.0).to_string()),
            ]);
            edge(&q).unwrap_or_else(|e| json!({ "error": e }))
        }

        "get_tournament_odds" => {
            let teams = input["teams"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(","))
                .unwrap_or_default();
            let q = Query(vec![("teams".into(), teams)]);
            tournament(&q).unwrap_or_else(|e| json!({ "error": e }))
        }

        "find_best_edges" => {
            let b = board();
            let top_n = input["top_n"].as_u64().unwrap_or(3) as usize;
            let mut rows: Vec<Value> = b["matches"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|m| {
                    let p = m["p_home_win"].as_f64().unwrap_or(0.5);
                    let dist = (p - 0.5).abs();
                    json!({
                        "id": m["id"],
                        "p_home_win": p,
                        "p_draw": m["p_draw"],
                        "p_away_win": m["p_away_win"],
                        "deviation_from_even": dist
                    })
                })
                .collect();
            rows.sort_by(|a, b| {
                b["deviation_from_even"].as_f64().unwrap_or(0.0)
                    .partial_cmp(&a["deviation_from_even"].as_f64().unwrap_or(0.0))
                    .unwrap()
            });
            json!(rows.into_iter().take(top_n).collect::<Vec<_>>())
        }

        "explain_model" => {
            let body = json!({
                "home": input["home"],
                "away": input["away"],
                "neutral": true,
                "knockout": true
            });
            simulate_body(&body.to_string()).unwrap_or_else(|e| json!({ "error": e }))
        }

        "get_conditional_odds" => {
            // Pass straight through — `conditional()` parses home/away/neutral/
            // knockout/condition/outcome/sims itself, with the same defaults
            // (neutral: true, knockout: false, sims: 50_000) as documented in
            // the tool schema above.
            conditional(&input.to_string()).unwrap_or_else(|e| json!({ "error": e }))
        }

        _ => json!({ "error": format!("unknown tool: {name}") }),
    }
}

const SYSTEM: &str = "You are the GoalDigger trading agent for the 2026 World Cup. \
You have access to a live Dixon-Coles + Monte Carlo engine (50,000 simulations per \
match) and live Polymarket (Gamma) prices via your tools. Use get_board for the \
current 8 fixtures, get_edge to compare a model probability against a live \
Polymarket market, simulate_match for custom matchups, get_tournament_odds for \
title probabilities, find_best_edges to scan for the biggest model/market \
divergences, explain_model to break down why a match is priced the way it is, and \
get_conditional_odds for conditional and player-prop markets (e.g. 'Portugal to \
score if Ronaldo is subbed off', anytime-scorer markets). For get_conditional_odds, \
the p_outcome_given_condition field is the model probability to use when comparing \
against a Polymarket price via get_edge. Be concise and precise. Lead with the \
number, then the reasoning. Always state probabilities as percentages.";

/// Entry point called from main.rs for POST /api/chat
pub fn chat(body: &str) -> Result<Value, String> {
    let req: Value = serde_json::from_str(body).map_err(|e| format!("bad json: {e}"))?;
    let mut conversation: Vec<Value> = req
        .get("messages")
        .and_then(|m| m.as_array())
        .cloned()
        .unwrap_or_default();

    if conversation.is_empty() {
        return Err("messages array is empty".into());
    }

    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| "ANTHROPIC_API_KEY not set".to_string())?;

    let mut tool_log: Vec<Value> = vec![];

    // Agentic loop, capped to avoid runaway calls
    for _ in 0..6 {
        let payload = json!({
            "model": MODEL,
            "max_tokens": 1024,
            "system": SYSTEM,
            "tools": tools(),
            "messages": conversation
        });

        let resp: Value = ureq::post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .send_json(&payload)
            .map_err(|e| format!("anthropic request failed: {e}"))?
            .body_mut()
            .read_json::<Value>()
            .map_err(|e| format!("anthropic response parse failed: {e}"))?;

        if let Some(err) = resp.get("error") {
            return Err(format!("anthropic error: {err}"));
        }

        let stop_reason = resp.get("stop_reason").and_then(|v| v.as_str()).unwrap_or("");
        let content = resp.get("content").cloned().unwrap_or_else(|| json!([]));

        if stop_reason != "tool_use" {
            let reply = content
                .as_array()
                .and_then(|arr| arr.iter().find(|b| b["type"] == "text"))
                .and_then(|b| b["text"].as_str())
                .unwrap_or("(no text response)")
                .to_string();
            return Ok(json!({ "reply": reply, "tool_calls": tool_log }));
        }

        // Record assistant's tool-use turn
        conversation.push(json!({ "role": "assistant", "content": content }));

        // Execute each tool call, build tool_result content blocks
        let mut tool_results: Vec<Value> = vec![];
        for block in content.as_array().unwrap_or(&vec![]) {
            if block["type"] == "tool_use" {
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
        }

        conversation.push(json!({ "role": "user", "content": tool_results }));
    }

    Ok(json!({
        "reply": "Hit the tool-call limit before reaching a final answer. Try a more specific question.",
        "tool_calls": tool_log
    }))
}
