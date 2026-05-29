import { WorkbenchToolLoopController } from "./loop/controller.js";
import { WorkbenchLlmClient } from "./loop/llm_client.js";
import { elements, renderAll, renderResult, renderTimeline } from "./render.js";

// This entrypoint wires DOM events, the iframe bridge, and the app-owned tool
// loop together.  Business behavior stays in the application package while all
// side effects continue to cross Macaca's declared service boundaries.
const declaredServices = ["service.interaction", "service.app_protocol", "service.file", "service.process", "service.sandbox", "service.approval", "service.hook", "service.config", "service.code_intelligence", "service.git", "service.review", "service.diagnostics", "service.llm", "service.tool", "service.mcp", "service.skill"];

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

const hostOrigin = (() => {
  try {
    return document.referrer ? new URL(document.referrer).origin : "*";
  } catch {
    return "*";
  }
})();

const pendingBridgeCalls = new Map();

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

function appendEvent(type, data) {
  state.events.push({ type, data, at: new Date().toISOString() });
  if (type === "final_answer") {
    state.result = data?.content || "";
    state.running = false;
    elements.tokenSummary.textContent = "Completed through LLM/tool loop";
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
  state.running = true;
  state.commandId = crypto.randomUUID();
  state.sessionId ||= `workbench-${state.commandId}`;
  state.result = "";
  state.events = [];
  elements.tokenSummary.textContent = "Running LLM/tool loop...";
  renderTimeline(state);
  renderResult(state);
  try {
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
  elements.tokenSummary.textContent = "No run yet";
  renderTimeline(state);
  renderResult(state);
}

function applicationIdFromLocation() {
  return window.location.pathname.match(/\/api\/apps\/([^/]+)\/ui\//)?.[1] ?? "00000000-0000-0000-0000-000000000000";
}

elements.taskTemplate.addEventListener("change", () => {
  if (elements.taskTemplate.value) elements.taskInput.value = elements.taskTemplate.value;
});
elements.providerSelect.addEventListener("change", resolveSelectedRoute);
elements.modelSelect.addEventListener("change", resolveSelectedRoute);
elements.submitTaskButton.addEventListener("click", startTask);
elements.clearButton.addEventListener("click", clearRun);
window.addEventListener("message", handleHostMessage);

renderAll({ state, declaredServices });
loadModelCatalog();
