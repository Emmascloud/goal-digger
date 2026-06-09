# Run Goal Digger (live engine + UI)

No AomiFrame. A small local server runs the real Rust engine and serves the UI
same-origin, so the dashboard shows live Dixon-Coles + Monte-Carlo numbers.

## One command

```bash
cd Projects/2026-06-June/goal-digger/server
cargo run --release
```

Then open **http://127.0.0.1:8787/** in a browser. The board, match detail, and
edges feed are priced by the live engine (the same `sim.rs` the Aomi plugin uses —
path-included, zero duplication).

## What's live vs sample

- **Model** numbers (win/draw/loss, advance, over/under, BTTS, scorelines, heatmap):
  real engine, 50k Monte-Carlo per match, with the per-fixture injury/fitness/host
  adjustments applied. Spain v Germany prices to 0.74 once the keeper-out + crowd
  adjustments lift Spain's lambda to 2.49.
- **Crowd** prices on the board: sample Polymarket values (real per-match markets for
  these illustrative fixtures don't exist yet). `/api/edge` fetches REAL Gamma prices
  for any live market slug on demand.

## Endpoints

| Endpoint | Returns |
|---|---|
| `GET /` | the camel-theme UI (modular `goal digger2.html`) |
| `GET /api/board` | all 8 fixtures priced by the engine |
| `POST /api/simulate` | one match: `{home, away, neutral, knockout, *_adj, seed, sims}` |
| `GET /api/edge?slug=&outcome=&model_prob=` | model vs live Polymarket (Gamma) price |
| `GET /api/tournament?teams=Spain,France,...` | title probabilities |

## Standalone (no server)

`ui/goal-digger-camel.html` opens directly in a browser. With no server it shows the
bundled mock data (the live fetch fails silently and falls back). Run the server for
real numbers.

## How the UI upgrades to live

`ui/src/data.jsx` ships mock data so the page renders instantly, then calls
`loadLiveBoard()` → `GET /api/board` and re-renders with real engine output when the
server is present. `window.GD_LIVE` is `true` once live data is in.

## Wiring the chat panel (next)

The right-rail Aomi agent is still presentational. To make it real, proxy
`POST /api/chat` to the Anthropic API (your key) with the six tools, or point it at the
Aomi runtime's `/api/chat`. The deterministic board/detail/edges already run on the
engine and need no LLM.
