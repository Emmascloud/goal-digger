# Goal Digger

An AI trading brain for the 2026 FIFA World Cup, built on [Aomi](https://aomi.dev).

Goal Digger simulates every World Cup match 50,000 times, finds where the Polymarket
crowd has the price wrong, and places the bet through your own wallet. Simulate before
you sign. No custody. The math does the thinking; you keep the keys.

> Probabilities are estimates, not promises. This is a research and demo project, not
> financial advice.

---

## Why this exists

Polymarket is the largest prediction market in the world, and the World Cup is the
single biggest event it has ever priced (over 1 billion dollars in volume across 140+
markets). Here is the uncomfortable truth about that market: most people lose. Public
reporting puts it bluntly. More than 70 percent of traders end up down, a tiny fraction
of accounts take the majority of the profit, and the winners "look like bots."

The winning cohort is already automated. Goal Digger hands that same capability to
everyone else. It is the bot, made honest and explainable.

The edge is real and well studied. Prediction-market prices are raw crowd probabilities
with no house margin, so a genuine model edge is not eaten by vig. Retail systematically
overpays for longshots and underpays favorites. A disciplined match model that prices
the full scoreline distribution can find those gaps and act on them before the crowd
corrects.

## Who it is for

- **Sports traders** who want a model that prices matches properly and flags value,
  instead of betting on vibes.
- **Crypto-native users** who want to act on-chain without juggling API keys, custodians,
  or raw contract calls.
- **Builders** who want a clean reference for an Aomi app: a real engine, a real tool
  surface, and a real UI, wired end to end.

## How it works

### The simulation engine

The core is a layered match model, not a single guess.

1. **Team strength.** Each team's Elo rating and expected goals (xG for and against)
   set a baseline attack and defense.
2. **Dixon-Coles expected goals.** Strength becomes a pair of expected-goal rates, with
   a venue tilt and the Dixon-Coles low-score correction so draws and 1-0s are priced
   correctly.
3. **Bounded adjustments.** Injuries, suspensions, fitness, rest, and host-crowd effects
   become multipliers on those rates (clamped, so judgment can nudge but never invent).
4. **50,000 Monte-Carlo simulations.** The match is played 50,000 times. Knockout ties
   resolve through extra time and a penalty shootout. The output is the full distribution:
   win, draw, loss, advance, over/under, both teams to score, and the most likely scores.
5. **Tournament rollouts.** Chain match simulations across the bracket to get each team's
   title probability.

"Look through all the possibilities" is literal here. The Monte-Carlo enumerates the
outcome distribution rather than returning one point estimate.

### Built on Aomi, borrowing the trade rails

The hard part of trading on-chain is already solved by Aomi's official Polymarket app:
typed orders, simulation before signing, EIP-712 wallet signatures, non-custodial
execution. Goal Digger does not rebuild any of that. It ships only the brain (the stats
and the simulation) and composes the existing execution tools at runtime. The user signs
every order; funds never leave their control.

## Repository layout

```
goal-digger/
  app/          Aomi plugin (Rust, cdylib). 6 tools + the Dixon-Coles / Monte-Carlo engine.
    src/sim.rs    the engine (team strength -> Dixon-Coles -> 50k Monte-Carlo)
    src/tool.rs   simulate_match, simulate_tournament, find_edge, get_team_dossier,
                  get_wc_fixtures, watch_match
    src/data.rs   team strength loader + API-FOOTBALL + Polymarket Gamma price fetch
    data/teams.json   bundled Elo + xG per team
  server/       Tiny HTTP server that reuses the exact engine and serves the UI.
  ui/           React dashboard (Camel Brown / retro theme). Board, Match Detail, Edges.
  data-prep/    Python: soccerdata (FBref xG + Elo) -> teams.json
```

## Run it

### The plugin

```bash
cd app
cargo build --release   # produces target/release/libgoal_digger.dylib
```

### The dashboard with live engine numbers

```bash
cd server
cargo run --release     # serves http://127.0.0.1:8787/
```

Open http://127.0.0.1:8787/. The board, match detail, and edges feed are priced by the
live engine. The UI also opens standalone (`ui/goal-digger-camel.html`) on bundled sample
data when no server is running.

## The tool surface

| Tool | What it does |
|---|---|
| `simulate_match` | One match, 50,000 simulations, full outcome distribution. |
| `simulate_tournament` | Bracket rollouts to each team's title probability. |
| `find_edge` | Model probability vs the live Polymarket price, with a quarter-Kelly stake. |
| `get_team_dossier` | Elo, xG, and live injuries for a team. |
| `get_wc_fixtures` | Schedule and live scores. |
| `watch_match` | Live momentum (early-goal overreactions). Streaming lands next. |

## Current blockers and honest status

This is a work in progress aimed at a demo before the tournament. What is real, and
what is not, stated plainly:

**Working now**
- The engine compiles to a loadable Aomi plugin and produces sane, verified
  probabilities (favorites win at believable rates, injuries shift the numbers, knockouts
  go to penalties, tournaments rank correctly).
- The dashboard renders and shows live engine numbers through the local server, with no
  heavy embed required.
- `find_edge` reads real Polymarket prices from the public Gamma API.

**Blockers**
1. **Live match data needs a paid key.** API-FOOTBALL covers the 2026 World Cup
   (fixtures, lineups, injuries), but the free tier may not unlock the current season.
   A paid tier (around 25 dollars per month) removes the doubt for a live demo. Until the
   key is set, fixtures and injuries fall back to bundled data.
2. **National-team xG has no clean API.** FBref has it (Opta-sourced) but is
   Cloudflare-protected, so a small Python prep step (`data-prep/`) using `soccerdata`
   pulls xG and Elo into `teams.json`. This runs offline, daily.
3. **Crowd prices on the board are sample data.** Real per-match Polymarket markets for
   these illustrative fixtures do not exist yet. The model column is live; the crowd
   column is illustrative. `find_edge` uses real prices on demand.
4. **The chat panel is presentational.** Wiring the right-rail agent means proxying chat
   to a Claude tool loop or to the Aomi runtime. The deterministic board and detail views
   already run on the engine and need no LLM.
5. **Adjustment magnitudes are uncalibrated.** The injury and rest multipliers are
   reasonable but not yet fit against historical World Cup data.

## License

MIT. See [LICENSE](LICENSE).
