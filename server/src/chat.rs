// chat.rs  –  /api/chat  proxy for Goal Digger
//
// Drop this file into  server/src/chat.rs
// Then in server/src/main.rs  add:
//
//   mod chat;
//   ...
//   .route("/api/chat", post(chat::handler))
//
// Cargo.toml additions (under [dependencies]):
//   reqwest  = { version = "0.12", features = ["json"] }
//   tokio    = { version = "1",    features = ["full"] }
//   serde    = { version = "1",    features = ["derive"] }
//   serde_json = "1"
//   axum     = "0.7"
//
// Set env var before running:
//   export ANTHROPIC_API_KEY="sk-ant-..."

use axum::{extract::State, http::StatusCode, Json};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

// ── shared app state (pass your existing SimState in here) ──────────────────

pub struct ChatState {
    pub http: Client,
    pub api_key: String,
    // add a reference to your board/sim state here if needed
    // pub sim: Arc<YourSimState>,
}

// ── request / response types ────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<Message>,
    #[serde(default)]
    pub context: Option<Value>, // optional: pass board snapshot from frontend
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Message {
    pub role: String,  // "user" | "assistant"
    pub content: String,
}

#[derive(Serialize)]
pub struct ChatResponse {
    pub reply: String,
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Serialize)]
pub struct ToolCall {
    pub name: String,
    pub input: Value,
    pub result: Value,
}

// ── the six Aomi tools ───────────────────────────────────────────────────────

fn aomi_tools() -> Value {
    json!([
        {
            "name": "get_board",
            "description": "Get all 8 World Cup fixtures priced by the live Dixon-Coles engine. Returns win/draw/loss probabilities, over/under, BTTS, and Polymarket edge for each match.",
            "input_schema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        },
        {
            "name": "simulate_match",
            "description": "Run 50,000 Monte Carlo simulations for a specific match and return full probability distribution including scorelines, xG, and heatmap data.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "home": { "type": "string", "description": "Home team name" },
                    "away": { "type": "string", "description": "Away team name" },
                    "neutral": { "type": "boolean", "description": "Neutral venue?" },
                    "knockout": { "type": "boolean", "description": "Knockout stage?" },
                    "home_adj": { "type": "number", "description": "Home lambda adjustment (injury/fitness)" },
                    "away_adj": { "type": "number", "description": "Away lambda adjustment" },
                    "sims": { "type": "integer", "description": "Number of simulations (default 50000)" }
                },
                "required": ["home", "away"]
            }
        },
        {
            "name": "get_edge",
            "description": "Compare model probability vs live Polymarket (Gamma) price for a specific market. Returns edge in percentage points and Kelly stake recommendation.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "slug": { "type": "string", "description": "Polymarket market slug" },
                    "outcome": { "type": "string", "description": "Outcome to check (e.g. 'yes', 'no', team name)" },
                    "model_prob": { "type": "number", "description": "Model probability (0-1) from simulation" }
                },
                "required": ["slug", "outcome", "model_prob"]
            }
        },
        {
            "name": "get_tournament_odds",
            "description": "Get tournament winner probabilities for a list of teams, simulated through all remaining fixtures.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "teams": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "List of team names to get title odds for"
                    }
                },
                "required": ["teams"]
            }
        },
        {
            "name": "find_best_edges",
            "description": "Scan all current fixtures and return the top N markets where the model probability diverges most from Polymarket crowd prices.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "top_n": { "type": "integer", "description": "Number of top edges to return (default 3)" },
                    "min_edge": { "type": "number", "description": "Minimum edge threshold in percentage points (default 5)" }
                },
                "required": []
            }
        },
        {
            "name": "explain_model",
            "description": "Explain how the Dixon-Coles model priced a specific match — which factors moved the lambda (injuries, home field, fitness), and why the final probabilities landed where they did.",
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

// ── tool execution: calls your local engine endpoints ───────────────────────

async fn execute_tool(
    http: &Client,
    tool_name: &str,
    tool_input: &Value,
) -> Value {
    let base = "http://127.0.0.1:8787";

    match tool_name {
        "get_board" => {
            match http.get(format!("{}/api/board", base)).send().await {
                Ok(r) => r.json::<Value>().await.unwrap_or(json!({"error": "parse failed"})),
                Err(e) => json!({"error": e.to_string()}),
            }
        }

        "simulate_match" => {
            match http.post(format!("{}/api/simulate", base))
                .json(tool_input)
                .send().await
            {
                Ok(r) => r.json::<Value>().await.unwrap_or(json!({"error": "parse failed"})),
                Err(e) => json!({"error": e.to_string()}),
            }
        }

        "get_edge" => {
            let slug = tool_input["slug"].as_str().unwrap_or("");
            let outcome = tool_input["outcome"].as_str().unwrap_or("");
            let model_prob = tool_input["model_prob"].as_f64().unwrap_or(0.5);
            let url = format!(
                "{}/api/edge?slug={}&outcome={}&model_prob={}",
                base, slug, outcome, model_prob
            );
            match http.get(&url).send().await {
                Ok(r) => r.json::<Value>().await.unwrap_or(json!({"error": "parse failed"})),
                Err(e) => json!({"error": e.to_string()}),
            }
        }

        "get_tournament_odds" => {
            let teams = tool_input["teams"]
                .as_array()
                .map(|arr| arr.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(","))
                .unwrap_or_default();
            let url = format!("{}/api/tournament?teams={}", base, teams);
            match http.get(&url).send().await {
                Ok(r) => r.json::<Value>().await.unwrap_or(json!({"error": "parse failed"})),
                Err(e) => json!({"error": e.to_string()}),
            }
        }

        "find_best_edges" => {
            // Fetch board and surface top edges by model_prob vs market_prob delta
            match http.get(format!("{}/api/board", base)).send().await {
                Ok(r) => {
                    let board = r.json::<Value>().await.unwrap_or(json!([]));
                    let top_n = tool_input["top_n"].as_u64().unwrap_or(3) as usize;
                    let min_edge = tool_input["min_edge"].as_f64().unwrap_or(5.0);

                    if let Some(fixtures) = board.as_array() {
                        let mut edges: Vec<Value> = fixtures.iter()
                            .filter_map(|f| {
                                let model = f["model_win_prob"].as_f64()?;
                                let market = f["market_win_prob"].as_f64()?;
                                let edge = (model - market) * 100.0;
                                if edge.abs() >= min_edge {
                                    Some(json!({
                                        "match": format!("{} vs {}",
                                            f["home"].as_str().unwrap_or("?"),
                                            f["away"].as_str().unwrap_or("?")),
                                        "edge_pct": edge,
                                        "model_prob": model,
                                        "market_prob": market,
                                        "direction": if edge > 0.0 { "model_higher" } else { "model_lower" }
                                    }))
                                } else {
                                    None
                                }
                            })
                            .collect();

                        edges.sort_by(|a, b| {
                            let ea = a["edge_pct"].as_f64().unwrap_or(0.0).abs();
                            let eb = b["edge_pct"].as_f64().unwrap_or(0.0).abs();
                            eb.partial_cmp(&ea).unwrap()
                        });

                        json!(edges.into_iter().take(top_n).collect::<Vec<_>>())
                    } else {
                        json!({"error": "no board data"})
                    }
                }
                Err(e) => json!({"error": e.to_string()}),
            }
        }

        "explain_model" => {
            // Simulate the match and return a structured explanation
            match http.post(format!("{}/api/simulate", base))
                .json(tool_input)
                .send().await
            {
                Ok(r) => {
                    let sim = r.json::<Value>().await.unwrap_or(json!({}));
                    json!({
                        "home": tool_input["home"],
                        "away": tool_input["away"],
                        "home_lambda": sim["home_lambda"],
                        "away_lambda": sim["away_lambda"],
                        "adjustments": sim["adjustments"],
                        "win": sim["home_win"],
                        "draw": sim["draw"],
                        "loss": sim["away_win"],
                        "top_scorelines": sim["top_scorelines"]
                    })
                }
                Err(e) => json!({"error": e.to_string()}),
            }
        }

        _ => json!({"error": format!("unknown tool: {}", tool_name)}),
    }
}

// ── main handler ─────────────────────────────────────────────────────────────

pub async fn handler(
    State(state): State<Arc<ChatState>>,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, (StatusCode, String)> {

    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .unwrap_or_else(|_| state.api_key.clone());

    if api_key.is_empty() {
        return Err((StatusCode::INTERNAL_SERVER_ERROR,
            "ANTHROPIC_API_KEY not set".into()));
    }

    // Build system prompt with live context if provided
    let system = if let Some(ctx) = &req.context {
        format!(
            "You are the GoalDigger trading agent. You help users find Polymarket \
             mispricings on 2026 World Cup matches using a live Dixon-Coles + \
             Monte Carlo engine (50,000 simulations per match).\n\n\
             Current board snapshot:\n{}\n\n\
             Use your tools to pull live numbers. Be concise and precise. \
             Lead with the edge, then the reasoning.",
            serde_json::to_string_pretty(ctx).unwrap_or_default()
        )
    } else {
        "You are the GoalDigger trading agent. You help users find Polymarket \
         mispricings on 2026 World Cup matches using a live Dixon-Coles + \
         Monte Carlo engine (50,000 simulations per match).\n\n\
         Use your tools to pull live numbers. Be concise and precise. \
         Lead with the edge, then the reasoning.".into()
    };

    // First call to Claude
    let body = json!({
        "model": "claude-sonnet-4-20250514",
        "max_tokens": 1024,
        "system": system,
        "tools": aomi_tools(),
        "messages": req.messages
    });

    let resp = state.http
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send().await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    let mut claude_resp: Value = resp.json().await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    let mut tool_calls_log: Vec<ToolCall> = vec![];

    // Agentic loop: keep running until Claude stops calling tools
    loop {
        let stop_reason = claude_resp["stop_reason"].as_str().unwrap_or("");

        if stop_reason != "tool_use" {
            break;
        }

        // Collect all tool use blocks from this response
        let content_blocks = claude_resp["content"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        let mut tool_results: Vec<Value> = vec![];

        for block in &content_blocks {
            if block["type"].as_str() == Some("tool_use") {
                let tool_id   = block["id"].as_str().unwrap_or("").to_string();
                let tool_name = block["name"].as_str().unwrap_or("").to_string();
                let tool_input = block["input"].clone();

                let result = execute_tool(&state.http, &tool_name, &tool_input).await;

                tool_calls_log.push(ToolCall {
                    name: tool_name.clone(),
                    input: tool_input,
                    result: result.clone(),
                });

                tool_results.push(json!({
                    "type": "tool_result",
                    "tool_use_id": tool_id,
                    "content": result.to_string()
                }));
            }
        }

        // Build updated message history with assistant turn + tool results
        let mut updated_messages = req.messages.clone();
        updated_messages.push(Message {
            role: "assistant".into(),
            content: serde_json::to_string(&content_blocks).unwrap_or_default(),
        });

        // Continue conversation with tool results
        let follow_up = json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 1024,
            "system": system,
            "tools": aomi_tools(),
            "messages": [
                // original messages
                serde_json::to_value(&req.messages).unwrap_or(json!([])),
                // assistant's tool call turn
                json!({
                    "role": "assistant",
                    "content": content_blocks
                }),
                // tool results turn
                json!({
                    "role": "user",
                    "content": tool_results
                })
            ].iter()
             .flat_map(|v| v.as_array().cloned().unwrap_or_else(|| vec![v.clone()]))
             .collect::<Vec<_>>()
        });

        let follow_resp = state.http
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&follow_up)
            .send().await
            .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

        claude_resp = follow_resp.json().await
            .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;
    }

    // Extract final text reply
    let reply = claude_resp["content"]
        .as_array()
        .and_then(|arr| {
            arr.iter()
                .find(|b| b["type"].as_str() == Some("text"))
                .and_then(|b| b["text"].as_str())
                .map(String::from)
        })
        .unwrap_or_else(|| "No response from model.".into());

    Ok(Json(ChatResponse { reply, tool_calls: tool_calls_log }))
}
