import { elements, renderAll, renderResult, renderTimeline } from "./render.js";

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
  eventSource: null,
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

function appendEvent(type, data) {
  state.events.push({ type, data, at: new Date().toISOString() });
  if (type === "execution_event") {
    const event = data?.event || data;
    state.eventCursor = event?.seq ? `event/${event.seq}` : state.eventCursor;
    if (event?.event_type === "ExecutionCompleted") {
      state.result = event?.sanitized_payload?.summary || "";
      state.running = false;
      elements.tokenSummary.textContent = "Completed through service.application_execution";
    }
    if (event?.event_type === "ExecutionFailed" || event?.event_type === "ExecutionCancelled") {
      state.running = false;
    }
  }
  if (type === "final_answer") {
    state.result = data?.content || "";
    state.running = false;
    elements.tokenSummary.textContent = "Completed through debug LLM/tool loop";
  }
  if (type === "loop_failed" || type === "bridge_error") {
    state.running = false;
  }
  console.info("[codex-wasm-workbench] event", type, data);
  renderTimeline(state);
  renderResult(state);
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
  elements.tokenSummary.textContent = "Starting through service.application_execution...";
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
    openExecutionStream();
    await refreshCurrentState();
    await replayEvents();
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
  elements.tokenSummary.textContent = "Running debug-only browser loop...";
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

function openExecutionStream() {
  if (!state.sessionId) return;
  state.eventSource?.close();
  const params = new URLSearchParams({
    session_id: state.sessionId,
    trace_id: `trace-stream-${state.sessionId}`,
  });
  const since = state.eventCursor?.replace("event/", "");
  if (since) params.set("since", since);
  state.eventSource = new EventSource(`/api/apps/${applicationIdFromLocation()}/execution/events?${params}`);
  state.eventSource.addEventListener("application_execution_event", (event) => {
    try {
      appendEvent("execution_event", JSON.parse(event.data));
      refreshCurrentState();
    } catch (error) {
      appendEvent("bridge_error", { error: error instanceof Error ? error.message : String(error) });
    }
  });
  state.eventSource.onerror = () => appendEvent("bridge_error", { error: "execution event stream disconnected" });
}

async function replayEvents() {
  if (!state.sessionId) return;
  const params = new URLSearchParams({
    session_id: state.sessionId,
    trace_id: `trace-replay-${state.sessionId}`,
    page_size: "100",
  });
  if (state.runId) params.set("run_id", state.runId);
  const replay = await getJson(`/api/apps/${applicationIdFromLocation()}/execution/replay?${params}`);
  state.currentState = replay.current_state || state.currentState;
  state.eventCursor = replay.next_cursor || state.eventCursor;
  state.events = (replay.events || []).map((event) => ({
    type: "execution_event",
    data: event,
    at: event.timestamp || new Date().toISOString(),
  }));
  renderTimeline(state);
  renderResult(state);
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

function clearRun() {
  state.events = [];
  state.result = "";
  state.currentState = null;
  elements.tokenSummary.textContent = "No run yet";
  renderTimeline(state);
  renderResult(state);
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
  openExecutionStream();
  refreshCurrentState()
    .then(replayEvents)
    .catch((error) => {
      appendEvent("bridge_error", { error: error instanceof Error ? error.message : String(error) });
    });
}
