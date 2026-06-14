/* ============================================================
   Goal Digger — Game Policies feature
   A "standing rule" the agent watches and acts on.
   e.g. "Bet $100 on Portugal to win if Ronaldo is subbed off."

   Components:
     PoliciesView        — main view (create panel + active list)
     CreatePolicyPanel   — stake / match+outcome / trigger picker
     ActivePoliciesList  — policy cards with status + simulate button
     PolicyApprovalModal — fires when policy is triggered
   ============================================================ */

// ---- Mock squads for trigger player list ----
// Keyed by match id, matches RAW_MATCHES ids
const MATCH_SQUADS = {
  "esp-ger": {
    home: [
      "Unai Simón",
      "Dani Carvajal",
      "Laporte",
      "Le Normand",
      "Cucurella",
      "Pedri",
      "Rodri",
      "Fabián Ruiz",
      "Yamal",
      "Morata",
      "Williams",
    ],
    away: [
      "Neuer",
      "Kimmich",
      "Schlotterbeck",
      "Tah",
      "Raum",
      "Kroos",
      "Andrich",
      "Gündoğan",
      "Musiala",
      "Havertz",
      "Wirtz",
    ],
  },
  "arg-mex": {
    home: [
      "Dibu Martínez",
      "Molina",
      "Romero",
      "Lisandro",
      "Acuña",
      "De Paul",
      "Enzo",
      "Mac Allister",
      "Di María",
      "Lautaro",
      "Messi",
    ],
    away: [
      "Ochoa",
      "Sánchez",
      "Montes",
      "Moreno",
      "Gallardo",
      "Herrera",
      "Álvarez",
      "Romo",
      "Antuna",
      "Giménez",
      "Vega",
    ],
  },
  "fra-eng": {
    home: [
      "Maignan",
      "Koundé",
      "Saliba",
      "Upamecano",
      "Hernández",
      "Tchouaméni",
      "Camavinga",
      "Griezmann",
      "Dembélé",
      "Mbappé",
      "Thuram",
    ],
    away: [
      "Pickford",
      "Alexander-Arnold",
      "Stones",
      "Guehi",
      "Trippier",
      "Rice",
      "Bellingham",
      "Mainoo",
      "Saka",
      "Kane",
      "Foden",
    ],
  },
  "bra-ned": {
    home: [
      "Alisson",
      "Militão",
      "Marquinhos",
      "Gabriel",
      "Danilo",
      "Gerson",
      "Bruno Guimarães",
      "Lucas Paquetá",
      "Rodrygo",
      "Vinicius Jr",
      "Endrick",
    ],
    away: [
      "Verbruggen",
      "Dumfries",
      "De Vrij",
      "Van Dijk",
      "Blind",
      "Schouten",
      "Reijnders",
      "Gakpo",
      "Bergwijn",
      "Memphis",
      "Xavi Simons",
    ],
  },
  "por-uru": {
    home: [
      "Diogo Costa",
      "João Cancelo",
      "Rúben Dias",
      "Gonçalo Inácio",
      "Nuno Mendes",
      "João Palhinha",
      "Vitinha",
      "Bernardo",
      "Leão",
      "Ronaldo",
      "Bruno Fernandes",
    ],
    away: [
      "Rochet",
      "Nández",
      "Godín",
      "Olivera",
      "Viña",
      "Bentancur",
      "Ugarte",
      "Valverde",
      "Pellistri",
      "Darwin Núñez",
      "Araújo",
    ],
  },
  "cro-mar": {
    home: [
      "Livakovic",
      "Juranovic",
      "Gvardiol",
      "Šimunović",
      "Sosa",
      "Modrić",
      "Brozović",
      "Kovačić",
      "Kramarić",
      "Petković",
      "Budimir",
    ],
    away: [
      "Bono",
      "Hakimi",
      "Aguerd",
      "Dari",
      "Ounahi",
      "Amrabat",
      "Ziyech",
      "En-Nesyri",
      "Boufal",
      "Mazraoui",
      "Sabiri",
    ],
  },
  "usa-col": {
    home: [
      "Turner",
      "Dest",
      "Richards",
      "Carter-Vickers",
      "Robinson",
      "McKennie",
      "Adams",
      "Musah",
      "Pulisic",
      "Reyna",
      "Balogun",
    ],
    away: [
      "Vargas",
      "Muñoz",
      "Mina",
      "Lucumí",
      "Mojica",
      "Lerma",
      "Ríos",
      "Carrascal",
      "Cuadrado",
      "Borja",
      "Díaz",
    ],
  },
  "jpn-sen": {
    home: [
      "Gonda",
      "Sugawara",
      "Itakura",
      "Yoshida",
      "Nagatomo",
      "Endo",
      "Morita",
      "Doan",
      "Minamino",
      "Ueda",
      "Kamada",
    ],
    away: [
      "Gomis",
      "Sabaly",
      "Koulibaly",
      "Niakhate",
      "Mendy",
      "Gueye",
      "Kouyaté",
      "Diatta",
      "Mané",
      "Dia",
      "Dieng",
    ],
  },
};

const TRIGGER_TYPES = [
  { value: "subbed_off", label: "Subbed off", icon: "arrow-down-left" },
  { value: "scores", label: "Scores", icon: "goal-net" },
  { value: "sent_off", label: "Sent off", icon: "square-x" },
];

// For trigger type display label
const triggerLabel = (type) =>
  TRIGGER_TYPES.find((t) => t.value === type)?.label ?? type;

// Outcome label helper (re-uses same logic as parent)
function policyOutcomeLabel(match, outcome) {
  if (outcome === "home") return `${match.home.name} win`;
  if (outcome === "away") return `${match.away.name} win`;
  return "Draw";
}

// ---- stub POST /api/policies ----
async function apiCreatePolicy(payload) {
  try {
    const res = await fetch("/api/policies", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    if (res.ok) return await res.json();
  } catch (_) {}
  // Fallback: generate a local id so the UI never breaks
  return {
    id: "pol_" + Math.random().toString(36).slice(2, 8),
    status: "watching",
  };
}

// ---- stub GET /api/policies ----
async function apiFetchPolicies() {
  try {
    const res = await fetch("/api/policies", { cache: "no-store" });
    if (res.ok) return await res.json();
  } catch (_) {}
  return null;
}

// ---- Policy polling: same pattern as price poller ----
function startPolicyPolling(
  getPolicies,
  setPolicies,
  onTriggered,
  intervalMs = 8000,
) {
  return setInterval(async () => {
    const data = await apiFetchPolicies();
    if (!data) return;
    const list = Array.isArray(data) ? data : data.policies || [];
    setPolicies((prev) => {
      const next = list.map((remote) => {
        const local = prev.find((p) => p.id === remote.id) || {};
        return { ...local, ...remote };
      });
      // detect newly triggered
      next.forEach((p) => {
        const was = prev.find((q) => q.id === p.id);
        if (p.status === "triggered" && was && was.status === "watching") {
          onTriggered(p);
        }
      });
      return next;
    });
  }, intervalMs);
}

// ============================================================
//  PolicyApprovalModal
//  Props: policy, onApprove(tradeCtx), onDismiss
// ============================================================
const PolicyApprovalModal = ({ policy, onApprove, onDismiss }) => {
  const I = window.GD.Icon;
  const match = window.MATCHES.find((m) => m.id === policy.matchId);
  if (!match) return null;

  const outcomeLabel = policyOutcomeLabel(match, policy.outcome);

  // Use live price if available, else fall back to model crowd price
  const crowdPrice = policy.currentPrice ?? match.crowd[policy.outcome] ?? 0.5;
  const modelProb = policy.modelProb ?? match.model[policy.outcome] ?? 0.5;
  const edge = modelProb - crowdPrice;
  const edgePts = edge;

  // Elapsed comes from policy (backend sets it when triggered), or from simulate
  const elapsed = policy.triggerElapsed ?? policy._simulateElapsed ?? "66";

  const tradeCtx = {
    label: `${outcomeLabel} · ${match.home.code} v ${match.away.code}`,
    sub: match.comp,
    side:
      policy.outcome === "home"
        ? match.home.code
        : policy.outcome === "away"
          ? match.away.code
          : "Draw",
    price: crowdPrice,
    stake: policy.stake,
    model: modelProb,
    edge: edgePts,
  };

  return (
    <div className="scrim" onClick={onDismiss}>
      <div className="policy-approval" onClick={(e) => e.stopPropagation()}>
        {/* Header */}
        <div className="pa-head">
          <div className="pa-trigger-badge">
            <span className="pa-trigger-pulse" />
            Trigger fired
          </div>
          <button
            className="icon-close"
            onClick={onDismiss}
            aria-label="Dismiss"
          >
            <I name="x" size={17} />
          </button>
        </div>

        {/* Trigger event */}
        <div className="pa-event">
          <div className="pa-event-icon">
            <I name="arrow-down-left" size={20} />
          </div>
          <div className="pa-event-body">
            <div className="pa-event-title">
              <strong>{policy.playerName}</strong> is off ({elapsed}′)
            </div>
            <div className="pa-event-sub">
              {match.home.name} v {match.away.name} · {match.comp}
            </div>
          </div>
        </div>

        {/* Trade summary */}
        <div className="pa-trade-card">
          <div className="pa-trade-label">Your standing order</div>
          <div className="pa-trade-desc">
            Place{" "}
            <span className="pa-money">{window.fmtUSD(policy.stake)}</span> on{" "}
            <span className="pa-outcome">{outcomeLabel}</span>
          </div>
          <div className="pa-figs">
            <div className="pa-fig">
              <div className="pa-fig-label">Crowd price</div>
              <div className="pa-fig-val mono">{crowdPrice.toFixed(2)}</div>
            </div>
            <div className="pa-fig pa-fig--model">
              <div className="pa-fig-label">Model</div>
              <div
                className="pa-fig-val mono"
                style={{ color: "var(--gd-green)" }}
              >
                {window.fmtPct(modelProb, 1)}
              </div>
            </div>
            <div className="pa-fig">
              <div className="pa-fig-label">Edge</div>
              <div
                className="pa-fig-val mono"
                style={{
                  color: edgePts >= 0 ? "var(--gd-green)" : "var(--gd-red)",
                }}
              >
                {window.fmtPts(edgePts)} pts
              </div>
            </div>
          </div>
        </div>

        {/* CTA */}
        <div className="pa-foot">
          <button
            className="btn btn-primary btn-lg"
            style={{ flex: 1 }}
            onClick={() => onApprove(tradeCtx)}
          >
            <I name="check" size={17} />
            Approve &amp; place{" "}
            <span className="mono">{window.fmtUSD(policy.stake)}</span>
          </button>
          <button
            className="btn btn-ghost btn-lg"
            style={{ flex: 1 }}
            onClick={onDismiss}
          >
            Dismiss
          </button>
        </div>
        <div className="pa-foot-note">
          <I name="lock" size={12} />
          Runs through simulate-before-sign. You confirm in wallet.
        </div>
      </div>
    </div>
  );
};

// ============================================================
//  PolicyCard — individual card in active list
// ============================================================
const PolicyCard = ({ policy, onSimulate, onRemove }) => {
  const I = window.GD.Icon;
  const match = window.MATCHES.find((m) => m.id === policy.matchId);

  const STATUS_META = {
    watching: { label: "Watching", cls: "watching", icon: "radio" },
    triggered: { label: "Triggered", cls: "triggered", icon: "zap" },
    placed: { label: "Placed", cls: "placed", icon: "check-circle" },
    expired: { label: "Expired", cls: "expired", icon: "clock" },
  };
  const meta = STATUS_META[policy.status] || STATUS_META.watching;

  return (
    <div className={`policy-card ${meta.cls}`}>
      {/* Status row */}
      <div className="pc-status-row">
        <div className={`pc-status-badge ${meta.cls}`}>
          {policy.status === "watching" && <span className="pc-watch-dot" />}
          <I name={meta.icon} size={12} />
          {meta.label}
        </div>
        <button
          className="pc-remove"
          onClick={() => onRemove(policy.id)}
          title="Remove policy"
          aria-label="Remove"
        >
          <I name="x" size={13} />
        </button>
      </div>

      {/* Policy description */}
      <div className="pc-rule">
        <div className="pc-stake mono">{window.fmtUSD(policy.stake)}</div>
        <div className="pc-on">on</div>
        <div className="pc-outcome">
          {match ? policyOutcomeLabel(match, policy.outcome) : policy.outcome}
        </div>
      </div>

      {/* Trigger */}
      <div className="pc-trigger">
        <I name="zap" size={12} style={{ color: "var(--gd-fg-3)" }} />
        <span className="pc-trigger-text">
          if <strong>{policy.playerName}</strong>{" "}
          {triggerLabel(policy.triggerType).toLowerCase()}
        </span>
      </div>

      {/* Match info */}
      {match && (
        <div className="pc-match">
          {match.home.code} v {match.away.code}
          <span className="pc-match-kick">{match.kickoff}</span>
        </div>
      )}

      {/* Simulate trigger button — dev tool, only visible when watching */}
      {policy.status === "watching" && (
        <button
          className="btn btn-ghost btn-sm pc-simulate"
          onClick={() => onSimulate(policy.id)}
        >
          <I name="play" size={12} />
          Simulate trigger
        </button>
      )}
    </div>
  );
};

// ============================================================
//  CreatePolicyPanel
// ============================================================
const CreatePolicyPanel = ({ onCreated }) => {
  const I = window.GD.Icon;
  const [stake, setStake] = React.useState("100");
  const [matchId, setMatchId] = React.useState(window.MATCHES[0]?.id ?? "");
  const [outcome, setOutcome] = React.useState("home");
  const [playerName, setPlayer] = React.useState("");
  const [triggerType, setTrigger] = React.useState("subbed_off");
  const [busy, setBusy] = React.useState(false);
  const [err, setErr] = React.useState(null);

  const selectedMatch = window.MATCHES.find((m) => m.id === matchId);
  const squad = selectedMatch
    ? [
        ...(MATCH_SQUADS[matchId]?.home ?? []),
        ...(MATCH_SQUADS[matchId]?.away ?? []),
      ]
    : [];

  // Reset player when match changes
  React.useEffect(() => {
    setPlayer("");
  }, [matchId]);

  const stakeNum = parseFloat(stake);
  const valid =
    selectedMatch &&
    outcome &&
    playerName &&
    triggerType &&
    !isNaN(stakeNum) &&
    stakeNum > 0;

  const handleCreate = async () => {
    if (!valid) return;
    setBusy(true);
    setErr(null);
    try {
      const payload = {
        stake: stakeNum,
        matchId,
        outcome,
        triggerType,
        playerName,
      };
      const result = await apiCreatePolicy(payload);
      const newPolicy = {
        id: result.id,
        status: result.status || "watching",
        stake: stakeNum,
        matchId,
        outcome,
        triggerType,
        playerName,
        createdAt: Date.now(),
      };
      onCreated(newPolicy);
      // Reset form (keep match/stake for quick sequential policies)
      setPlayer("");
      setOutcome("home");
    } catch (e) {
      setErr("Failed to create policy. Try again.");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="create-policy-panel">
      <div className="cp-head">
        <div className="panel-label">New policy</div>
        <div className="cp-head-sub">
          Set a standing rule. We watch — you approve.
        </div>
      </div>

      {/* Stake */}
      <div className="cp-field">
        <label className="cp-label">Stake (USDC)</label>
        <div className="cp-input-wrap">
          <span className="cp-prefix">$</span>
          <input
            className="cp-input mono"
            type="number"
            min="1"
            step="10"
            value={stake}
            onChange={(e) => setStake(e.target.value)}
            placeholder="100"
          />
        </div>
      </div>

      {/* Match + outcome */}
      <div className="cp-field">
        <label className="cp-label">Match</label>
        <select
          className="cp-select"
          value={matchId}
          onChange={(e) => setMatchId(e.target.value)}
        >
          {window.MATCHES.map((m) => (
            <option key={m.id} value={m.id}>
              {m.home.name} v {m.away.name} · {m.kickoff}
            </option>
          ))}
        </select>
      </div>

      <div className="cp-field">
        <label className="cp-label">Bet on outcome</label>
        <div className="cp-outcomes">
          {selectedMatch &&
            [
              { k: "home", label: selectedMatch.home.name },
              { k: "draw", label: "Draw" },
              { k: "away", label: selectedMatch.away.name },
            ].map((o) => (
              <button
                key={o.k}
                className={`cp-outcome-btn ${outcome === o.k ? "active" : ""}`}
                onClick={() => setOutcome(o.k)}
                type="button"
              >
                {o.label}
                {outcome === o.k && selectedMatch && (
                  <span className="cp-outcome-price mono">
                    {window.fmtPrice(selectedMatch.crowd[o.k])}
                  </span>
                )}
              </button>
            ))}
        </div>
      </div>

      {/* Trigger */}
      <div className="cp-trigger-row">
        <div className="cp-field cp-field--half">
          <label className="cp-label">Trigger player</label>
          <select
            className="cp-select"
            value={playerName}
            onChange={(e) => setPlayer(e.target.value)}
          >
            <option value="">— pick player —</option>
            {selectedMatch && (
              <optgroup label={selectedMatch.home.name}>
                {(MATCH_SQUADS[matchId]?.home ?? []).map((p) => (
                  <option key={p} value={p}>
                    {p}
                  </option>
                ))}
              </optgroup>
            )}
            {selectedMatch && (
              <optgroup label={selectedMatch.away.name}>
                {(MATCH_SQUADS[matchId]?.away ?? []).map((p) => (
                  <option key={p} value={p}>
                    {p}
                  </option>
                ))}
              </optgroup>
            )}
          </select>
        </div>

        <div className="cp-field cp-field--half">
          <label className="cp-label">Event</label>
          <select
            className="cp-select"
            value={triggerType}
            onChange={(e) => setTrigger(e.target.value)}
          >
            {TRIGGER_TYPES.map((t) => (
              <option key={t.value} value={t.value}>
                {t.label}
              </option>
            ))}
          </select>
        </div>
      </div>

      {/* Policy preview */}
      {valid && (
        <div className="cp-preview">
          <I
            name="eye"
            size={13}
            style={{ color: "var(--gd-fg-3)", flexShrink: 0 }}
          />
          <span>
            Bet <strong className="mono">{window.fmtUSD(stakeNum)}</strong> on{" "}
            <strong>{policyOutcomeLabel(selectedMatch, outcome)}</strong> if{" "}
            <strong>{playerName}</strong>{" "}
            {triggerLabel(triggerType).toLowerCase()}
          </span>
        </div>
      )}

      {err && (
        <div className="cp-error">
          <I name="alert-triangle" size={13} />
          {err}
        </div>
      )}

      <button
        className="btn btn-primary btn-block"
        onClick={handleCreate}
        disabled={!valid || busy}
      >
        {busy ? (
          <>
            <span className="spin" style={{ display: "inline-flex" }}>
              <I name="loader" size={15} />
            </span>{" "}
            Creating…
          </>
        ) : (
          <>
            <I name="plus" size={15} /> Create policy
          </>
        )}
      </button>
    </div>
  );
};

// ============================================================
//  ActivePoliciesList
// ============================================================
const ActivePoliciesList = ({ policies, onSimulate, onRemove }) => {
  const I = window.GD.Icon;

  if (!policies.length) {
    return (
      <div className="policies-empty">
        <I
          name="shield"
          size={28}
          style={{ color: "var(--gd-fg-3)", opacity: 0.45 }}
        />
        <div className="policies-empty-title">No active policies</div>
        <div className="policies-empty-sub">
          Create one and we'll watch the match for you.
        </div>
      </div>
    );
  }

  const watching = policies.filter((p) => p.status === "watching");
  const triggered = policies.filter((p) => p.status === "triggered");
  const placed = policies.filter((p) => p.status === "placed");
  const expired = policies.filter((p) => p.status === "expired");

  const Section = ({ title, items }) =>
    !items.length ? null : (
      <div className="apl-section">
        <div className="apl-section-label">{title}</div>
        <div className="apl-grid">
          {items.map((p) => (
            <PolicyCard
              key={p.id}
              policy={p}
              onSimulate={onSimulate}
              onRemove={onRemove}
            />
          ))}
        </div>
      </div>
    );

  return (
    <div className="active-policies-list">
      <Section title="Triggered" items={triggered} />
      <Section title="Watching" items={watching} />
      <Section title="Placed" items={placed} />
      <Section title="Expired" items={expired} />
    </div>
  );
};

// ============================================================
//  PoliciesView — the full center view
// ============================================================
const PoliciesView = ({ onPlaceBet }) => {
  const [policies, setPolicies] = React.useState([]);
  const [pendingApproval, setPendingApproval] = React.useState(null); // policy to approve

  // Polling effect
  React.useEffect(() => {
    // initial fetch
    apiFetchPolicies().then((data) => {
      if (!data) return;
      const list = Array.isArray(data) ? data : data.policies || [];
      setPolicies(list);
    });

    const id = startPolicyPolling(
      () => policies,
      setPolicies,
      (triggered) => {
        // Only pop one approval at a time; queue handled by next poll cycle
        setPendingApproval((prev) => (prev ? prev : triggered));
      },
    );
    return () => clearInterval(id);
  }, []);

  const handleCreated = (newPolicy) => {
    setPolicies((prev) => [newPolicy, ...prev]);
  };

  // "Simulate trigger" dev button
  const handleSimulate = (policyId) => {
    setPolicies((prev) =>
      prev.map((p) => {
        if (p.id !== policyId) return p;
        const sim = {
          ...p,
          status: "triggered",
          _simulateElapsed: String(55 + Math.floor(Math.random() * 30)),
        };
        // Immediately pop approval modal
        setPendingApproval(sim);
        return sim;
      }),
    );
  };

  const handleRemove = (policyId) => {
    setPolicies((prev) => prev.filter((p) => p.id !== policyId));
  };

  const handleApprove = (tradeCtx) => {
    // Mark policy as placed
    setPolicies((prev) =>
      prev.map((p) =>
        p.id === pendingApproval?.id ? { ...p, status: "placed" } : p,
      ),
    );
    setPendingApproval(null);
    // Run the existing simulate-before-sign trade flow
    onPlaceBet(tradeCtx);
  };

  const handleDismiss = () => {
    // Leave as triggered (user declined this time)
    setPendingApproval(null);
  };

  return (
    <div className="block policies-view">
      <div className="section-head">
        <h2 className="section-title serif">Policies</h2>
        <div className="section-meta">
          <span className="policies-count mono">
            {policies.filter((p) => p.status === "watching").length}
          </span>{" "}
          watching
        </div>
      </div>
      <p className="section-sub">
        Standing rules the agent watches and acts on. When a trigger fires you
        approve the bet—nothing signs without you.
      </p>

      <div className="policies-layout">
        {/* Left: create panel */}
        <div className="policies-create-col">
          <CreatePolicyPanel onCreated={handleCreated} />
        </div>

        {/* Right: active list */}
        <div className="policies-list-col">
          <div className="panel-label" style={{ marginBottom: 16 }}>
            Active policies
          </div>
          <ActivePoliciesList
            policies={policies}
            onSimulate={handleSimulate}
            onRemove={handleRemove}
          />
        </div>
      </div>

      {pendingApproval && (
        <PolicyApprovalModal
          policy={pendingApproval}
          onApprove={handleApprove}
          onDismiss={handleDismiss}
        />
      )}
    </div>
  );
};

// Export to window.GD
window.GD = Object.assign(window.GD || {}, { PoliciesView });
