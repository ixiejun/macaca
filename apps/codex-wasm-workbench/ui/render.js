// Rendering is intentionally isolated from execution.  The Workbench can show
// model routes, loop events, diagnostics, and final answers, but it must not
// become the owner of service semantics or tool execution policy.
export const elements = {
  statusStrip: document.querySelector("#statusStrip"),
  runState: document.querySelector("#runState"),
  sessionBadge: document.querySelector("#sessionBadge"),
  taskTemplate: document.querySelector("#taskTemplate"),
  taskInput: document.querySelector("#taskInput"),
  providerSelect: document.querySelector("#providerSelect"),
  modelSelect: document.querySelector("#modelSelect"),
  routeSummary: document.querySelector("#routeSummary"),
  submitTaskButton: document.querySelector("#submitTaskButton"),
  approveButton: document.querySelector("#approveButton"),
  rejectButton: document.querySelector("#rejectButton"),
  cancelButton: document.querySelector("#cancelButton"),
  clearButton: document.querySelector("#clearButton"),
  eventTimeline: document.querySelector("#eventTimeline"),
  threadItems: document.querySelector("#threadItems"),
  diagnosticsList: document.querySelector("#diagnosticsList"),
  resultOutput: document.querySelector("#resultOutput"),
  tokenSummary: document.querySelector("#tokenSummary"),
};

export function renderAll({ state, declaredServices }) {
  renderServiceStrip(declaredServices);
  renderTimeline(state);
  renderResult(state);
}

export function renderServiceStrip(declaredServices) {
  elements.statusStrip.replaceChildren(
    ...declaredServices.map((service) => {
      const item = document.createElement("span");
      item.className = "status-pill";
      item.textContent = service;
      return item;
    }),
  );
}

export function renderTimeline(state) {
  if (state.events.length === 0) {
    elements.eventTimeline.innerHTML = '<p class="empty">Submit a task to stream execution events.</p>';
    return;
  }
  elements.eventTimeline.replaceChildren(
    ...state.events.map((entry) => {
      const item = document.createElement("article");
      item.className = `event-card event-${entry.type.replaceAll("_", "-")}`;
      const view = presentTimelineEvent(entry);
      const title = document.createElement("h3");
      title.textContent = view.title;
      const body = renderDisplayBody(view);
      item.append(title, body);
      if (view.meta.length > 0) {
        const meta = document.createElement("ul");
        meta.className = "event-meta";
        meta.replaceChildren(...view.meta.map(listItem));
        item.append(meta);
      }
      return item;
    }),
  );
  elements.eventTimeline.scrollTop = elements.eventTimeline.scrollHeight;
}

export function presentTimelineEvent(entry) {
  // The Workbench Presenter converts provider-neutral protocol envelopes into
  // user-facing timeline rows.  It deliberately consumes only sanitized display
  // fields or bounded summaries supplied by Macaca; raw provider prompts,
  // credentials, manifests, and unbounded tool payloads stay out of the UI.
  const event = entry?.type === "execution_event" ? entry.data : null;
  if (event) return presentExecutionEvent(event);
  if (entry?.type === "stream_connected") {
    return {
      title: "Live updates connected",
      body: "This session is now following backend execution events in real time.",
      meta: entry.data?.since ? [`Resumed after event ${entry.data.since}`] : [],
      usePre: false,
    };
  }
  if (entry?.type === "execution_start_result") {
    return {
      title: "Task accepted",
      body: "Macaca accepted the task and assigned a backend execution session.",
      meta: compactMeta({
        session: entry.data?.session_id,
        run: entry.data?.run_id,
        provider: entry.data?.provider_kind,
      }),
      usePre: false,
    };
  }
  if (entry?.type === "bridge_error") {
    return {
      title: "Connection issue",
      body: entry.data?.error || "The workbench bridge reported an error.",
      meta: [],
      usePre: false,
    };
  }
  if (entry?.type === "assistant_response") {
    return {
      title: entry.data?.tool_calls?.length ? "Assistant requested tools" : "Assistant response",
      body: [entry.data?.reasoning_content, entry.data?.content].filter(Boolean).join("\n\n") || "No assistant text was emitted.",
      meta: compactMeta({
        model: entry.data?.model,
        tools: (entry.data?.tool_calls || []).map((toolCall) => toolCall.name).join(", "),
      }),
      format: "markdown",
      usePre: false,
    };
  }
  if (entry?.type === "tool_result") {
    const result = entry.data?.result || {};
    const display = result.display || {};
    return {
      title: display.title || `${entry.data?.tool || "Tool"} ${entry.data?.status || "completed"}`,
      body: display.body || result.output?.text || JSON.stringify(result, null, 2),
      meta: compactMeta({
        tool: entry.data?.tool,
        operation: result.operation,
        file: display.file_path,
        status: result.status || entry.data?.status,
      }),
      format: display.format || "markdown",
      usePre: display.format === "json",
    };
  }
  if (entry?.type === "model_catalog") {
    return {
      title: "Model catalog loaded",
      body: `${entry.data?.providers || 0} providers and ${entry.data?.models || 0} models are available.`,
      meta: [],
      usePre: false,
    };
  }
  return {
    title: humanize(entry?.type || "event"),
    body: typeof entry?.data === "string" ? entry.data : JSON.stringify(entry?.data || {}, null, 2),
    meta: [],
    usePre: typeof entry?.data !== "string",
  };
}

function presentExecutionEvent(event) {
  const payload = event.sanitized_payload || {};
  const data = payload.data || {};
  const title = data.display_title || eventTitle(event.event_type);
  const body =
    data.display_body ||
    data.display_markdown ||
    data.content ||
    data.output ||
    payload.summary ||
    "Execution event received.";
  return {
    title,
    body,
    meta: compactMeta({
      tool: data.tool_name,
      file: data.file_path,
      status: data.status,
      provider: event.provider_kind,
      seq: event.seq,
    }),
    format: data.display_format || (data.display_markdown ? "markdown" : "text"),
    usePre: data.display_format === "json",
  };
}

function eventTitle(eventType) {
  switch (eventType) {
    case "LlmRequested":
      return "Task sent to the agent";
    case "LlmCompleted":
      return "Assistant response";
    case "ToolCallRequested":
      return "Tool call requested";
    case "ToolCallCompleted":
      return "Tool call completed";
    case "ExecutionCompleted":
      return "Execution completed";
    case "ExecutionFailed":
      return "Execution failed";
    case "ProviderAssigned":
      return "Provider assigned";
    case "ProviderHeartbeat":
      return "Execution progress";
    default:
      return humanize(eventType || "execution event");
  }
}

function compactMeta(values) {
  return Object.entries(values)
    .filter(([, value]) => value !== null && value !== undefined && value !== "")
    .map(([key, value]) => `${humanize(key)}: ${value}`);
}

function humanize(value) {
  return String(value)
    .replaceAll("_", " ")
    .replaceAll("-", " ")
    .replace(/([a-z])([A-Z])/g, "$1 $2")
    .replace(/\s+/g, " ")
    .trim()
    .replace(/^./, (char) => char.toUpperCase());
}

export function renderResult(state) {
  elements.resultOutput.replaceChildren(renderMarkdownDocument(state.result || "No assistant result yet."));
  elements.sessionBadge.textContent = state.sessionId ? `Session ${state.sessionId.slice(0, 8)}` : "No session";
  elements.runState.textContent = state.running ? "Running" : "Ready";
  elements.submitTaskButton.disabled = state.running;
  elements.submitTaskButton.textContent = state.running ? "Running..." : "Run task";
  const hasScope = Boolean(state.sessionId && state.runId);
  const hasApproval = (state.currentState?.pending_approval_refs || []).length > 0;
  elements.approveButton.disabled = !hasScope || !hasApproval;
  elements.rejectButton.disabled = !hasScope || !hasApproval;
  elements.cancelButton.disabled = !hasScope || !state.running;
  renderModelSelector(state);
  renderThread(state);
  renderDiagnostics(state);
}

function renderModelSelector(state) {
  const currentProvider = elements.providerSelect.value;
  const currentModel = elements.modelSelect.value;
  if (state.providers.length === 0) {
    elements.providerSelect.replaceChildren(new Option("No providers", ""));
    elements.modelSelect.replaceChildren(new Option("No models", ""));
    elements.routeSummary.textContent = "Model catalog unavailable";
    return;
  }
  elements.providerSelect.replaceChildren(...state.providers.map(providerOption));
  if (currentProvider) elements.providerSelect.value = currentProvider;
  if (!elements.providerSelect.value) {
    elements.providerSelect.value = state.providers.find((provider) => provider.healthy)?.provider_id || "";
  }
  const providerModels = state.models.filter((model) => model.provider_id === elements.providerSelect.value && model.available !== false);
  elements.modelSelect.replaceChildren(
    ...(providerModels.length > 0
      ? providerModels.map((model) => new Option(model.display_name || model.model, model.model))
      : [new Option("No selectable models", "")]),
  );
  if (currentModel) elements.modelSelect.value = currentModel;
  elements.routeSummary.textContent = state.route
    ? `${state.route.source || "request"} route: ${state.route.provider_id}:${state.route.model}`
    : "Route pending";
}

function renderThread(state) {
  const threadItems = [
    state.sessionId ? `Session ${state.sessionId}` : "No execution session yet.",
    state.runId ? `Run ${state.runId}` : "No execution run yet.",
    state.currentState?.lifecycle_state ? `State ${state.currentState.lifecycle_state}` : "State unavailable.",
    state.running ? "Task is running through service.application_execution." : "Ready for a real coding task.",
  ];
  elements.threadItems.replaceChildren(...threadItems.map(listItem));
}

function renderDiagnostics(state) {
  const errors = state.events.filter((entry) => entry.type === "error" || entry.type === "bridge_error").length;
  const toolResults = state.events.filter((entry) => entry.type === "tool_result").length;
  const diagnostics = [
    `Bridge: ${window.parent === window ? "unhosted" : "hosted iframe"}`,
    `Tool results: ${toolResults}`,
    `Errors: ${errors}`,
    state.route ? `Route: ${state.route.provider_id}:${state.route.model}` : "Route: unresolved",
    state.eventCursor ? `Replay cursor: ${state.eventCursor}` : "Replay cursor: none",
    state.debugToolLoop ? "Debug loop: enabled" : "Debug loop: disabled",
  ];
  elements.diagnosticsList.replaceChildren(...diagnostics.map(listItem));
}

function providerOption(provider) {
  const option = new Option(provider.healthy ? provider.provider_id : `${provider.provider_id} unavailable`, provider.provider_id);
  option.disabled = !provider.healthy;
  return option;
}

function listItem(value) {
  const item = document.createElement("li");
  item.textContent = value;
  return item;
}

function renderDisplayBody(view) {
  if (view.format === "markdown") return renderMarkdownDocument(view.body);
  const body = document.createElement(view.usePre ? "pre" : "p");
  body.textContent = view.body;
  return body;
}

export function renderMarkdownDocument(markdown) {
  // The renderer is deliberately small and DOM-based.  It creates elements with
  // text nodes instead of injecting HTML, which lets the Workbench beautify
  // user-visible markdown while preserving the shell/application security
  // boundary.
  const root = document.createElement("div");
  root.className = "markdown-body";
  const lines = normalizeMarkdownLines(markdown);
  let paragraph = [];
  let list = null;
  let codeLines = null;
  let codeLanguage = "";

  const flushParagraph = () => {
    if (paragraph.length === 0) return;
    const p = document.createElement("p");
    appendInlineMarkdown(p, paragraph.join("\n"));
    root.append(p);
    paragraph = [];
  };
  const flushList = () => {
    if (!list) return;
    root.append(list);
    list = null;
  };

  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    const fence = line.match(/^```(.*)$/);
    if (fence) {
      if (codeLines) {
        const pre = document.createElement("pre");
        const code = document.createElement("code");
        if (codeLanguage) code.dataset.language = codeLanguage;
        code.textContent = codeLines.join("\n");
        pre.append(code);
        root.append(pre);
        codeLines = null;
        codeLanguage = "";
      } else {
        flushParagraph();
        flushList();
        codeLines = [];
        codeLanguage = fence[1].trim();
      }
      continue;
    }
    if (codeLines) {
      codeLines.push(line);
      continue;
    }
    if (isMarkdownRule(line)) {
      flushParagraph();
      flushList();
      const rule = document.createElement("hr");
      root.append(rule);
      continue;
    }
    if (isMarkdownTableStart(lines, index)) {
      flushParagraph();
      flushList();
      const { table, nextIndex } = renderMarkdownTable(lines, index);
      root.append(table);
      index = nextIndex - 1;
      continue;
    }
    if (!line.trim()) {
      flushParagraph();
      flushList();
      continue;
    }
    const heading = line.match(/^(#{1,3})\s+(.+)$/);
    if (heading) {
      flushParagraph();
      flushList();
      const level = String(heading[1]).length + 2;
      const h = document.createElement(`h${level}`);
      appendInlineMarkdown(h, heading[2]);
      root.append(h);
      continue;
    }
    const bullet = line.match(/^\s*[-*]\s+(.+)$/);
    if (bullet) {
      flushParagraph();
      list ||= document.createElement("ul");
      const li = document.createElement("li");
      appendInlineMarkdown(li, bullet[1]);
      list.append(li);
      continue;
    }
    const ordered = line.match(/^\s*\d+\.\s+(.+)$/);
    if (ordered) {
      flushParagraph();
      if (!list || list.tagName !== "OL") {
        flushList();
        list = document.createElement("ol");
      }
      const li = document.createElement("li");
      appendInlineMarkdown(li, ordered[1]);
      list.append(li);
      continue;
    }
    paragraph.push(line.trim());
  }
  flushParagraph();
  flushList();
  if (codeLines) {
    const pre = document.createElement("pre");
    const code = document.createElement("code");
    code.textContent = codeLines.join("\n");
    pre.append(code);
    root.append(pre);
  }
  return root;
}

function normalizeMarkdownLines(markdown) {
  // Some model providers emit several markdown constructs on one physical line,
  // for example `Done --- ## Files ``` tree`.  This Adapter normalizes only
  // presentation structure: fences, horizontal rules, headings, and compact
  // pipe tables become logical lines before the DOM renderer consumes them.
  // It does not interpret application semantics or mutate the model's visible
  // content, preserving the app boundary while making the stream readable.
  const normalized = [];
  let inFence = false;
  for (const rawLine of String(markdown || "").split(/\r?\n/)) {
    let line = rawLine;
    while (line.includes("```")) {
      const fenceIndex = line.indexOf("```");
      const before = line.slice(0, fenceIndex).trimEnd();
      if (before) normalized.push(...splitInlineMarkdownStructure(before));
      const after = line.slice(fenceIndex + 3).trimStart();
      if (inFence) {
        normalized.push("```");
        inFence = false;
        line = after;
        continue;
      }
      const languageOnly = after && /^[A-Za-z0-9_-]+$/.test(after);
      normalized.push(languageOnly ? `\`\`\`${after}` : "```");
      inFence = true;
      if (!after || languageOnly) {
        line = "";
      } else {
        line = after;
      }
    }
    if (inFence) {
      normalized.push(line);
    } else {
      normalized.push(...splitInlineMarkdownStructure(line));
    }
  }
  return normalized;
}

function splitInlineMarkdownStructure(line) {
  // Hosted LLM output often optimizes for a compact chat transcript and emits
  // `---`, headings, or simple pipe tables inline.  The renderer treats those
  // tokens as layout hints only when they have markdown-like spacing around
  // them, so ordinary prose that contains hyphens or pipes remains untouched.
  const expandedTables = expandCompactPipeTable(line);
  const parts = [];
  for (const segment of expandedTables) {
    parts.push(...splitInlineRulesAndHeadings(segment));
  }
  return parts;
}

function splitInlineRulesAndHeadings(line) {
  const text = String(line || "");
  if (!text.trim()) return [text];
  const tokens = [];
  const pattern = /(?:^|\s)(---|\#{1,3}\s+[^#]+?)(?=(?:\s---|\s#{1,3}\s+|$))/g;
  let offset = 0;
  for (const match of text.matchAll(pattern)) {
    const tokenStart = match.index + (match[0].startsWith(" ") ? 1 : 0);
    const before = text.slice(offset, tokenStart).trim();
    if (before) tokens.push(before);
    tokens.push(match[1].trim());
    offset = tokenStart + match[1].length;
  }
  const after = text.slice(offset).trim();
  if (after) tokens.push(after);
  return tokens.length > 0 ? tokens : [line];
}

function expandCompactPipeTable(line) {
  const text = String(line || "").trim();
  if (!text.includes("||") || !/\|\s*-{3,}/.test(text)) return [line];
  const rows = text
    .split(/\s*\|\|\s*/g)
    .map((row) => row.trim())
    .filter(Boolean)
    .map((row) => row.startsWith("|") ? row : `| ${row}`)
    .map((row) => row.endsWith("|") ? row : `${row} |`);
  return rows.length >= 2 ? rows : [line];
}

function isMarkdownRule(line) {
  return /^\s*(-{3,}|\*{3,}|_{3,})\s*$/.test(line);
}

function isMarkdownTableStart(lines, index) {
  return isTableRow(lines[index]) && isTableSeparator(lines[index + 1] || "");
}

function isTableRow(line) {
  return /^\s*\|.*\|\s*$/.test(line || "");
}

function isTableSeparator(line) {
  return /^\s*\|?(\s*:?-{3,}:?\s*\|)+\s*:?-{3,}:?\s*\|?\s*$/.test(line || "");
}

function renderMarkdownTable(lines, startIndex) {
  const table = document.createElement("table");
  const thead = document.createElement("thead");
  const tbody = document.createElement("tbody");
  const headerRow = document.createElement("tr");
  for (const cell of splitTableCells(lines[startIndex])) {
    const th = document.createElement("th");
    appendInlineMarkdown(th, cell);
    headerRow.append(th);
  }
  thead.append(headerRow);
  let index = startIndex + 2;
  while (index < lines.length && isTableRow(lines[index])) {
    const tr = document.createElement("tr");
    for (const cell of splitTableCells(lines[index])) {
      const td = document.createElement("td");
      appendInlineMarkdown(td, cell);
      tr.append(td);
    }
    tbody.append(tr);
    index += 1;
  }
  table.append(thead, tbody);
  return { table, nextIndex: index };
}

function splitTableCells(line) {
  return String(line || "")
    .trim()
    .replace(/^\|/, "")
    .replace(/\|$/, "")
    .split("|")
    .map((cell) => cell.trim());
}

function appendInlineMarkdown(parent, text) {
  const pattern = /(`[^`]+`|\*\*[^*]+\*\*|\[[^\]]+\]\([^)]+\))/g;
  let offset = 0;
  for (const match of String(text).matchAll(pattern)) {
    if (match.index > offset) parent.append(document.createTextNode(text.slice(offset, match.index)));
    const token = match[0];
    if (token.startsWith("`")) {
      const code = document.createElement("code");
      code.textContent = token.slice(1, -1);
      parent.append(code);
    } else if (token.startsWith("**")) {
      const strong = document.createElement("strong");
      strong.textContent = token.slice(2, -2);
      parent.append(strong);
    } else {
      const link = token.match(/^\[([^\]]+)\]\(([^)]+)\)$/);
      const anchor = document.createElement("a");
      anchor.textContent = link?.[1] || token;
      anchor.href = link?.[2] || "#";
      anchor.rel = "noreferrer";
      anchor.target = "_blank";
      parent.append(anchor);
    }
    offset = match.index + token.length;
  }
  if (offset < text.length) parent.append(document.createTextNode(text.slice(offset)));
}
