/* ============================================================
   Goal Digger — UI components
   ============================================================ */

// ---- Icon (lucide-react via CDN — real React components, no DOM mutation) ----
const toPascalCase = (name) =>
  name.replace(/(^\w|-\w)/g, (c) => c.replace("-", "").toUpperCase());

const Icon = ({ name, size = 18, className = "", style = {} }) => {
  const Cmp = window.LucideReact && window.LucideReact[toPascalCase(name)];
  if (!Cmp) return null;
  return <Cmp size={size} className={className} style={style} />;
};

// kept as a no-op so existing useLucide() call sites don't need to change
const useLucide = () => {};

// ---- Flag: abstract color bands, no emoji ----
const Flag = ({ code, w = 30, h = 21 }) => {
  const f = (window.FLAGS || {})[code] || { dir: "solid", bands: ["#3A3F5C"] };
  const special = {
    ENG: (
      <svg viewBox="0 0 30 21" width={w} height={h}>
        <rect width="30" height="21" fill="#fff" />
        <rect x="12" width="6" height="21" fill="#CF142B" />
        <rect y="7.5" width="30" height="6" fill="#CF142B" />
      </svg>
    ),
    JPN: (
      <svg viewBox="0 0 30 21" width={w} height={h}>
        <rect width="30" height="21" fill="#fff" />
        <circle cx="15" cy="10.5" r="6" fill="#BC002D" />
      </svg>
    ),
    MAR: (
      <svg viewBox="0 0 30 21" width={w} height={h}>
        <rect width="30" height="21" fill="#C1272D" />
        <path d="M15 6 l1.6 4.9 5.2 0 -4.2 3 1.6 4.9 -4.2-3 -4.2 3 1.6-4.9 -4.2-3 5.2 0z" fill="none" stroke="#006233" strokeWidth="1" />
      </svg>
    ),
  };
  if (special[code]) return <span className="flag" style={{ width: w, height: h }}>{special[code]}</span>;

  let bg;
  if (f.dir === "solid") bg = f.bands[0];
  else {
    const dir = f.dir === "v" ? "to right" : "to bottom";
    const n = f.bands.length;
    const stops = f.bands
      .map((c, i) => `${c} ${((i / n) * 100).toFixed(2)}%, ${c} ${(((i + 1) / n) * 100).toFixed(2)}%`)
      .join(", ");
    bg = `linear-gradient(${dir}, ${stops})`;
  }
  return <span className="flag" style={{ width: w, height: h, background: bg }} />;
};

const EdgeBadge = ({ edge, showWord }) => {
  const cls = edge > 0.004 ? "up" : edge < -0.004 ? "down" : "flat";
  return (
    <span className={`badge ${cls}`}>
      {cls === "up" && <Icon name="trending-up" size={12} />}
      {cls === "down" && <Icon name="trending-down" size={12} />}
      <span className="mono">{window.fmtPts(edge)}</span>
    </span>
  );
};

// ---- Match card ----
const MatchCard = ({ m, onOpen }) => {
  const rows = [
    { k: "home", label: m.home.name },
    { k: "draw", label: "Draw" },
    { k: "away", label: m.away.name },
  ];
  return (
    <button className="match" onClick={() => onOpen(m.id)}>
      <div className="match-top">
        <span className="match-comp">{m.comp}</span>
        <span className={`match-kick ${m.soon ? "soon" : ""}`}>{m.kickoff}</span>
      </div>
      <div className="teams">
        <div className="team">
          <Flag code={m.home.code} />
          <span className="team-name">{m.home.name}</span>
          <span className="team-code mono">{m.home.code}</span>
        </div>
        <div className="team">
          <Flag code={m.away.code} />
          <span className="team-name">{m.away.name}</span>
          <span className="team-code mono">{m.away.code}</span>
        </div>
      </div>
      <div className="outs">
        <div className="out" style={{ paddingBottom: 4, borderBottom: "none" }}>
          <span className="out-name" />
          <span className="out-col-h" style={{ justifySelf: "end" }}>Crowd</span>
          <span className="out-col-h" style={{ justifySelf: "end" }}>Model</span>
          <span className="out-col-h" style={{ justifySelf: "end" }}>Edge</span>
        </div>
        {rows.map((r) => (
          <div className="out" key={r.k}>
            <span className="out-name">{r.label}</span>
            <span className="out-num crowd mono" style={{ justifySelf: "end" }}>{window.fmtPrice(m.crowd[r.k])}</span>
            <span className="out-num model mono" style={{ justifySelf: "end" }}>{window.fmtPrice(m.model[r.k])}</span>
            <EdgeBadge edge={m.edge[r.k]} />
          </div>
        ))}
      </div>
    </button>
  );
};

const Board = ({ onOpen }) => (
  <div className="block">
    <div className="section-head">
      <h2 className="section-title serif">The Board</h2>
      <div className="section-meta">
        <span className="mono">{window.MATCHES.length}</span> matches
        <span>·</span>
        <span className="mono">50,000</span> sims each
      </div>
    </div>
    <p className="section-sub">
      Crowd is the live Polymarket price. Model is our simulated probability. Green marks an outcome the crowd has underpriced.
    </p>
    <div className="board">
      {window.MATCHES.map((m) => <MatchCard key={m.id} m={m} onOpen={onOpen} />)}
    </div>
  </div>
);

// ---- Edges feed ----
const EdgesFeed = ({ onPlaceBet, onOpen }) => (
  <div className="block">
    <div className="section-head">
      <h2 className="section-title serif">Best value today</h2>
      <div className="section-meta">ranked by edge · model − crowd</div>
    </div>
    <div className="edges">
      <div className="edges-head">
        <span>Market</span>
        <span className="r">Model</span>
        <span className="r">Crowd</span>
        <span className="r">Edge</span>
        <span className="r">Stake</span>
        <span />
      </div>
      {window.EDGES.map((e, i) => (
        <div className="edge-row" key={i}>
          <div
            className="edge-mkt"
            style={{ cursor: e.kind === "match" ? "pointer" : "default" }}
            onClick={() => e.kind === "match" && onOpen(e.matchId)}
          >
            <div className="m1">{e.title}</div>
            <div className="m2 mono">{e.sub}</div>
          </div>
          <div className="edge-cell mono">{window.fmtPct(e.model_prob, 1)}</div>
          <div className="edge-cell muted mono">{window.fmtPct(e.market_price, 1)}</div>
          <div className={`edge-edge mono ${e.edge >= 0 ? "up" : "down"}`}>{window.fmtPts(e.edge)}</div>
          <div className="edge-stake mono">{window.fmtUSD(e.suggested_stake)}</div>
          <button className="btn btn-primary btn-sm" onClick={() => onPlaceBet({
            label: e.title, sub: e.sub, side: typeof e.side === "string" && e.side.length <= 4 ? e.side : "Yes",
            price: e.market_price, stake: e.suggested_stake, model: e.model_prob, edge: e.edge,
          })}>Place bet</button>
        </div>
      ))}
    </div>
  </div>
);

// ---- Left rail ----
const LeftRail = ({ active, setActive }) => {
  const items = [
    { k: "board", label: "Board", icon: "layout-grid" },
    { k: "edges", label: "Edges", icon: "trending-up", count: window.EDGES.length },
    { k: "mybets", label: "My Bets", icon: "receipt-text", count: window.MY_BETS.length },
    { k: "live", label: "Live", icon: "radio", live: true },
  ];
  return (
    <aside className="rail-left">
      <div>
        {window.GD_BRAND ? (
          <div className="brand brand-custom" dangerouslySetInnerHTML={{ __html: window.GD_BRAND.html }} />
        ) : (
          <div className="brand">
            <svg className="brand-mark" viewBox="0 0 32 32" fill="none">
              <circle cx="16" cy="16" r="13" stroke="currentColor" strokeWidth="1.4" opacity="0.35" />
              <path d="M16 3 a13 13 0 0 1 11.3 19.5" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" />
              <circle cx="16" cy="16" r="4.4" fill="currentColor" />
            </svg>
            <div>
              <div className="brand-name">Goal<span>Digger</span></div>
            </div>
          </div>
        )}
      </div>

      <nav className="nav">
        <div className="nav-label">Terminal</div>
        {items.map((it) => (
          <button key={it.k} className={`nav-item ${active === it.k ? "active" : ""}`} onClick={() => setActive(it.k)}>
            <Icon name={it.icon} />
            <span>{it.label}</span>
            {it.live ? <span className="live-dot" /> : it.count != null ? <span className="nav-count mono">{it.count}</span> : null}
          </button>
        ))}
      </nav>

      <div className="rail-spacer" />

      <div className="wallet">
        <div className="wallet-top">
          <span className="wallet-dot" />
          <span className="wallet-label">Wallet connected</span>
          <span className="wallet-addr mono">{window.WALLET.address}</span>
        </div>
        <div className="wallet-bal">
          <span className="amt mono">{window.WALLET.balance.toLocaleString("en-US", { minimumFractionDigits: 2 })}</span>
          <span className="cur mono">USDC</span>
        </div>
        <div className="wallet-foot">No custody · simulate before signing</div>
      </div>
    </aside>
  );
};

// ============================================================
//  Match Detail
// ============================================================
const ProbBars = ({ m }) => {
  const rows = [
    { k: "home", label: `${m.home.name} win`, cls: "win" },
    { k: "draw", label: "Draw", cls: "draw" },
    { k: "away", label: `${m.away.name} win`, cls: "loss" },
  ];
  return (
    <div>
      {rows.map((r) => (
        <div className="pbar-row" key={r.k}>
          <div className="pbar-top">
            <span className="pbar-name">{r.label}</span>
            <span>
              <span className="pbar-val mono">{window.fmtPct(m.model[r.k], 1)}</span>
              <span className="pbar-crowd mono">crowd {window.fmtPct(m.crowd[r.k], 0)}</span>
            </span>
          </div>
          <div className="pbar-track">
            <div className={`pbar-fill ${r.cls}`} style={{ width: window.fmtPct(m.model[r.k]) }} />
            <div className="pbar-crowdmark" style={{ left: `calc(${window.fmtPct(m.crowd[r.k])} - 1px)` }} title="crowd price" />
          </div>
        </div>
      ))}
    </div>
  );
};

const Heatmap = ({ m }) => (
  <div className="heat-wrap">
    <div className="heat-ylab">{m.home.code} goals</div>
    <div className="heat">
      {m.heat.map((row, h) =>
        row.map((p, a) => {
          const intensity = p / m.heatMax;
          const isPeak = h === m.peak.h && a === m.peak.a;
          return (
            <div
              key={`${h}-${a}`}
              className={`heat-cell ${isPeak ? "peak" : ""}`}
              style={{ background: `rgba(var(--gd-green-rgb),${(0.04 + intensity * 0.82).toFixed(3)})` }}
              title={`${m.home.code} ${h} – ${a} ${m.away.code} · ${window.fmtPct(p, 1)}`}
            >
              {p >= 0.01 ? (p * 100).toFixed(0) : ""}
            </div>
          );
        })
      )}
    </div>
    <div className="heat-corner" />
    <div>
      <div className="heat-axisx">{[0, 1, 2, 3, 4, 5].map((n) => <span key={n}>{n}</span>)}</div>
      <div className="heat-xlab">{m.away.code} goals →</div>
    </div>
  </div>
);

const Scorelines = ({ m }) => {
  const top = m.top_scorelines[0].prob;
  return (
    <div>
      {m.top_scorelines.map((s, i) => (
        <div className="scoreline" key={i}>
          <span className="score-val">{s.home}–{s.away}</span>
          <span className="score-track"><span className="score-fill" style={{ width: `${(s.prob / top) * 100}%` }} /></span>
          <span className="score-prob">{window.fmtPct(s.prob, 1)}</span>
        </div>
      ))}
    </div>
  );
};

const Adjustments = ({ m }) => (
  <div>
    {m.adjustments.map((a, i) => {
      const [head, tail] = a.reason.split(" · ");
      return (
        <div className="adj" key={i}>
          <span className={`adj-dot ${a.dir}`} />
          <div className="adj-body">
            <div className="adj-team">{a.team}</div>
            <div className="adj-reason">{head}{tail ? <> · <span className="mono">{tail}</span></> : null}</div>
          </div>
        </div>
      );
    })}
  </div>
);

const StatTiles = ({ m }) => (
  <div className="stat-tiles">
    <div className="stat-tile">
      <div className="st-label">Over / Under 2.5 goals</div>
      <div className="st-val mono">{window.fmtPct(m.p_over_2_5, 0)} <span style={{ fontSize: 13, color: "var(--gd-fg-3)" }}>over</span></div>
      <div className="st-split">
        <i style={{ width: window.fmtPct(m.p_over_2_5), background: "var(--gd-green)" }} />
        <i style={{ width: window.fmtPct(1 - m.p_over_2_5), background: "var(--gd-fg-3)" }} />
      </div>
      <div className="st-legend"><span>Over {window.fmtPct(m.p_over_2_5, 0)}</span><span>Under {window.fmtPct(1 - m.p_over_2_5, 0)}</span></div>
    </div>
    <div className="stat-tile">
      <div className="st-label">Both teams to score</div>
      <div className="st-val mono">{window.fmtPct(m.p_btts, 0)} <span style={{ fontSize: 13, color: "var(--gd-fg-3)" }}>yes</span></div>
      <div className="st-split">
        <i style={{ width: window.fmtPct(m.p_btts), background: "var(--gd-green)" }} />
        <i style={{ width: window.fmtPct(1 - m.p_btts), background: "var(--gd-fg-3)" }} />
      </div>
      <div className="st-legend"><span>Yes {window.fmtPct(m.p_btts, 0)}</span><span>No {window.fmtPct(1 - m.p_btts, 0)}</span></div>
    </div>
  </div>
);

const MatchDetail = ({ m, onClose, onPlaceBet }) => {
  React.useEffect(() => {
    const h = (e) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", h);
    return () => window.removeEventListener("keydown", h);
  }, []);
  const bk = m.best;
  const bestLabel = window.outcomeLabel(m, bk);
  const edgePts = m.edge[bk];
  return (
    <div className="scrim" onClick={onClose}>
      <div className="sheet" onClick={(e) => e.stopPropagation()}>
        <div className="detail-head">
          <div style={{ flex: 1 }}>
            <div className="detail-matchup">
              <div className="detail-team"><Flag code={m.home.code} w={34} h={24} /><span className="team-name">{m.home.name}</span></div>
              <span className="detail-vs">v</span>
              <div className="detail-team"><Flag code={m.away.code} w={34} h={24} /><span className="team-name">{m.away.name}</span></div>
            </div>
            <div className="detail-sub">{m.comp} · {m.kickoff} · {m.venue}</div>
            <div className="sim-tag">
              <Icon name="cpu" size={13} />
              Simulated <span className="mono">{m.sims.toLocaleString()}</span> times · λ
              <span className="mono">{m.lambda_home.toFixed(2)}</span> /
              <span className="mono">{m.lambda_away.toFixed(2)}</span>
            </div>
          </div>
          <button className="icon-close" onClick={onClose} aria-label="Close"><Icon name="x" size={17} /></button>
        </div>

        <div className="detail-grid">
          <div className="detail-col">
            <div className="subblock">
              <div className="panel-label">Match result</div>
              <ProbBars m={m} />
            </div>
            <div className="subblock">
              <div className="panel-label">Scoreline probability — {m.home.code} goals × {m.away.code} goals</div>
              <Heatmap m={m} />
            </div>
            <div className="subblock">
              <StatTiles m={m} />
            </div>
          </div>

          <div className="detail-col">
            <div className="subblock">
              <div className="panel-label">Most likely scorelines</div>
              <Scorelines m={m} />
            </div>
            <div className="subblock">
              <div className="panel-label">Adjustments applied</div>
              <Adjustments m={m} />
            </div>
          </div>
        </div>

        <div className="callout">
          <div className="callout-figs">
            <div className="callout-fig model">
              <div className="cf-label">Model</div>
              <div className="cf-val">{window.fmtPct(m.model[bk], 0)}</div>
            </div>
            <span className="callout-arrow"><Icon name="arrow-right" size={20} /></span>
            <div className="callout-fig">
              <div className="cf-label">Crowd</div>
              <div className="cf-val">{window.fmtPct(m.crowd[bk], 0)}</div>
            </div>
            <div className="callout-edge">
              <span className="ce-val">{window.fmtPts(edgePts)}</span>
              <span className="ce-label">point edge</span>
            </div>
          </div>
          <div className="callout-cta">
            <button className="btn btn-primary btn-lg" onClick={() => onPlaceBet({
              label: `${bestLabel} · ${m.home.code} v ${m.away.code}`, sub: m.comp,
              side: bk === "home" ? m.home.code : bk === "away" ? m.away.code : "Draw",
              price: m.crowd[bk], stake: 0.05 * window.WALLET.balance, model: m.model[bk], edge: edgePts,
            })}>
              Place bet on {bestLabel}
            </button>
            <span className="callout-note">Estimate from {m.sims.toLocaleString()} simulations. Not a promise.</span>
          </div>
        </div>
      </div>
    </div>
  );
};

window.GD = Object.assign(window.GD || {}, {
  Icon, useLucide, Flag, EdgeBadge, MatchCard, Board, EdgesFeed, LeftRail, MatchDetail,
});
