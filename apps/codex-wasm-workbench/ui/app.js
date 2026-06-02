import { elements, renderAll, renderResult, renderTimeline } from "./render.js";
import { WorkbenchSessionMementoStore } from "./session_memento.js";

// This entrypoint is an app-owned UI bridge over Macaca's generic application
// execution protocol.  The browser keeps only render caches and transport
// handles; authoritative execution facts are started, controlled, persisted,
// replayed, and projected by `service.application_execution`.
const declaredServices = ["service.interaction", "service.app_protocol", "service.file", "service.process", "service.sandbox", "service.approval", "service.hook", "service.config", "service.code_intelligence", "service.git", "service.review", "service.diagnostics", "service.llm", "service.tool", "service.mcp", "service.skill"];

const state = {
  running: false,
  commandId: null,
  sessionId: new URLSearchParams(window.location.search).get("session_id"),
  runId: new URLSearchParams(window.location.search).get("run_id"),
  eventCursor: new URLSearchParams(window.location.search).get("cursor"),
  currentState: null,
  events: [],
  result: "",
  providers: [],
  models: [],
  route: null,
  tokenSummary: "No run yet",
  eventSocket: null,
  eventSocketClosing: false,
  debugToolLoop: new URLSearchParams(window.location.search).get("debug_tool_loop") === "1",
};

const hostOrigin = (() => {
  try {
    return document.referrer ? new URL(document.referrer).origin : "*";
  } catch {
    return "*";
  }
})();

const pendingBridgeCalls = new Map();
const sessionMementos = new WorkbenchSessionMementoStore();

function appendEvent(type, data) {
  state.events.push({ type, data, at: new Date().toISOString() });
  if (type === "execution_event") {
    const event = normalizeExecutionEvent(data);
    state.eventCursor = event?.seq ? `event/${event.seq}` : state.eventCursor;
    if (event?.event_type === "ExecutionCompleted") {
      state.result = event?.sanitized_payload?.summary || "";
      state.running = false;
      setTokenSummary("Completed through service.application_execution");
    }
    if (event?.event_type === "ExecutionFailed" || event?.event_type === "ExecutionCancelled") {
      state.running = false;
    }
  }
  if (type === "final_answer") {
    state.result = data?.content || "";
    state.running = false;
    setTokenSummary("Completed through debug LLM/tool loop");
  }
  if (type === "loop_failed" || type === "bridge_error") {
    state.running = false;
  }
  console.info("[codex-wasm-workbench] event", type, data);
  sessionMementos.save(state);
  renderTimeline(state);
  renderResult(state);
}

function appendExecutionEvent(rawEvent) {
  const event = normalizeExecutionEvent(rawEvent);
  appendEvent("execution_event", event);
}

function normalizeExecutionEvent(rawEvent) {
  // WebSocket messages carry the same durable EventLog row wrapper as the
  // legacy SSE adapter, while replay returns protocol envelopes directly.  The
  // UI keeps this Adapter logic local so render code receives one normalized
  // shape without becoming the owner of protocol projection semantics.
  const event = rawEvent?.payload || rawEvent?.event || rawEvent;
  if (rawEvent?.event_ref && event && typeof event === "object") {
    return { ...event, event_ref: rawEvent.event_ref, seq: event.seq || rawEvent.seq };
  }
  if (rawEvent?.seq && event && typeof event === "object" && !event.seq) {
    return { ...event, seq: rawEvent.seq };
  }
  return event;
}

function callService(serviceId, operation, payload = {}) {
  const commandId = crypto.randomUUID();
  return new Promise((resolve, reject) => {
    const timeout = window.setTimeout(() => {
      pendingBridgeCalls.delete(commandId);
      reject(new Error(`${operation} timed out`));
    }, 120000);
    pendingBridgeCalls.set(commandId, { resolve, reject, timeout });
    window.parent.postMessage(
      {
        type: "macaca.call",
        command_id: commandId,
        session_id: state.sessionId,
        capability: "service.call",
        service_id: serviceId,
        operation,
        payload,
      },
      hostOrigin,
    );
  });
}

function bridgeOutput(response) {
  return response?.result?.output ?? response?.output ?? null;
}

async function loadModelCatalog() {
  try {
    const [providerResponse, modelResponse] = await Promise.all([
      callService("service.llm", "model.provider.capabilities.read", { include_disabled: true }),
      callService("service.llm", "model.list", { include_disabled: true }),
    ]);
    state.providers = bridgeOutput(providerResponse)?.providers || [];
    state.models = bridgeOutput(modelResponse)?.models || [];
    appendEvent("model_catalog", { providers: state.providers.length, models: state.models.length });
    await resolveSelectedRoute();
  } catch (error) {
    appendEvent("bridge_error", { error: error instanceof Error ? error.message : String(error) });
  }
}

function selectedModelReference() {
  const provider = elements.providerSelect.value.trim();
  const model = elements.modelSelect.value.trim();
  return provider && model ? `${provider}:${model}` : null;
}

async function resolveSelectedRoute() {
  const requestModel = selectedModelReference();
  if (!requestModel) return;
  try {
    const output = bridgeOutput(await callService("service.llm", "model.route.resolve", { request_model: requestModel }));
    state.route = output?.selected || null;
    appendEvent("model_route", { selected: state.route, diagnostics: output?.diagnostics || [] });
  } catch (error) {
    appendEvent("bridge_error", { error: error instanceof Error ? error.message : String(error) });
  }
}

async function startTask() {
  const prompt = elements.taskInput.value.trim();
  if (!prompt) {
    appendEvent("bridge_error", { error: "Task prompt is required." });
    return;
  }
  if (state.debugToolLoop) {
    await startDebugToolLoop(prompt);
    return;
  }
  state.running = true;
  state.commandId = crypto.randomUUID();
  state.sessionId ||= `workbench-${state.commandId}`;
  state.runId = `run-${state.commandId}`;
  state.eventCursor = null;
  state.currentState = null;
  state.result = "";
  state.events = [];
  closeExecutionSocket(false);
  setTokenSummary("Starting through service.application_execution...");
  sessionMementos.save(state);
  renderTimeline(state);
  renderResult(state);
  try {
    const result = await postJson(`/api/apps/${applicationIdFromLocation()}/execution/start`, {
      session_id: state.sessionId,
      run_id: state.runId,
      task_input: {
        summary: prompt,
        data: {
          requested_model: selectedModelReference(),
          route_hint: state.route,
        },
        payload_ref: null,
        truncated: false,
      },
      workspace_ref: `workspace://session/${state.sessionId}`,
      requested_capabilities: ["capability.application_execution"],
      provider_preference: null,
      policy_context: {
        "application_execution.profile": "workbench",
      },
      tenant_id: null,
      actor: "app-owned-ui",
      idempotency_key: state.commandId,
      trace_id: `trace-${state.commandId}`,
    });
    state.sessionId = result.session_id || state.sessionId;
    state.runId = result.run_id || state.runId;
    state.eventCursor = result.event_cursor || state.eventCursor;
    appendEvent("execution_start_result", result);
    await replayEvents();
    await refreshCurrentState();
    openExecutionWebSocket();
  } catch (error) {
    appendEvent("bridge_error", { error: error instanceof Error ? error.message : String(error) });
  }
}

async function startDebugToolLoop(prompt) {
  state.running = true;
  state.commandId = crypto.randomUUID();
  state.sessionId ||= `workbench-debug-${state.commandId}`;
  state.result = "";
  state.events = [];
  setTokenSummary("Running debug-only browser loop...");
  sessionMementos.save(state);
  renderTimeline(state);
  renderResult(state);
  try {
    const [{ WorkbenchToolLoopController }, { WorkbenchLlmClient }] = await Promise.all([
      import("./loop/controller.js"),
      import("./loop/llm_client.js"),
    ]);
    const llmClient = new WorkbenchLlmClient({
      callService,
      applicationIdProvider: applicationIdFromLocation,
      sessionIdProvider: () => state.sessionId,
    });
    const toolLoop = new WorkbenchToolLoopController({
      llmClient,
      callService,
      declaredServices,
      eventSink: appendEvent,
    });
    const result = await toolLoop.run({
      task: prompt,
      model: selectedModelReference(),
      route: state.route,
      availableServices: new Set(declaredServices),
    });
    if (result.status !== "complete") appendEvent("loop_failed", result);
  } catch (error) {
    appendEvent("bridge_error", { error: error instanceof Error ? error.message : String(error) });
  }
}

async function postJson(path, body) {
  const response = await fetch(path, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!response.ok) throw new Error(await response.text());
  return response.json();
}

async function getJson(path) {
  const response = await fetch(path);
  if (!response.ok) throw new Error(await response.text());
  return response.json();
}

function openExecutionWebSocket() {
  if (!state.sessionId) return;
  closeExecutionSocket(false);
  const params = new URLSearchParams({
    session_id: state.sessionId,
    trace_id: `trace-websocket-${state.sessionId}`,
  });
  const since = state.eventCursor?.replace("event/", "");
  if (since) params.set("since", since);
  const url = new URL(`/api/apps/${applicationIdFromLocation()}/execution/events/ws`, window.location.origin);
  url.protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  url.search = params.toString();
  const socket = new WebSocket(url);
  state.eventSocket = socket;
  socket.addEventListener("open", () => {
    console.info("[codex-wasm-workbench] websocket opened", { session_id: state.sessionId, since });
  });
  socket.addEventListener("message", (event) => {
    try {
      appendExecutionEvent(JSON.parse(event.data));
      refreshCurrentState();
    } catch (error) {
      appendEvent("bridge_error", { error: error instanceof Error ? error.message : String(error) });
    }
  });
  socket.addEventListener("close", () => {
    if (state.eventSocket === socket) state.eventSocket = null;
    if (!state.eventSocketClosing && state.running) {
      appendEvent("bridge_error", { error: "execution event websocket disconnected" });
    }
    state.eventSocketClosing = false;
  });
  socket.addEventListener("error", () => {
    if (state.running) appendEvent("bridge_error", { error: "execution event websocket failed" });
  });
}

function closeExecutionSocket(reportDisconnect = false) {
  if (!state.eventSocket) return;
  state.eventSocketClosing = !reportDisconnect;
  state.eventSocket.close();
}

async function replayEvents({ preserveLocalOnEmpty = false } = {}) {
  if (!state.sessionId) return;
  const params = new URLSearchParams({
    session_id: state.sessionId,
    trace_id: `trace-replay-${state.sessionId}`,
    page_size: "100",
  });
  if (state.runId) params.set("run_id", state.runId);
  const replay = await getJson(`/api/apps/${applicationIdFromLocation()}/execution/replay?${params}`);
  state.currentState = replay.current_state || state.currentState;
  state.runId = state.currentState?.run_id || state.currentState?.scope?.run_id || state.runId;
  state.eventCursor = replay.next_cursor || state.eventCursor;
  const replayedEvents = (replay.events || []).map((rawEvent) => {
    const event = normalizeExecutionEvent(rawEvent);
    return {
      type: "execution_event",
      data: event,
      at: event.timestamp || new Date().toISOString(),
    };
  });
  if (replayedEvents.length > 0) {
    state.events = replayedEvents;
  } else if (!preserveLocalOnEmpty || state.events.length === 0) {
    state.events = await loadGenericSessionHistoryEvents();
    if (state.events.length === 0) {
      state.events = [sessionContextEvent("No durable session history is stored for this session yet.")];
    }
  }
  sessionMementos.save(state);
  renderTimeline(state);
  renderResult(state);
}

async function loadGenericSessionHistoryEvents() {
  // Application-execution replay is the authoritative protocol view when the
  // selected session was started through `service.application_execution`.
  // Historical Workbench sessions may instead be legacy shell sessions or
  // generic app-owned bridge sessions.  This Adapter keeps that compatibility
  // local to the application UI by reading Macaca's provider-neutral session
  // history endpoints; it does not ask the OS to understand Codex-specific
  // execution stream semantics.
  const [eventHistory, sessionDetail] = await Promise.all([
    getJson(`/api/sessions/${encodeURIComponent(state.sessionId)}/events?limit=100`).catch((error) => {
      console.warn("[codex-wasm-workbench] generic session event replay failed", {
        session_id: state.sessionId,
        error: error instanceof Error ? error.message : String(error),
      });
      return null;
    }),
    getJson(`/api/sessions/detail/${encodeURIComponent(state.sessionId)}`).catch((error) => {
      console.warn("[codex-wasm-workbench] generic session detail replay failed", {
        session_id: state.sessionId,
        error: error instanceof Error ? error.message : String(error),
      });
      return null;
    }),
  ]);

  const sessionEvents = (eventHistory?.events || []).map((event) => ({
    type: "session_event",
    data: event,
    at: event.timestamp || event.created_at || new Date().toISOString(),
  }));
  const turnEvents = (sessionDetail?.turns || []).map((turn, index) => ({
    type: "session_turn",
    data: {
      index,
      role: turn.role,
      status: turn.status || null,
      content: turn.content,
      trace_steps: turn.trace_steps || [],
      meta: turn.meta || null,
    },
    at: sessionDetail.updated_at || new Date().toISOString(),
  }));

  // EventLog rows are the most granular history.  Stored turns are still useful
  // when a legacy or bridge-projected session has no EventLog rows, so they are
  // used as a bounded fallback rather than merged into a duplicated stream.
  if (sessionEvents.length > 0) return sessionEvents;
  return turnEvents;
}

async function refreshCurrentState() {
  if (!state.sessionId || !state.runId) return;
  const params = new URLSearchParams({
    session_id: state.sessionId,
    run_id: state.runId,
    actor: "app-owned-ui",
    trace_id: `trace-current-${state.sessionId}`,
  });
  state.currentState = await getJson(`/api/apps/${applicationIdFromLocation()}/execution/current-state?${params}`);
  state.running = !["Completed", "Failed", "Cancelled"].includes(state.currentState.lifecycle_state);
  sessionMementos.save(state);
  renderResult(state);
}

async function sendControl(command, reasonCode) {
  if (!state.sessionId || !state.runId) return;
  const controlId = crypto.randomUUID();
  try {
    const result = await postJson(`/api/apps/${applicationIdFromLocation()}/execution/control`, {
      scope: {
        application_id: applicationIdFromLocation(),
        session_id: state.sessionId,
        run_id: state.runId,
        tenant_id: null,
        actor: "app-owned-ui",
      },
      command,
      control_id: controlId,
      reason_code: reasonCode,
      trace: traceContext(`trace-control-${controlId}`),
      policy_context: {},
      payload: null,
      idempotency_key: controlId,
    });
    appendEvent("control_result", result);
    await refreshCurrentState();
  } catch (error) {
    appendEvent("bridge_error", { error: error instanceof Error ? error.message : String(error) });
  }
}

function handleHostMessage(event) {
  if (event.source !== window.parent) return;
  if (hostOrigin !== "*" && event.origin !== hostOrigin) return;
  const message = event.data || {};
  if (message.type === "macaca.session.changed") {
    void switchSession(message.session_id || null, message.trace_id || null);
    return;
  }
  if (message.type !== "macaca.result") return;
  const pending = pendingBridgeCalls.get(message.command_id);
  if (!pending) return;
  window.clearTimeout(pending.timeout);
  pendingBridgeCalls.delete(message.command_id);
  if (message.ok && message.response?.accepted !== false) {
    pending.resolve(message.response);
  } else {
    pending.reject(new Error(message.error || "Bridge call failed"));
  }
}

async function switchSession(nextSessionId, traceId) {
  const normalizedSessionId = typeof nextSessionId === "string" && nextSessionId.trim() ? nextSessionId.trim() : null;
  if (normalizedSessionId === state.sessionId) return;
  console.info("[codex-wasm-workbench] host session context received", {
    previousSessionId: state.sessionId,
    nextSessionId: normalizedSessionId,
    traceId,
  });
  sessionMementos.save(state);
  closeExecutionSocket(false);
  const snapshot = sessionMementos.restore(normalizedSessionId);
  state.sessionId = snapshot.sessionId;
  state.runId = snapshot.runId;
  state.eventCursor = snapshot.eventCursor;
  state.currentState = snapshot.currentState;
  state.events = snapshot.events;
  state.result = snapshot.result;
  state.running = snapshot.running;
  setTokenSummary(snapshot.tokenSummary);
  renderTimeline(state);
  renderResult(state);
  if (!state.debugToolLoop && state.sessionId) {
    try {
      await replayEvents({ preserveLocalOnEmpty: true });
      await refreshCurrentState();
      openExecutionWebSocket();
    } catch (error) {
      appendEvent("bridge_error", { error: error instanceof Error ? error.message : String(error) });
    }
  }
}

function sessionContextEvent(message) {
  return {
    type: "session_context",
    data: {
      session_id: state.sessionId,
      run_id: state.runId,
      message,
    },
    at: new Date().toISOString(),
  };
}

function clearRun() {
  closeExecutionSocket(false);
  state.events = [];
  state.result = "";
  state.currentState = null;
  setTokenSummary("No run yet");
  sessionMementos.save(state);
  renderTimeline(state);
  renderResult(state);
}

function setTokenSummary(value) {
  state.tokenSummary = value || "No run yet";
  elements.tokenSummary.textContent = state.tokenSummary;
}

function applicationIdFromLocation() {
  return window.location.pathname.match(/\/api\/apps\/([^/]+)\/ui\//)?.[1] ?? "00000000-0000-0000-0000-000000000000";
}

function traceContext(traceId) {
  return {
    trace_id: traceId,
    session_id: state.sessionId,
    task_id: state.runId,
    agent: "codex-wasm-workbench-ui",
    emitted_at: new Date().toISOString(),
  };
}

elements.taskTemplate.addEventListener("change", () => {
  if (elements.taskTemplate.value) elements.taskInput.value = elements.taskTemplate.value;
});
elements.providerSelect.addEventListener("change", resolveSelectedRoute);
elements.modelSelect.addEventListener("change", resolveSelectedRoute);
elements.submitTaskButton.addEventListener("click", startTask);
elements.approveButton.addEventListener("click", () => sendControl("Approve", "ui_approved"));
elements.rejectButton.addEventListener("click", () => sendControl("Reject", "ui_rejected"));
elements.cancelButton.addEventListener("click", () => sendControl("Cancel", "ui_cancelled"));
elements.clearButton.addEventListener("click", clearRun);
window.addEventListener("message", handleHostMessage);

renderAll({ state, declaredServices });
loadModelCatalog();
if (!state.debugToolLoop && state.sessionId) {
  replayEvents()
    .then(refreshCurrentState)
    .then(openExecutionWebSocket)
    .catch((error) => {
      appendEvent("bridge_error", { error: error instanceof Error ? error.message : String(error) });
    });
}
