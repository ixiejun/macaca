const declaredServices = [
  "service.interaction",
  "service.app_protocol",
  "service.file",
  "service.process",
  "service.sandbox",
  "service.git",
  "service.review",
  "service.diagnostics",
];

const state = {
  running: false,
  commandId: null,
  sessionId: null,
  events: [],
  result: "",
  providers: [],
  models: [],
  route: null,
};

// The UI is hosted inside Macaca's app-owned iframe surface.  The parent origin
// is derived from the browser-provided referrer so task execution messages are
// addressed only to the shell that embedded this bundle.
const hostOrigin = (() => {
  try {
    return document.referrer ? new URL(document.referrer).origin : "*";
  } catch {
    return "*";
  }
})();

const elements = {
  statusStrip: document.querySelector("#statusStrip"),
  runState: document.querySelector("#runState"),
  sessionBadge: document.querySelector("#sessionBadge"),
  taskTemplate: document.querySelector("#taskTemplate"),
  taskInput: document.querySelector("#taskInput"),
  providerSelect: document.querySelector("#providerSelect"),
  modelSelect: document.querySelector("#modelSelect"),
  routeSummary: document.querySelector("#routeSummary"),
  submitTaskButton: document.querySelector("#submitTaskButton"),
  clearButton: document.querySelector("#clearButton"),
  eventTimeline: document.querySelector("#eventTimeline"),
  threadItems: document.querySelector("#threadItems"),
  diagnosticsList: document.querySelector("#diagnosticsList"),
  resultOutput: document.querySelector("#resultOutput"),
  tokenSummary: document.querySelector("#tokenSummary"),
};

function renderServiceStrip() {
  elements.statusStrip.replaceChildren(
    ...declaredServices.map((service) => {
      const item = document.createElement("span");
      item.className = "status-pill";
      item.textContent = service;
      return item;
    }),
  );
}

function renderThread() {
  const threadItems = [
    state.sessionId ? `Session ${state.sessionId}` : "No execution session yet.",
    state.running ? "Task is running through Macaca OS /api/chat/v2." : "Ready for a real coding task.",
    "UI owns presentation; Macaca services own execution, policy, trace, and audit.",
  ];
  elements.threadItems.replaceChildren(
    ...threadItems.map((value) => {
      const item = document.createElement("li");
      item.textContent = value;
      return item;
    }),
  );
}

function renderDiagnostics() {
  const audits = state.events.filter((entry) => entry.type === "service_call_audit").length;
  const errors = state.events.filter((entry) => entry.type === "error" || entry.type === "bridge_error").length;
  const diagnostics = [
    `Bridge: ${window.parent === window ? "unhosted" : "hosted iframe"}`,
    `Service audit events: ${audits}`,
    `Errors: ${errors}`,
    state.route ? `Route: ${state.route.provider_id}:${state.route.model}` : "Route: unresolved",
    state.running ? "Execution stream is open." : "Execution stream is closed.",
  ];
  elements.diagnosticsList.replaceChildren(
    ...diagnostics.map((value) => {
      const item = document.createElement("li");
      item.textContent = value;
      return item;
    }),
  );
}

function renderTimeline() {
  if (state.events.length === 0) {
    elements.eventTimeline.innerHTML = '<p class="empty">Submit a task to stream execution events.</p>';
    return;
  }
  elements.eventTimeline.replaceChildren(
    ...state.events.slice(-80).map((entry) => {
      const item = document.createElement("article");
      item.className = `event-card event-${entry.type.replaceAll("_", "-")}`;
      const title = document.createElement("h3");
      title.textContent = entry.type;
      const body = document.createElement("pre");
      body.textContent = compactJson(entry.data);
      item.append(title, body);
      return item;
    }),
  );
  elements.eventTimeline.scrollTop = elements.eventTimeline.scrollHeight;
}

function renderResult() {
  elements.resultOutput.textContent = state.result || "No assistant result yet.";
  elements.sessionBadge.textContent = state.sessionId ? `Session ${state.sessionId.slice(0, 8)}` : "No session";
  elements.runState.textContent = state.running ? "Running" : "Ready";
  elements.submitTaskButton.disabled = state.running;
  elements.submitTaskButton.textContent = state.running ? "Running..." : "Run task";
  renderModelSelector();
  renderThread();
  renderDiagnostics();
}

function compactJson(value) {
  if (typeof value === "string") return value;
  return JSON.stringify(value, null, 2);
}

function appendEvent(type, data) {
  state.events.push({ type, data, at: new Date().toISOString() });
  if (type === "session_id" && data?.session_id) state.sessionId = data.session_id;
  if (type === "content" || type === "assistant") {
    state.result += data?.content || "";
  }
  if (type === "done") {
    state.running = false;
    if (data?.llm_route) state.route = data.llm_route;
    const tokens = data?.tokens;
    elements.tokenSummary.textContent = tokens
      ? `${tokens.total} tokens / ${data.iterations ?? 0} iterations`
      : "Completed";
  }
  if (type === "error") state.running = false;
  console.info("[codex-wasm-workbench] event", type, data);
  renderTimeline();
  renderResult();
}

function callService(serviceId, operation, payload = {}) {
  const commandId = crypto.randomUUID();
  return new Promise((resolve, reject) => {
    const timeout = window.setTimeout(() => {
      pendingBridgeCalls.delete(commandId);
      reject(new Error(`${operation} timed out`));
    }, 15000);
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
      callService("service.llm", "model.provider.capabilities.read", {
        include_disabled: true,
      }),
      callService("service.llm", "model.list", {
        include_disabled: true,
      }),
    ]);
    state.providers = bridgeOutput(providerResponse)?.providers || [];
    state.models = bridgeOutput(modelResponse)?.models || [];
    appendEvent("model_catalog", {
      providers: state.providers.length,
      models: state.models.length,
    });
    await resolveSelectedRoute();
  } catch (error) {
    appendEvent("bridge_error", {
      error: error instanceof Error ? error.message : String(error),
    });
  }
  renderResult();
}

function renderModelSelector() {
  const currentProvider = elements.providerSelect.value;
  const currentModel = elements.modelSelect.value;
  const providers = state.providers;
  if (providers.length === 0) {
    elements.providerSelect.replaceChildren(new Option("No providers", ""));
    elements.modelSelect.replaceChildren(new Option("No models", ""));
    elements.routeSummary.textContent = "Model catalog unavailable";
    return;
  }

  elements.providerSelect.replaceChildren(
    ...providers.map((provider) => {
      const label = provider.healthy
        ? provider.provider_id
        : `${provider.provider_id} unavailable`;
      const option = new Option(label, provider.provider_id);
      option.disabled = !provider.healthy;
      return option;
    }),
  );
  if (currentProvider) elements.providerSelect.value = currentProvider;
  if (!elements.providerSelect.value) {
    const firstHealthy = providers.find((provider) => provider.healthy);
    if (firstHealthy) elements.providerSelect.value = firstHealthy.provider_id;
  }

  const providerId = elements.providerSelect.value;
  const providerModels = state.models.filter(
    (model) => model.provider_id === providerId && model.available !== false,
  );
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

function selectedModelReference() {
  const provider = elements.providerSelect.value.trim();
  const model = elements.modelSelect.value.trim();
  if (!provider || !model) return null;
  return `${provider}:${model}`;
}

async function resolveSelectedRoute() {
  const requestModel = selectedModelReference();
  if (!requestModel) return;
  try {
    const response = await callService("service.llm", "model.route.resolve", {
      request_model: requestModel,
    });
    const output = bridgeOutput(response);
    state.route = output?.selected || null;
    appendEvent("model_route", {
      selected: state.route,
      diagnostics: output?.diagnostics || [],
    });
  } catch (error) {
    appendEvent("bridge_error", {
      error: error instanceof Error ? error.message : String(error),
    });
  }
  renderResult();
}

function startTask() {
  const prompt = elements.taskInput.value.trim();
  if (!prompt) {
    appendEvent("bridge_error", { error: "Task prompt is required." });
    return;
  }
  state.running = true;
  state.commandId = crypto.randomUUID();
  state.result = "";
  state.events = [];
  elements.tokenSummary.textContent = "Streaming...";
  renderTimeline();
  renderResult();
  console.info("[codex-wasm-workbench] starting task", {
    commandId: state.commandId,
    sessionId: state.sessionId,
  });
  window.parent.postMessage(
    {
      type: "macaca.chat.start",
      command_id: state.commandId,
      session_id: state.sessionId,
      prompt,
      model: selectedModelReference(),
    },
    hostOrigin,
  );
}

const pendingBridgeCalls = new Map();

function handleHostMessage(event) {
  if (event.source !== window.parent) return;
  if (hostOrigin !== "*" && event.origin !== hostOrigin) return;
  const message = event.data || {};
  if (message.type === "macaca.result") {
    const pending = pendingBridgeCalls.get(message.command_id);
    if (!pending) return;
    window.clearTimeout(pending.timeout);
    pendingBridgeCalls.delete(message.command_id);
    if (message.ok && message.response?.accepted !== false) {
      pending.resolve(message.response);
    } else {
      pending.reject(new Error(message.error || "Bridge call failed"));
    }
    return;
  }
  if (!message.type || message.command_id !== state.commandId) return;
  if (message.type === "macaca.chat.open") {
    appendEvent("stream_open", { command_id: message.command_id });
  } else if (message.type === "macaca.chat.event") {
    appendEvent(message.event.type, message.event.data);
  } else if (message.type === "macaca.chat.error") {
    appendEvent("bridge_error", { error: message.error });
    state.running = false;
    renderResult();
  } else if (message.type === "macaca.chat.closed") {
    state.running = false;
    renderResult();
  }
}

function clearRun() {
  state.events = [];
  state.result = "";
  elements.tokenSummary.textContent = "No run yet";
  renderTimeline();
  renderResult();
}

elements.taskTemplate.addEventListener("change", () => {
  if (elements.taskTemplate.value) elements.taskInput.value = elements.taskTemplate.value;
});
elements.providerSelect.addEventListener("change", resolveSelectedRoute);
elements.modelSelect.addEventListener("change", resolveSelectedRoute);
elements.submitTaskButton.addEventListener("click", startTask);
elements.clearButton.addEventListener("click", clearRun);
window.addEventListener("message", handleHostMessage);

renderServiceStrip();
renderTimeline();
renderResult();
loadModelCatalog();
