# GOAL.md — Goal Digger Contribution Roadmap
> theAstralProgrammer0 | Joined: 2026-06-11 | Deadline: Friday 2026-06-13

## Context

Goal Digger is a World Cup trading brain on Aomi: simulates every match 50,000 times
(Dixon-Coles + Monte Carlo), finds Polymarket mispricings, and lets users place trades
non-custodially. Victor (victorchimakanu) owns the project. The repo is live at
https://github.com/victorchimakanu/goal-digger.

My role: ship two launch-blocking PRs before Friday. Emma (Emmascloud) shipped PR #1
(lucide-react crash fix, merged 2026-06-11). I'm picking up the two items Victor called
out as launch blockers.

---

## PR #1 — feat: wire /api/chat — live agentic right-rail

**Why it matters:** Victor's exact words: "wire up the chat agent... This is what makes
it feel like 'an AI that finds you bets,' not just a dashboard." The right-rail is
currently a static React component that pattern-matches on keywords. No real AI.

**Scope:**
- `server/src/chat.rs` — full rewrite from axum/async to tiny_http-compatible sync
  using `reqwest::blocking::Client` (already in server deps). The key constraint:
  tiny_http processes one request at a time, so tool execution must call engine
  functions directly (never HTTP back into the same server — that deadlocks).
- `server/src/main.rs` — expose `board()`, `simulate_body()`, `edge()`, `tournament()`,
  `Query`, and `urldecode` as `pub(crate)`. Add `mod chat;`. Add `/api/chat` POST route.
- `ui/src/views.jsx` — replace fake `buildResponse()` with real async fetch to
  `/api/chat`. Add `chatHistory` state for multi-turn conversations.
- `ui/src/data.jsx` — save raw board snapshot to `window.__GD_BOARD__` after live load
  so the chat handler has immediate context.

**6 tools wired to engine:**
1. `get_board` / `find_best_edges` → `board()`
2. `simulate_match` / `explain_model` → `simulate_body()`
3. `get_edge` → `edge()` via reconstructed Query
4. `get_tournament_odds` → `tournament()` via reconstructed Query

**Test checklist:**
- [ ] `cargo build` passes (in `server/`)
- [ ] Server starts: `cargo run` in `server/`
- [ ] Open http://localhost:8787 — board renders
- [ ] Type "where's the value today?" in right rail — response comes back from Claude
  with real tool calls visible in browser console
- [ ] Tool chips animate during the call, show tool names after
- [ ] Multi-turn: ask a follow-up — history is preserved
- [ ] No ANTHROPIC_API_KEY → server starts but /api/chat returns clear error message
- [ ] Bad JSON body → 400 error, not a crash

---

## PR #2 — feat: live Polymarket outright prices via /api/prices

**Why it matters:** Victor: "pull real Polymarket prices onto the board for the handful
of markets we'll demo, so the edges shown are real." Currently all `crowd` prices in
OUTRIGHTS and CHAMPIONSHIP are hardcoded sample numbers. This makes the edges fake.

**Finding:** No match-specific markets exist yet on Polymarket for these specific
knockout fixtures (fictional future matches). What DOES exist: tournament outright
markets for every team in the demo. Using these gives real Polymarket data.

**Real Polymarket prices confirmed live (2026-06-11):**
| Team        | Polymarket price | Hardcoded crowd | Gap (real vs fake) |
|-------------|-----------------|-----------------|-------------------|
| Spain       | 0.1695          | 0.27            | −0.10             |
| Argentina   | 0.0895          | 0.215           | −0.13             |
| France      | 0.1605          | 0.20            | −0.04             |
| England     | 0.1085          | 0.10            | +0.01             |
| Brazil      | 0.0865          | —               | —                 |
| Germany     | 0.0525          | —               | —                 |
| Netherlands | 0.0425          | —               | —                 |
| Portugal    | 0.1085          | —               | —                 |

**Scope:**
- `server/src/main.rs` — add SLUG_MAP (fixture-id/team → Polymarket slug), add
  `/api/prices` GET endpoint: fetches gamma_market for each outright in parallel
  (sequential is fine for a demo), returns `{outrights: [{team, slug, market_price}]}`
- `ui/src/data.jsx` — add `loadLivePrices()`, call after `loadLiveBoard()`, update
  `OUTRIGHTS` and `CHAMPIONSHIP` with live market prices, re-build EDGES

**Test checklist:**
- [ ] `cargo build` passes
- [ ] GET /api/prices returns real prices (check the numbers match Polymarket)
- [ ] Championship section on dashboard shows live Polymarket odds, not the hardcoded ones
- [ ] EdgesFeed recalculates with real edges — Spain model 0.313 vs market 0.1695 should
  show a large positive edge (but also a caution: outright edge ≠ match-level edge)
- [ ] Price fetch failure (network down) → board still renders with fallback data

---

## Architecture decisions

**Why sync / tiny_http for chat, not axum?**
The existing server is single-binary tiny_http. Adding axum would mean either running
two servers or a full server rewrite. For a weekend launch, keeping one process and one
technology is the right call. Blocking reqwest is fine — chat requests are rare and a
human is waiting anyway.

**Why outright prices, not match prices, for PR #2?**
No 2026 World Cup match-specific Polymarket markets exist yet (confirmed by querying
Gamma API 2026-06-11). The 8 demo fixtures are fictional knockout matchups. Outright
markets are live and give real, interesting edge data.

**No new Cargo dependencies for either PR.** Both use `reqwest::blocking` which is
already in `server/Cargo.toml`.
