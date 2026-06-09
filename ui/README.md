# Goal Digger UI — Camel Brown / Zelda theme

Implemented from the Claude Design handoff (`goal digger2.html`, the Camel Brown
tab). Three-panel World Cup value terminal: left rail nav, center Board + Best-value
edges feed, right rail Aomi agent chat. Hero path: Board -> Match Detail -> Trade
(simulate-before-sign) modal.

## Two ways to run

- **`goal-digger-camel.html`** — single self-contained file. Open it directly in a
  browser (double-click). CSS, the three JSX modules, the pixel-footballer brand
  lockup, and the Aomi avatar (inlined as a data URI) are all bundled. React, Babel,
  and Lucide load from CDN, so it needs internet but no server.
  Verified: renders clean headlessly (board, edges, brand lockup, no console errors).

- **`goal digger2.html`** — the modular source (edit this). Loads `styles/*.css`,
  `src/brand-camel.js`, and `src/{data,components,views}.jsx`. Babel fetches the JSX
  over HTTP, so serve it rather than opening from file://:
  `cd ui && python3 -m http.server 8080` then open `http://localhost:8080/goal%20digger2.html`.
  After editing the modular files, rebuild the single file with `build_single.py` (below).

## Layout

```
ui/
├── goal-digger-camel.html   # portable single-file build (run this)
├── goal digger2.html        # modular entry (camel theme)
├── Goal Digger.html         # original pitch-green theme (tab 1, for reference)
├── src/
│   ├── data.jsx             # mock matches + Poisson model (matches simulate_match shape)
│   ├── components.jsx       # board, edges feed, rails, flags
│   ├── views.jsx            # Match Detail, Trade modal, Aomi chat, secondary views
│   └── brand-camel.js       # pixel-footballer logo kicking the real Aomi mark
├── styles/
│   ├── aomi-tokens.css      # Aomi design-system tokens
│   ├── goaldigger.css       # component styling (themeable via --gd-* tokens)
│   └── theme-camel.css      # Camel Brown / Zelda overrides
└── assets/aomi-symbol-pink.png
```

## Notes

- The data layer derives every figure (heatmap, W/D/L bars, scorelines, O/U, BTTS)
  from two Poisson lambdas, matching the real `simulate_match` output shape. When the
  Rust engine is wired in (via AomiFrame), swap `data.jsx`'s mock for live tool calls.
- The right-rail chat is the Aomi agent embed: it keeps Aomi's pink identity inside
  the Camel Brown skin. This is where the real runtime agent slots in.
- Themes are pure token overrides + a swappable brand config, so logic fixes flow to
  both tabs.
