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
    ...state.events.slice(-80).map((entry) => {
      const item = document.createElement("article");
      item.className = `event-card event-${entry.type.replaceAll("_", "-")}`;
      const title = document.createElement("h3");
      title.textContent = entry.type;
      const body = document.createElement("pre");
      body.textContent = typeof entry.data === "string" ? entry.data : JSON.stringify(entry.data, null, 2);
      item.append(title, body);
      return item;
    }),
  );
  elements.eventTimeline.scrollTop = elements.eventTimeline.scrollHeight;
}

export function renderResult(state) {
  elements.resultOutput.textContent = state.result || "No assistant result yet.";
  elements.sessionBadge.textContent = state.sessionId ? `Session ${state.sessionId.slice(0, 8)}` : "No session";
  elements.runState.textContent = state.running ? "Running" : "Ready";
  elements.submitTaskButton.disabled = state.running;
  elements.submitTaskButton.textContent = state.running ? "Running..." : "Run task";
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
    state.running ? "Task is running through the Workbench LLM/tool loop." : "Ready for a real coding task.",
    "The application orchestrates; Macaca services own side effects, policy, trace, and audit.",
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
    state.running ? "Execution loop is open." : "Execution loop is closed.",
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
