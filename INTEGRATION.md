# GoalDigger /api/chat — Integration Guide
# 3 files, ~30 lines of wiring, then the agent is live.

## Files delivered
- chat.rs        → server/src/chat.rs
- chat-panel.js  → ui/src/chat-panel.js  (or inline in goal-digger-camel.html)
- chat-panel.css → add to goal-digger-camel.html <style> block

---

## Step 1 — Cargo.toml  (server/Cargo.toml)

Add under [dependencies] if not already present:

    reqwest    = { version = "0.12", features = ["json"] }
    serde      = { version = "1",    features = ["derive"] }
    serde_json = "1"

reqwest is needed by chat.rs to call both the local engine
endpoints and the Anthropic API.

---

## Step 2 — main.rs  (server/src/main.rs)

### 2a. Add the module
    mod chat;

### 2b. Build shared state
    use std::sync::Arc;

    let chat_state = Arc::new(chat::ChatState {
        http: reqwest::Client::new(),
        api_key: std::env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
    });

### 2c. Register the route (Axum 0.7 syntax)
    .route("/api/chat", post(chat::handler))
    .with_state(chat_state)

If your router already uses .with_state() for a different state type,
merge ChatState into your existing AppState struct and pass it through.

---

## Step 3 — HTML  (ui/goal-digger-camel.html)

### 3a. Add the DOM elements to the right rail
    <div id="chat-log"></div>

    <div style="display:flex;gap:0.5rem;padding:0.75rem">
      <textarea id="chat-input" rows="2"
        placeholder="Ask about edges, match prices, best bets…"></textarea>
      <button id="chat-send">Send</button>
    </div>

### 3b. Import and init
    <script type="module">
      import { initChatPanel, updateBoardSnapshot } from "./chat-panel.js";

      document.addEventListener("DOMContentLoaded", () => {
        initChatPanel();
      });

      // After loadLiveBoard() resolves in data.jsx, call:
      // updateBoardSnapshot(boardData);
    </script>

### 3c. Add CSS
    <link rel="stylesheet" href="./chat-panel.css">
    <!-- or paste the contents of chat-panel.css into your <style> block -->

---

## Step 4 — Run

    export ANTHROPIC_API_KEY="sk-ant-..."
    cd server && cargo run --release

Open http://127.0.0.1:8787/
The right-rail chat now calls Claude with the six Aomi tools,
which in turn call your live engine endpoints.

---

## The six tools wired up

| Tool              | Calls                      | What it returns                          |
|-------------------|----------------------------|------------------------------------------|
| get_board         | GET  /api/board            | All 8 fixtures with probabilities        |
| simulate_match    | POST /api/simulate         | 50k sim output for one match             |
| get_edge          | GET  /api/edge             | Model vs Polymarket edge                 |
| get_tournament_odds| GET /api/tournament       | Title probabilities                      |
| find_best_edges   | GET  /api/board (derived)  | Top N mispricings across all fixtures    |
| explain_model     | POST /api/simulate         | Why the model priced a match as it did   |

---

## What to tell the team

"I built the /api/chat proxy. It's a Rust handler that:
 - Accepts conversation history from the frontend
 - Calls Claude with six tools mapped to your live engine endpoints
 - Runs the full agentic loop (tool_use → execute → continue) until Claude
   has all the data it needs
 - Returns the final reply + a tool_calls log for debugging
 
 Three files, ~30 lines of wiring in main.rs, done."
