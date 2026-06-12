/* ============================================================
   Goal Digger — data layer
   A real (independent) Poisson model so every figure in the
   Match Detail view derives from the same two lambdas and stays
   internally consistent. Output shapes mirror the backend tools:
   simulate_match · find_edge · simulate_tournament
   ============================================================ */

const WALLET = { address: "0x7Af3…C21e", balance: 4820.5 };

// ---- Poisson helpers ----
const fact = (n) => { let r = 1; for (let i = 2; i <= n; i++) r *= i; return r; };
const pois = (k, l) => Math.exp(-l) * Math.pow(l, k) / fact(k);

// full joint grid up to `max` goals each, independent Poisson
function jointGrid(lh, la, max = 10) {
  const g = [];
  for (let h = 0; h <= max; h++) {
    const row = [];
    for (let a = 0; a <= max; a++) row.push(pois(h, lh) * pois(a, la));
    g.push(row);
  }
  return g;
}

function deriveMatch(m) {
  const lh = m.lambda_home, la = m.lambda_away;
  const g = jointGrid(lh, la, 10);
  let pH = 0, pD = 0, pA = 0, pOver = 0, pBtts = 0;
  for (let h = 0; h <= 10; h++) {
    for (let a = 0; a <= 10; a++) {
      const p = g[h][a];
      if (h > a) pH += p; else if (h === a) pD += p; else pA += p;
      if (h + a > 2.5) pOver += p;
      if (h >= 1 && a >= 1) pBtts += p;
    }
  }
  // 6x6 display heatmap (0..5 exact scores) + peak
  const heat = [];
  let peak = { h: 0, a: 0, p: 0 };
  let maxCell = 0;
  for (let h = 0; h <= 5; h++) {
    const row = [];
    for (let a = 0; a <= 5; a++) {
      const p = g[h][a];
      row.push(p);
      if (p > maxCell) maxCell = p;
      if (p > peak.p) peak = { h, a, p };
    }
    heat.push(row);
  }
  // top scorelines across full grid
  const all = [];
  for (let h = 0; h <= 6; h++) for (let a = 0; a <= 6; a++) all.push({ home: h, away: a, prob: g[h][a] });
  all.sort((x, y) => y.prob - x.prob);
  const top_scorelines = all.slice(0, 5);

  const model = { home: pH, draw: pD, away: pA };
  // edges (model - crowd) in probability terms
  const edge = {
    home: model.home - m.crowd.home,
    draw: model.draw - m.crowd.draw,
    away: model.away - m.crowd.away,
  };
  // featured = outcome with largest positive edge (best value buy)
  const order = ["home", "draw", "away"];
  const best = order.reduce((b, k) => (edge[k] > edge[b] ? k : b), "home");

  return {
    ...m,
    sims: 50000,
    model, edge, best,
    p_over_2_5: pOver, p_btts: pBtts,
    expected_goals_home: lh, expected_goals_away: la,
    heat, heatMax: maxCell, peak, top_scorelines,
  };
}

// ---- flags: minimal recognizable color bands (no emoji) ----
const FLAGS = {
  ESP: { dir: "h", bands: ["#AA151B", "#F1BF00", "#AA151B"], stops: [25, 75] },
  GER: { dir: "h", bands: ["#111", "#D00", "#FFCE00"] },
  ARG: { dir: "h", bands: ["#74ACDF", "#fff", "#74ACDF"] },
  MEX: { dir: "v", bands: ["#006847", "#fff", "#CE1126"] },
  FRA: { dir: "v", bands: ["#0055A4", "#fff", "#EF4135"] },
  ENG: { dir: "cross", bands: ["#fff", "#CF142B"] },
  BRA: { dir: "h", bands: ["#009C3B", "#FFDF00", "#009C3B"] },
  NED: { dir: "h", bands: ["#AE1C28", "#fff", "#21468B"] },
  POR: { dir: "v", bands: ["#006600", "#006600", "#FF0000"], stops: [40, 40] },
  URU: { dir: "h", bands: ["#fff", "#0038A8", "#fff", "#0038A8", "#fff"] },
  CRO: { dir: "h", bands: ["#FF0000", "#fff", "#171796"] },
  MAR: { dir: "solid", bands: ["#C1272D"] },
  USA: { dir: "h", bands: ["#B22234", "#fff", "#B22234", "#fff", "#3C3B6E"] },
  COL: { dir: "h", bands: ["#FCD116", "#FCD116", "#003893", "#CE1126"] },
  JPN: { dir: "solid", bands: ["#fff"] },
  SEN: { dir: "v", bands: ["#00853F", "#FDEF42", "#E31B23"] },
};

// ---- seeded matches (8) ----
// crowd = Polymarket implied price; model derives from lambdas.
const RAW_MATCHES = [
  {
    id: "esp-ger",
    comp: "Quarter-final · R8",
    kickoff: "Today 18:00",
    soon: true,
    venue: "Estadio Azteca, Mexico City",
    home: { code: "ESP", name: "Spain" },
    away: { code: "GER", name: "Germany" },
    lambda_home: 2.18, lambda_away: 0.83,
    crowd: { home: 0.55, draw: 0.25, away: 0.20 },
    adjustments: [
      { team: "Germany", dir: "up", reason: "first-choice keeper out · opponent attack ×1.10" },
      { team: "Spain", dir: "flat", reason: "3 days rest · no change" },
      { team: "Spain", dir: "up", reason: "neutral venue, pro-Spain crowd · attack ×1.04" },
      { team: "Germany", dir: "down", reason: "two starters on yellow-card suspension · attack ×0.94" },
    ],
  },
  {
    id: "arg-mex",
    comp: "Quarter-final · R8",
    kickoff: "Today 21:00",
    soon: true,
    venue: "MetLife Stadium, New Jersey",
    home: { code: "ARG", name: "Argentina" },
    away: { code: "MEX", name: "Mexico" },
    lambda_home: 1.74, lambda_away: 0.96,
    crowd: { home: 0.62, draw: 0.23, away: 0.15 },
    adjustments: [
      { team: "Argentina", dir: "flat", reason: "full-strength squad · no change" },
      { team: "Mexico", dir: "up", reason: "altitude-trained, low fatigue · defense ×1.05" },
      { team: "Argentina", dir: "down", reason: "captain managing a knock · attack ×0.97" },
    ],
  },
  {
    id: "fra-eng",
    comp: "Quarter-final · R8",
    kickoff: "Tomorrow 18:00",
    venue: "AT&T Stadium, Dallas",
    home: { code: "FRA", name: "France" },
    away: { code: "ENG", name: "England" },
    lambda_home: 1.46, lambda_away: 1.21,
    crowd: { home: 0.44, draw: 0.27, away: 0.29 },
    adjustments: [
      { team: "France", dir: "flat", reason: "settled XI · no change" },
      { team: "England", dir: "up", reason: "key winger returns from injury · attack ×1.08" },
      { team: "France", dir: "down", reason: "left-back doubtful · defense ×0.96" },
    ],
  },
  {
    id: "bra-ned",
    comp: "Quarter-final · R8",
    kickoff: "Tomorrow 21:00",
    venue: "Hard Rock Stadium, Miami",
    home: { code: "BRA", name: "Brazil" },
    away: { code: "NED", name: "Netherlands" },
    lambda_home: 1.58, lambda_away: 1.12,
    crowd: { home: 0.52, draw: 0.26, away: 0.22 },
    adjustments: [
      { team: "Brazil", dir: "up", reason: "playmaker back from suspension · attack ×1.07" },
      { team: "Netherlands", dir: "flat", reason: "fully rested · no change" },
      { team: "Brazil", dir: "down", reason: "humid conditions, high tempo risk · defense ×0.97" },
    ],
  },
  {
    id: "por-uru",
    comp: "Round of 16",
    kickoff: "Jun 06 · 18:00",
    venue: "Levi's Stadium, San Francisco",
    home: { code: "POR", name: "Portugal" },
    away: { code: "URU", name: "Uruguay" },
    lambda_home: 1.63, lambda_away: 1.04,
    crowd: { home: 0.58, draw: 0.24, away: 0.18 },
    adjustments: [
      { team: "Portugal", dir: "flat", reason: "first-choice front line · no change" },
      { team: "Uruguay", dir: "down", reason: "centre-back suspended · defense ×0.93" },
    ],
  },
  {
    id: "cro-mar",
    comp: "Round of 16",
    kickoff: "Jun 06 · 21:00",
    venue: "Lincoln Financial Field, Philadelphia",
    home: { code: "CRO", name: "Croatia" },
    away: { code: "MAR", name: "Morocco" },
    lambda_home: 1.18, lambda_away: 1.09,
    crowd: { home: 0.41, draw: 0.31, away: 0.28 },
    adjustments: [
      { team: "Morocco", dir: "up", reason: "back four intact, strong press · defense ×1.06" },
      { team: "Croatia", dir: "down", reason: "midfield legs after extra time · attack ×0.95" },
    ],
  },
  {
    id: "usa-col",
    comp: "Round of 16",
    kickoff: "Jun 07 · 18:00",
    venue: "SoFi Stadium, Los Angeles",
    home: { code: "USA", name: "United States" },
    away: { code: "COL", name: "Colombia" },
    lambda_home: 1.31, lambda_away: 1.34,
    crowd: { home: 0.40, draw: 0.27, away: 0.33 },
    adjustments: [
      { team: "United States", dir: "up", reason: "home crowd, short travel · attack ×1.06" },
      { team: "Colombia", dir: "flat", reason: "full squad available · no change" },
    ],
  },
  {
    id: "jpn-sen",
    comp: "Round of 16",
    kickoff: "Jun 07 · 21:00",
    venue: "GEHA Field, Kansas City",
    home: { code: "JPN", name: "Japan" },
    away: { code: "SEN", name: "Senegal" },
    lambda_home: 1.22, lambda_away: 1.28,
    crowd: { home: 0.36, draw: 0.28, away: 0.36 },
    adjustments: [
      { team: "Senegal", dir: "up", reason: "physical edge in transition · attack ×1.05" },
      { team: "Japan", dir: "flat", reason: "press intensity unchanged · no change" },
    ],
  },
];

const MATCHES = RAW_MATCHES.map(deriveMatch);

// ---- simulate_tournament: championship ----
const CHAMPIONSHIP = [
  { team: "Spain", code: "ESP", title_probability: 0.313 },
  { team: "Argentina", code: "ARG", title_probability: 0.235 },
  { team: "France", code: "FRA", title_probability: 0.193 },
  { team: "England", code: "ENG", title_probability: 0.118 },
  { team: "Brazil", code: "BRA", title_probability: 0.086 },
];

// outright title markets (find_edge shape) — crowd price vs model
const OUTRIGHTS = [
  { team: "Spain", code: "ESP", model: 0.313, crowd: 0.27 },
  { team: "Argentina", code: "ARG", model: 0.235, crowd: 0.215 },
  { team: "France", code: "FRA", model: 0.193, crowd: 0.20 },
  { team: "England", code: "ENG", model: 0.118, crowd: 0.10 },
];

// label helper for a match outcome
const outcomeLabel = (m, k) =>
  k === "home" ? `${m.home.name} win` : k === "draw" ? "Draw" : `${m.away.name} win`;

// ---- build the ranked Edges feed (find_edge shape) ----
function buildEdges(matches = MATCHES) {
  const rows = [];
  // best value outcome from each match
  matches.forEach((m) => {
    const k = m.best;
    const edge = m.edge[k];
    rows.push({
      kind: "match",
      matchId: m.id,
      market: `${m.home.code.toLowerCase()}-${m.away.code.toLowerCase()}-${k === "home" ? "home" : k === "away" ? "away" : "draw"}-win`,
      title: outcomeLabel(m, k),
      sub: `${m.home.code} v ${m.away.code} · ${m.comp.split(" · ")[0]}`,
      model_prob: m.model[k],
      market_price: m.crowd[k],
      edge,
      verdict: edge > 0 ? "VALUE_BUY" : "NO_EDGE",
      side: k,
    });
  });
  // outright title markets
  OUTRIGHTS.forEach((o) => {
    rows.push({
      kind: "outright",
      team: o.team,
      market: `will-${o.team.toLowerCase()}-win-the-2026-world-cup`,
      title: `${o.team} to win it all`,
      sub: "Outright · 2026 World Cup",
      model_prob: o.model,
      market_price: o.crowd,
      edge: o.model - o.crowd,
      verdict: o.model - o.crowd > 0 ? "VALUE_BUY" : "NO_EDGE",
      side: "Yes",
    });
  });
  // Kelly-ish stake fraction (quarter Kelly, capped), then $ from bankroll
  rows.forEach((r) => {
    const b = (1 - r.market_price) / r.market_price; // decimal-odds payout
    const p = r.model_prob;
    const kelly = (b * p - (1 - p)) / b;
    const frac = Math.max(0, Math.min(0.08, kelly * 0.25));
    r.suggested_stake_fraction = frac;
    r.suggested_stake = frac * WALLET.balance;
  });
  return rows
    .filter((r) => r.edge > 0.004)
    .sort((a, b) => b.edge - a.edge);
}

const EDGES = buildEdges();

// already-placed bets (My Bets)
const MY_BETS = [
  { market: "Spain win vs Germany", sub: "Filled Today 17:42 · 0.171 avg", stake: 289.0, price: 0.171, shares: 1690.1, status: "open", pnl: 0 },
  { market: "Brazil to reach final", sub: "Outright · filled Jun 02", stake: 150.0, price: 0.205, shares: 731.7, status: "open", pnl: 0 },
  { market: "Argentina win vs Mexico", sub: "Filled Today 16:10 · 0.158", stake: 120.0, price: 0.158, shares: 759.5, status: "open", pnl: 0 },
];

// ---- live wiring: replace mock figures with the real Rust engine ----
// Builds a match object from the engine's /api/board row, merged with the static
// presentation fields (crowd, adjustments, venue) from RAW_MATCHES.
function deriveFromEngine(m, eng) {
  const lh = eng.lambda_home, la = eng.lambda_away;
  const g = jointGrid(lh, la, 10);
  const heat = [];
  let peak = { h: 0, a: 0, p: 0 }, maxCell = 0;
  for (let h = 0; h <= 5; h++) {
    const row = [];
    for (let a = 0; a <= 5; a++) {
      const p = g[h][a];
      row.push(p);
      if (p > maxCell) maxCell = p;
      if (p > peak.p) peak = { h, a, p };
    }
    heat.push(row);
  }
  const model = { home: eng.p_home_win, draw: eng.p_draw, away: eng.p_away_win };
  const edge = {
    home: model.home - m.crowd.home,
    draw: model.draw - m.crowd.draw,
    away: model.away - m.crowd.away,
  };
  const best = ["home", "draw", "away"].reduce((b, k) => (edge[k] > edge[b] ? k : b), "home");
  const top_scorelines = (eng.top_scorelines || []).slice(0, 5);
  return {
    ...m,
    sims: eng.sims || 50000,
    model, edge, best,
    p_over_2_5: eng.p_over_2_5, p_btts: eng.p_btts,
    p_home_advance: eng.p_home_advance,
    expected_goals_home: lh, expected_goals_away: la,
    heat, heatMax: maxCell, peak, top_scorelines,
    live: true,
  };
}

// Fetch /api/board (same-origin when served by goal-digger-server). On success,
// rebuilds MATCHES + EDGES from real engine output and updates the window globals.
// On any failure, the mock data already in place stays — the UI never breaks.
async function loadLiveBoard() {
  try {
    const res = await fetch("/api/board", { cache: "no-store" });
    if (!res.ok) throw new Error("board " + res.status);
    const data = await res.json();
    const byId = {};
    (data.matches || []).forEach((e) => { byId[e.id] = e; });
    const live = RAW_MATCHES.map((m) => (byId[m.id] ? deriveFromEngine(m, byId[m.id]) : deriveMatch(m)));
    const liveEdges = buildEdges(live);
    Object.assign(window, { MATCHES: live, EDGES: liveEdges, GD_LIVE: true, __GD_BOARD__: data });
    return true;
  } catch (e) {
    Object.assign(window, { GD_LIVE: false });
    return false;
  }
}

// SLUG_MAP team name → RAW_MATCHES display name where they differ.
const SLUG_NAME_OVERRIDES = { "USA": "United States" };

// Fetch /api/prices — live Polymarket outright winner prices for all fixture
// teams. On success, patches OUTRIGHTS crowd prices and adds market_price to
// CHAMPIONSHIP entries, then rebuilds EDGES so the EdgesFeed shows real gaps.
// On any failure, hardcoded values stay — the UI never breaks.
async function loadLivePrices() {
  try {
    const res = await fetch("/api/prices", { cache: "no-store" });
    if (!res.ok) throw new Error("prices " + res.status);
    const data = await res.json();

    const prevPrices = window.__GD_LIVE_PRICES || {};
    const byTeam = {};
    const livePrices = {};
    (data.outrights || []).forEach((o) => {
      byTeam[o.team] = o.market_price;
      livePrices[o.team] = o.market_price;
    });
    // Add match-name aliases so MatchCard lookups by m.home.name work.
    Object.entries(SLUG_NAME_OVERRIDES).forEach(([slug, matchName]) => {
      if (livePrices[slug] != null) livePrices[matchName] = livePrices[slug];
    });

    // Detect price moves; noise floor 0.0005 filters rounding drift.
    const moves = {};
    Object.entries(livePrices).forEach(([team, price]) => {
      const prev = prevPrices[team];
      if (prev != null && Math.abs(price - prev) > 0.0005)
        moves[team] = { dir: price > prev ? "up" : "down", from: prev, to: price };
    });

    OUTRIGHTS.forEach((o) => { if (byTeam[o.team] != null) o.crowd = byTeam[o.team]; });
    CHAMPIONSHIP.forEach((c) => { if (byTeam[c.team] != null) c.market_price = byTeam[c.team]; });
    const liveEdges = buildEdges(window.MATCHES || MATCHES);

    Object.assign(window, {
      OUTRIGHTS, CHAMPIONSHIP, EDGES: liveEdges,
      GD_PRICES_LIVE: true,
      __GD_LIVE_PRICES: livePrices,
      __GD_PRICE_MOVES: moves,
      __GD_PRICES_UPDATED_AT: Date.now(),
    });

    // Auto-clear move indicators after the flash animation finishes (3.5s).
    if (Object.keys(moves).length > 0)
      setTimeout(() => { window.__GD_PRICE_MOVES = {}; }, 3500);

    return true;
  } catch (e) {
    Object.assign(window, { GD_PRICES_LIVE: false });
    return false;
  }
}

// Poll /api/prices every intervalMs and call onUpdate() whenever a fetch
// succeeds. Returns the interval ID so the caller can cancel if needed.
function startPricePolling(onUpdate, intervalMs = 20000) {
  return setInterval(async () => {
    const updated = await loadLivePrices();
    if (updated) onUpdate();
  }, intervalMs);
}

const fmtPct = (x, dp = 0) => (x * 100).toFixed(dp) + "%";
const fmtPrice = (x) => x.toFixed(3);
const fmtPts = (x) => (x >= 0 ? "+" : "−") + Math.abs(x * 100).toFixed(1);
const fmtUSD = (x) => "$" + x.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 });

Object.assign(window, {
  WALLET, MATCHES, CHAMPIONSHIP, OUTRIGHTS, EDGES, MY_BETS, FLAGS,
  outcomeLabel, fmtPct, fmtPrice, fmtPts, fmtUSD,
  loadLiveBoard, loadLivePrices, startPricePolling,
  GD_LIVE: false, GD_PRICES_LIVE: false,
  __GD_LIVE_PRICES: {}, __GD_PRICE_MOVES: {}, __GD_PRICES_UPDATED_AT: null,
});
