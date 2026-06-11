// chat-panel.js  –  drop into goal-digger-camel.html
//
// Replaces the presentational right-rail Aomi agent with a live one.
// Requires the Rust server running on :8787 with chat.rs integrated.
//
// Usage: call  initChatPanel()  after DOMContentLoaded.

const CHAT_ENDPOINT = "/api/chat";

// ── state ────────────────────────────────────────────────────────────────────

let messageHistory = [];

// ── init ─────────────────────────────────────────────────────────────────────

export function initChatPanel() {
  const input  = document.getElementById("chat-input");
  const button = document.getElementById("chat-send");
  const log    = document.getElementById("chat-log");

  if (!input || !button || !log) {
    console.warn("GoalDigger chat: missing DOM elements (chat-input / chat-send / chat-log)");
    return;
  }

  button.addEventListener("click", () => sendMessage(input, log));
  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      sendMessage(input, log);
    }
  });

  // Seed with a welcome message
  appendMessage(log, "assistant",
    "GoalDigger live 🟢  Ask me which matches have the most edge against Polymarket, " +
    "or say something like: *\"How is Spain vs Germany priced?\"*"
  );
}

// ── send ─────────────────────────────────────────────────────────────────────

async function sendMessage(input, log) {
  const text = input.value.trim();
  if (!text) return;

  input.value = "";
  appendMessage(log, "user", text);
  messageHistory.push({ role: "user", content: text });

  // Show typing indicator
  const typingId = appendTyping(log);

  try {
    const body = {
      messages: messageHistory,
      // Pass current board snapshot so Claude has immediate context
      context: window.GD_LIVE ? window.__GD_BOARD__ : null,
    };

    const res = await fetch(CHAT_ENDPOINT, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });

    if (!res.ok) throw new Error(`Server error: ${res.status}`);

    const data = await res.json();

    removeTyping(log, typingId);
    appendMessage(log, "assistant", data.reply);
    messageHistory.push({ role: "assistant", content: data.reply });

    // Optional: surface tool call debug in console
    if (data.tool_calls?.length) {
      console.groupCollapsed(`[GoalDigger] ${data.tool_calls.length} tool call(s)`);
      data.tool_calls.forEach(tc => {
        console.log(`▶ ${tc.name}`, tc.input, "→", tc.result);
      });
      console.groupEnd();
    }

  } catch (err) {
    removeTyping(log, typingId);
    appendMessage(log, "error", `⚠️ ${err.message}`);
    console.error("[GoalDigger chat]", err);
  }
}

// ── DOM helpers ───────────────────────────────────────────────────────────────

function appendMessage(log, role, text) {
  const div = document.createElement("div");
  div.className = `chat-msg chat-msg--${role}`;

  // Minimal markdown: bold, italic, inline code
  div.innerHTML = text
    .replace(/\*\*(.*?)\*\*/g, "<strong>$1</strong>")
    .replace(/\*(.*?)\*/g, "<em>$1</em>")
    .replace(/`(.*?)`/g, "<code>$1</code>")
    .replace(/\n/g, "<br>");

  log.appendChild(div);
  log.scrollTop = log.scrollHeight;
  return div;
}

function appendTyping(log) {
  const id = `typing-${Date.now()}`;
  const div = document.createElement("div");
  div.id = id;
  div.className = "chat-msg chat-msg--typing";
  div.innerHTML = `<span></span><span></span><span></span>`;
  log.appendChild(div);
  log.scrollTop = log.scrollHeight;
  return id;
}

function removeTyping(log, id) {
  const el = document.getElementById(id);
  if (el) log.removeChild(el);
}

// ── board snapshot hook ───────────────────────────────────────────────────────
// Call this from data.jsx after loadLiveBoard() resolves so the chat
// panel always has fresh board context.

export function updateBoardSnapshot(board) {
  window.__GD_BOARD__ = board;
}
