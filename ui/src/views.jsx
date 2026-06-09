/* ============================================================
   Goal Digger — Trade modal, Aomi agent chat, secondary views
   ============================================================ */

// ============================================================
//  Trade confirmation — simulate fill, THEN sign
// ============================================================
const TradeModal = ({ ctx, onClose }) => {
  const I = window.GD.Icon;
  window.GD.useLucide();
  const [stage, setStage] = React.useState("sim"); // sim → review → signing → done
  const fillPrice = ctx.price + 0.001; // modeled slippage
  const shares = ctx.stake / fillPrice;

  React.useEffect(() => {
    if (stage === "sim") { const t = setTimeout(() => setStage("review"), 1600); return () => clearTimeout(t); }
    if (stage === "signing") { const t = setTimeout(() => setStage("done"), 1100); return () => clearTimeout(t); }
  }, [stage]);

  React.useEffect(() => {
    const h = (e) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", h);
    return () => window.removeEventListener("keydown", h);
  }, []);

  return (
    <div className="scrim" onClick={onClose}>
      <div className="trade" onClick={(e) => e.stopPropagation()}>
        <div className="trade-head">
          <div className="trade-eyebrow">Place bet · Polymarket</div>
          <div className="trade-mkt">{ctx.label}</div>
          <div className="trade-side">
            <span>Buy</span>
            <span className="pill mono">{ctx.side}</span>
            <span style={{ color: "var(--gd-fg-3)" }}>· {ctx.sub}</span>
          </div>
          <button className="icon-close" onClick={onClose} aria-label="Close"><I name="x" size={17} /></button>
        </div>

        {stage === "done" ? (
          <div className="trade-body">
            <div className="done-check"><I name="check" size={26} /></div>
            <div className="done-title">Order signed</div>
            <div className="done-sub">Filled <span className="mono">{shares.toFixed(1)}</span> shares at <span className="mono">{fillPrice.toFixed(3)}</span>.</div>
            <div className="done-meta">
              <span>0x8a91…f2b4</span><span>·</span><span>{window.fmtUSD(ctx.stake)}</span><span>·</span><span>3.1s</span>
            </div>
          </div>
        ) : (
          <div className="trade-body">
            <div className="trade-rows">
              <div className="trade-r"><span className="k">Your stake</span><span className="v">{window.fmtUSD(ctx.stake)} <span style={{ color: "var(--gd-fg-3)" }}>USDC</span></span></div>
              <div className="trade-r"><span className="k">Crowd price</span><span className="v">{ctx.price.toFixed(3)}</span></div>
              <div className="trade-r"><span className="k">Model probability</span><span className="v" style={{ color: "var(--gd-green)" }}>{window.fmtPct(ctx.model, 1)}</span></div>
              <div className="trade-r"><span className="k">Edge</span><span className="v" style={{ color: ctx.edge >= 0 ? "var(--gd-green)" : "var(--gd-red)" }}>{window.fmtPts(ctx.edge)} pts</span></div>
            </div>

            <div className={`sim-box ${stage !== "sim" ? "ok" : ""}`}>
              {stage === "sim" ? (
                <>
                  <div className="sim-title"><span className="spin"><I name="loader" size={16} /></span> Simulating fill…</div>
                  <div className="sim-pending">Routing your order through the order book before anything is signed.</div>
                </>
              ) : (
                <>
                  <div className="sim-title"><I name="activity" size={16} /> Fill simulated</div>
                  <div className="sim-result">
                    Fills <span className="hl">{shares.toFixed(1)}</span> shares @ <span className="hl">{fillPrice.toFixed(3)}</span>
                    <br /><span className="muted">avg price · {window.fmtUSD(ctx.stake)} committed · max return {window.fmtUSD(shares)}</span>
                  </div>
                  <div className="sim-norevert"><I name="shield-check" size={13} /> No revert · slippage within tolerance</div>
                </>
              )}
            </div>
          </div>
        )}

        <div className="trade-foot">
          {stage === "review" && (
            <button className="btn btn-primary btn-lg btn-block" onClick={() => setStage("signing")}>
              <I name="wallet" size={17} /> Sign in wallet
            </button>
          )}
          {stage === "sim" && (
            <button className="btn btn-ghost btn-lg btn-block" disabled>
              <span className="spin" style={{ display: "inline-flex" }}><I name="loader" size={16} /></span> Simulating…
            </button>
          )}
          {stage === "signing" && (
            <button className="btn btn-primary btn-lg btn-block" disabled>
              <span className="spin" style={{ display: "inline-flex" }}><I name="loader" size={16} /></span> Waiting for signature…
            </button>
          )}
          {stage === "done" && (
            <button className="btn btn-ghost btn-lg btn-block" onClick={onClose}>Done</button>
          )}
          <div className="trade-foot-note"><I name="lock" size={13} /> Non-custodial. You sign every order.</div>
        </div>
      </div>
    </div>
  );
};

// ============================================================
//  Aomi agent chat (right rail)
// ============================================================
const N = (s) => <span className="num">{s}</span>;

function buildResponse(text, openMatch) {
  const t = text.toLowerCase();
  const E = window.EDGES, M = window.MATCHES, C = window.CHAMPIONSHIP;
  const spain = M.find((m) => m.id === "esp-ger");

  if (/simulat|spain|germany|breakdown/.test(t)) {
    return {
      tools: ["simulate_match"],
      content: (
        <>
          Spain v Germany, {N("50,000")} runs. Expected goals {N(spain.lambda_home.toFixed(2))} – {N(spain.lambda_away.toFixed(2))}.
          The model splits it <strong>{window.fmtPct(spain.model.home, 0)}</strong> Spain / {N(window.fmtPct(spain.model.draw, 0))} draw / <span className="neg">{window.fmtPct(spain.model.away, 0)}</span> Germany.
          Most likely score is {N(`${spain.peak.h}–${spain.peak.a}`)} at {N(window.fmtPct(spain.peak.p, 1))}. The crowd underprices Spain by <strong>{window.fmtPts(spain.edge.home)}</strong> points.
          <div style={{ marginTop: 10 }}>
            <button className="btn btn-ghost btn-sm" onClick={() => openMatch("esp-ger")}>Open full breakdown</button>
          </div>
        </>
      ),
    };
  }
  if (/value|edge|today|where|best|find/.test(t)) {
    const e = E[0];
    return {
      tools: ["find_edge", "simulate_match"],
      content: (
        <>
          The cleanest edge on the board is <strong>{e.title}</strong> — {e.sub}. The model lands at {N(window.fmtPct(e.model_prob, 1))}; the crowd is at {N(window.fmtPct(e.market_price, 1))}. That is a <strong>{window.fmtPts(e.edge)}</strong> point edge.
          I would keep it small, around {N(window.fmtPct(e.suggested_stake_fraction, 0))} of bankroll ({N(window.fmtUSD(e.suggested_stake))}). Several matches today show no edge, so this is a selective board.
          <div style={{ marginTop: 8, color: "var(--gd-fg-3)", fontSize: 12 }}>Probabilities are estimates, not promises.</div>
          {e.kind === "match" && (
            <div style={{ marginTop: 10 }}>
              <button className="btn btn-ghost btn-sm" onClick={() => openMatch(e.matchId)}>Open {e.sub.split(" · ")[0]}</button>
            </div>
          )}
        </>
      ),
    };
  }
  if (/win it all|champion|title|tournament|trophy|who wins|lift/.test(t)) {
    return {
      tools: ["simulate_tournament"],
      content: (
        <>
          Title odds after the latest run of {N("50,000")} tournaments:
          <div style={{ marginTop: 10, display: "flex", flexDirection: "column", gap: 7 }}>
            {C.map((c) => (
              <div key={c.code} style={{ display: "flex", alignItems: "center", gap: 9 }}>
                <window.GD.Flag code={c.code} w={22} h={15} />
                <span style={{ fontSize: 13, minWidth: 78 }}>{c.team}</span>
                <span style={{ flex: 1, height: 5, background: "rgba(250,246,241,0.06)", borderRadius: 999, overflow: "hidden" }}>
                  <span style={{ display: "block", height: "100%", width: window.fmtPct(c.title_probability / C[0].title_probability), background: "var(--gd-green)", opacity: 0.75, borderRadius: 999 }} />
                </span>
                <span className="num" style={{ fontSize: 12.5, width: 42, textAlign: "right" }}>{window.fmtPct(c.title_probability, 1)}</span>
              </div>
            ))}
          </div>
          <div style={{ marginTop: 9 }}>Spain leads, but the field is open past the semis. The outright on Spain still carries a small edge against the crowd.</div>
        </>
      ),
    };
  }
  if (/size|stake|how much|bankroll|kelly|risk/.test(t)) {
    const e = E[0];
    return {
      tools: ["find_edge"],
      content: (
        <>
          I size with a quarter-Kelly rule and cap any single position at {N("8%")} of bankroll. For {e.title} that comes to about {N(window.fmtPct(e.suggested_stake_fraction, 0))} — {N(window.fmtUSD(e.suggested_stake))} on your {N(window.fmtUSD(window.WALLET.balance))}.
          <div style={{ marginTop: 8 }}>Edge is necessary, not sufficient. Small and repeated beats large and rare.</div>
        </>
      ),
    };
  }
  return {
    tools: [],
    content: (
      <>
        I can simulate any match, rank today's edges, or run the full tournament. Try asking <em>"where's the value today?"</em> or open a match on the board.
      </>
    ),
  };
}

const ToolChips = ({ tools, running }) => {
  const I = window.GD.Icon;
  return (
    <div className="toolchips">
      {tools.map((name) => (
        <span className={`toolchip ${running ? "run" : ""}`} key={name}>
          {running ? <span className="spin"><I name="loader" size={12} /></span> : <I name="terminal" size={12} />}
          {name}
        </span>
      ))}
    </div>
  );
};

const RightRail = ({ onOpenMatch }) => {
  const I = window.GD.Icon;
  window.GD.useLucide();
  const [messages, setMessages] = React.useState([
    {
      role: "assistant",
      tools: [],
      content: (
        <>
          I simulate every World Cup match {N("50,000")} times and compare the result to the live Polymarket price. Ask me where the value is, or open any match for the full breakdown.
        </>
      ),
    },
  ]);
  const [input, setInput] = React.useState("");
  const [busy, setBusy] = React.useState(false);
  const scrollRef = React.useRef(null);

  React.useEffect(() => {
    if (scrollRef.current) scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
  });

  const send = (text) => {
    const q = (text ?? input).trim();
    if (!q || busy) return;
    setInput("");
    setBusy(true);
    const resp = buildResponse(q, onOpenMatch);
    setMessages((prev) => [
      ...prev,
      { role: "user", content: q },
      { role: "assistant", pending: true, tools: resp.tools },
    ]);
    setTimeout(() => {
      setMessages((prev) => {
        const copy = prev.slice();
        copy[copy.length - 1] = { role: "assistant", tools: resp.tools, content: resp.content };
        return copy;
      });
      setBusy(false);
    }, 1500);
  };

  const suggestions = ["where's the value today?", "simulate Spain v Germany", "who wins it all?", "how should I size it?"];

  return (
    <aside className="rail-right">
      <div className="agent-head">
        <img className="agent-avatar" src="assets/aomi-symbol-pink.png" alt="Aomi" />
        <div>
          <div className="agent-name"><em>Aomi</em> agent</div>
          <div className="agent-status"><span className="d" /> Connected to model · live prices</div>
        </div>
        <div className="agent-powered">natural language<br />on-chain</div>
      </div>

      <div className="chat" ref={scrollRef}>
        {messages.map((m, i) => (
          <div className={`msg ${m.role}`} key={i}>
            {m.role === "assistant" && m.tools && m.tools.length > 0 && (
              <ToolChips tools={m.tools} running={m.pending} />
            )}
            {m.pending ? (
              <div className="bubble" style={{ padding: 0 }}>
                <div className="typing"><i /><i /><i /></div>
              </div>
            ) : (
              <div className="bubble">{m.content}</div>
            )}
          </div>
        ))}
      </div>

      <div className="chat-foot">
        <div className="chat-suggest">
          {suggestions.map((s) => (
            <button className="suggest" key={s} onClick={() => send(s)} disabled={busy}>{s}</button>
          ))}
        </div>
        <div className="composer-bar">
          <input
            value={input}
            placeholder="Ask the model anything"
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && send()}
          />
          <button onClick={() => send()} disabled={!input.trim() || busy} aria-label="Send"><I name="arrow-up" size={16} /></button>
        </div>
      </div>
    </aside>
  );
};

// ============================================================
//  Secondary center views
// ============================================================
const MyBets = ({ onPlaceBet }) => (
  <div className="block">
    <div className="section-head">
      <h2 className="section-title serif">My Bets</h2>
      <div className="section-meta"><span className="mono">{window.MY_BETS.length}</span> open positions</div>
    </div>
    <p className="section-sub">Positions you signed from this wallet. Settlement follows the on-chain market resolution.</p>
    <div className="edges">
      <div className="edges-head" style={{ gridTemplateColumns: "minmax(0,1fr) 90px 78px 90px 90px" }}>
        <span>Position</span><span className="r">Price</span><span className="r">Shares</span><span className="r">Stake</span><span className="r">Status</span>
      </div>
      {window.MY_BETS.map((b, i) => (
        <div className="bet-row" key={i}>
          <div><div className="bm1">{b.market}</div><div className="bm2 mono">{b.sub}</div></div>
          <div className="bc">{b.price.toFixed(3)}</div>
          <div className="bc">{b.shares.toFixed(1)}</div>
          <div className="bc">{window.fmtUSD(b.stake)}</div>
          <span className={`bet-status ${b.status}`}>{b.status === "open" ? "Open" : "Won"}</span>
        </div>
      ))}
    </div>
  </div>
);

const LiveView = ({ onOpen }) => {
  const live = window.MATCHES.filter((m) => m.soon);
  return (
    <div className="block">
      <div className="section-head">
        <h2 className="section-title serif">Live</h2>
        <div className="section-meta"><span className="live-dot" style={{ position: "static" }} /> {live.length} kicking off soon</div>
      </div>
      <p className="section-sub">Matches starting within the hour. The model re-runs as line-ups confirm and prices move.</p>
      <div className="board">
        {live.map((m) => <window.GD.MatchCard key={m.id} m={m} onOpen={onOpen} />)}
      </div>
      {live.length === 0 && <div className="empty">Quiet for now. The board is your best view until kickoff.</div>}
    </div>
  );
};

window.GD = Object.assign(window.GD || {}, { TradeModal, RightRail, MyBets, LiveView });
