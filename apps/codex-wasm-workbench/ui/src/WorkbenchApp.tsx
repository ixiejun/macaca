import { useEffect, useMemo, useReducer, useRef, useState } from 'react';
import { declaredServices, taskTemplates } from './constants';
import { callAppExecution, callService, callSessionRead, createBridgeRuntime, resolveBridgeMessage } from './bridge';
import { MarkdownView } from './markdown';
import { createInitialState, normalizeExecutionEvent, workbenchReducer } from './state';
import { Timeline } from './timeline';
import type { ModelInfo, ModelRoute, ProviderInfo, TimelineEntry, WorkbenchState } from './types';
import { WorkbenchSessionMementoStore } from '../session_memento.js';

export function WorkbenchApp() {
  const [state, dispatch] = useReducer(workbenchReducer, undefined, createInitialState);
  const [task, setTask] = useState('');
  const [selectedProvider, setSelectedProvider] = useState('');
  const [selectedModel, setSelectedModel] = useState('');
  const stateRef = useRef<WorkbenchState>(state);
  const mementosRef = useRef(new WorkbenchSessionMementoStore());
  const bridgeRef = useRef(createBridgeRuntime(stateRef));
  const socketRef = useRef<WebSocket | null>(null);
  const socketClosingRef = useRef(false);

  useEffect(() => {
    stateRef.current = state;
    mementosRef.current.save(state);
  }, [state]);

  useEffect(() => {
    if (!state.sessionId) {
      const snapshot = mementosRef.current.restore(null);
      applyPatch({
        sessionId: snapshot.sessionId,
        runId: snapshot.runId,
        eventCursor: snapshot.eventCursor,
        currentState: snapshot.currentState as WorkbenchState['currentState'],
        events: snapshot.events as TimelineEntry[],
        result: snapshot.result,
        running: snapshot.running,
        tokenSummary: snapshot.tokenSummary || 'No run yet',
      });
    }
    void loadModelCatalog();
    const onMessage = (event: MessageEvent) => handleHostMessage(event);
    window.addEventListener('message', onMessage);
    return () => {
      window.removeEventListener('message', onMessage);
      closeExecutionSocket(false);
      for (const pending of bridgeRef.current.pending.values()) window.clearTimeout(pending.timeout);
      bridgeRef.current.pending.clear();
    };
    // The bridge listener is intentionally registered once. It reads mutable
    // refs so it can observe the latest state without recreating host handlers.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (!state.debugToolLoop && state.sessionId) {
      void replayEvents().then(refreshCurrentState).then(openExecutionWebSocket).catch((error) => appendEvent('bridge_error', { error: errorMessage(error) }));
    }
    // This effect should run only for the first restored session. Later session
    // changes are handled by `switchSession`, which saves the previous Memento.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [Boolean(state.sessionId)]);

  const providerModels = useMemo(
    () => state.models.filter((model) => model.provider_id === selectedProvider && model.available !== false),
    [selectedProvider, state.models],
  );
  const routeSummary = state.route
    ? `${state.route.source || 'request'} route: ${state.route.provider_id}:${state.route.model}`
    : 'Route pending';

  function applyPatch(patch: Partial<WorkbenchState>) {
    stateRef.current = { ...stateRef.current, ...patch };
    dispatch({ type: 'merge', patch });
  }

  function appendEvent(type: string, data: TimelineEntry['data']) {
    const event = { type, data, at: new Date().toISOString() };
    console.info('[codex-wasm-workbench] event', type, data);
    dispatch({ type: 'appendEvent', event });
  }

  function selectedModelReference() {
    return selectedProvider && selectedModel ? `${selectedProvider}:${selectedModel}` : null;
  }

  async function loadModelCatalog() {
    try {
      const [providerResponse, modelResponse] = await Promise.all([
        callService(bridgeRef.current, 'service.llm', 'model.provider.capabilities.read', { include_disabled: true }),
        callService(bridgeRef.current, 'service.llm', 'model.list', { include_disabled: true }),
      ]);
      const providers = ((providerResponse.result?.output as { providers?: ProviderInfo[] })?.providers || []) as ProviderInfo[];
      const models = ((modelResponse.result?.output as { models?: ModelInfo[] })?.models || []) as ModelInfo[];
      const providerId = selectedProvider || providers.find((provider) => provider.healthy)?.provider_id || providers[0]?.provider_id || '';
      const modelId = selectedModel || models.find((model) => model.provider_id === providerId && model.available !== false)?.model || '';
      applyPatch({ providers, models });
      setSelectedProvider(providerId);
      setSelectedModel(modelId);
      appendEvent('model_catalog', { providers: providers.length, models: models.length });
      if (providerId && modelId) await resolveSelectedRoute(providerId, modelId);
    } catch (error) {
      appendEvent('bridge_error', { error: errorMessage(error) });
    }
  }

  async function resolveSelectedRoute(providerId = selectedProvider, modelId = selectedModel) {
    if (!providerId || !modelId) return;
    try {
      const output = await callService(bridgeRef.current, 'service.llm', 'model.route.resolve', { request_model: `${providerId}:${modelId}` });
      const selected = (output.result?.output as { selected?: ModelRoute })?.selected || null;
      applyPatch({ route: selected });
      appendEvent('model_route', { selected, diagnostics: (output.result?.output as { diagnostics?: unknown[] })?.diagnostics || [] });
    } catch (error) {
      appendEvent('bridge_error', { error: errorMessage(error) });
    }
  }

  async function startTask() {
    const prompt = task.trim();
    if (!prompt) {
      appendEvent('bridge_error', { error: 'Task prompt is required.' });
      return;
    }
    if (stateRef.current.debugToolLoop) {
      await startDebugToolLoop(prompt);
      return;
    }
    const commandId = beginNewExecutionSession();
    applyPatch({ running: true, eventCursor: null, currentState: null, result: '', events: [], tokenSummary: 'Starting through service.application_execution...' });
    closeExecutionSocket(false);
    openExecutionWebSocket();
    try {
      const result = await callAppExecution(bridgeRef.current, 'start', {
        session_id: stateRef.current.sessionId,
        run_id: stateRef.current.runId,
        task_input: {
          summary: prompt,
          data: { requested_model: selectedModelReference(), route_hint: stateRef.current.route },
          payload_ref: null,
          truncated: false,
        },
        workspace_ref: `workspace://session/${stateRef.current.sessionId}`,
        requested_capabilities: ['capability.application_execution'],
        provider_preference: null,
        policy_context: { 'application_execution.profile': 'workbench' },
        tenant_id: null,
        actor: 'app-owned-ui',
        idempotency_key: commandId,
        trace_id: `trace-${commandId}`,
      });
      applyPatch({
        sessionId: String(result.session_id || stateRef.current.sessionId),
        runId: String(result.run_id || stateRef.current.runId),
        eventCursor: typeof result.event_cursor === 'string' ? result.event_cursor : stateRef.current.eventCursor,
      });
      appendEvent('execution_start_result', result);
      await replayEvents();
      await refreshCurrentState();
      if (!socketRef.current && stateRef.current.running) openExecutionWebSocket();
    } catch (error) {
      appendEvent('bridge_error', { error: errorMessage(error) });
    }
  }

  async function startDebugToolLoop(prompt: string) {
    beginNewDebugSession();
    applyPatch({ running: true, result: '', events: [], tokenSummary: 'Running debug-only browser loop...' });
    try {
      const [{ WorkbenchToolLoopController }, { WorkbenchLlmClient }] = await Promise.all([
        import('../loop/controller.js'),
        import('../loop/llm_client.js'),
      ]);
      const llmClient = new WorkbenchLlmClient({
        callService: (serviceId: string, operation: string, payload: Record<string, unknown>) => callService(bridgeRef.current, serviceId, operation, payload),
        applicationIdProvider: applicationIdFromLocation,
        sessionIdProvider: () => stateRef.current.sessionId,
      });
      const toolLoop = new WorkbenchToolLoopController({
        llmClient,
        callService: (serviceId: string, operation: string, payload: Record<string, unknown>) => callService(bridgeRef.current, serviceId, operation, payload),
        declaredServices,
        eventSink: appendEvent,
      });
      const result = await toolLoop.run({ task: prompt, model: selectedModelReference(), route: stateRef.current.route, availableServices: new Set(declaredServices) });
      if (result.status !== 'complete') appendEvent('loop_failed', result);
    } catch (error) {
      appendEvent('bridge_error', { error: errorMessage(error) });
    }
  }

  function beginNewExecutionSession() {
    const commandId = crypto.randomUUID();
    if (stateRef.current.sessionId) mementosRef.current.save(stateRef.current);
    applyPatch({ commandId, sessionId: `workbench-${commandId}`, runId: `run-${commandId}` });
    console.info('[codex-wasm-workbench] new execution session allocated', { command_id: commandId });
    return commandId;
  }

  function beginNewDebugSession() {
    const commandId = crypto.randomUUID();
    if (stateRef.current.sessionId) mementosRef.current.save(stateRef.current);
    applyPatch({ commandId, sessionId: `workbench-debug-${commandId}`, runId: null });
    console.info('[codex-wasm-workbench] new debug session allocated', { command_id: commandId });
  }

  function openExecutionWebSocket() {
    if (!stateRef.current.sessionId) return;
    closeExecutionSocket(false);
    const params = new URLSearchParams({ session_id: stateRef.current.sessionId, trace_id: `trace-websocket-${stateRef.current.sessionId}` });
    const since = stateRef.current.eventCursor?.replace('event/', '');
    if (since) params.set('since', since);
    const url = new URL(`/api/apps/${applicationIdFromLocation()}/execution/events/ws`, window.location.origin);
    url.protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    url.search = params.toString();
    const socket = new WebSocket(url);
    socketRef.current = socket;
    socket.addEventListener('open', () => appendEvent('stream_connected', { session_id: stateRef.current.sessionId, since: since || null }));
    socket.addEventListener('message', (event) => {
      try {
        appendEvent('execution_event', normalizeExecutionEvent(JSON.parse(event.data)));
        void refreshCurrentState();
      } catch (error) {
        appendEvent('bridge_error', { error: errorMessage(error) });
      }
    });
    socket.addEventListener('close', () => {
      if (socketRef.current === socket) socketRef.current = null;
      if (!socketClosingRef.current && stateRef.current.running) appendEvent('bridge_error', { error: 'execution event websocket disconnected' });
      socketClosingRef.current = false;
    });
    socket.addEventListener('error', () => {
      if (stateRef.current.running) appendEvent('bridge_error', { error: 'execution event websocket failed' });
    });
  }

  function closeExecutionSocket(reportDisconnect = false) {
    if (!socketRef.current) return;
    socketClosingRef.current = !reportDisconnect;
    socketRef.current.close();
  }

  async function replayEvents() {
    if (!stateRef.current.sessionId) return;
    const replay = await callAppExecution(bridgeRef.current, 'replay', {
      session_id: stateRef.current.sessionId,
      trace_id: `trace-replay-${stateRef.current.sessionId}`,
      page_size: 1000,
      run_id: stateRef.current.runId || '',
    });
    const replayed = Array.isArray(replay.events) ? replay.events.map((event) => ({ type: 'execution_event', data: normalizeExecutionEvent(event), at: new Date().toISOString() })) : [];
    if (replayed.length > 0) {
      // Replayed backend events must update the reducer and the mutable ref in
      // the same turn.  The ref is the source used by bridge calls, websocket
      // reconnects, and the app-owned Memento store, so leaving it stale would
      // make a refreshed Workbench persist the previous session snapshot.
      applyPatch({
        events: replayed,
        currentState: (replay.current_state as WorkbenchState['currentState']) || stateRef.current.currentState,
      });
    } else {
      await loadGenericSessionHistoryEvents();
    }
  }

  async function loadGenericSessionHistoryEvents() {
    if (!stateRef.current.sessionId) return;
    const eventsOutput = await callSessionRead(bridgeRef.current, 'events', { session_id: stateRef.current.sessionId, limit: 1000 });
    const sessionEvents = Array.isArray(eventsOutput.events) ? eventsOutput.events.map((event) => ({ type: 'session_event', data: event as Record<string, unknown>, at: new Date().toISOString() })) : [];
    // Generic session events are a fallback for sessions that exist before the
    // application-execution replay API has materialized records.  They must be
    // cached in the same app-owned state path as normal execution events so the
    // Workbench can rehydrate after browser refresh without asking Macaca OS to
    // know application UI details.
    applyPatch({ events: sessionEvents });
  }

  async function refreshCurrentState() {
    if (!stateRef.current.sessionId || !stateRef.current.runId) return;
    const currentState = await callAppExecution(bridgeRef.current, 'current-state', {
      session_id: stateRef.current.sessionId,
      run_id: stateRef.current.runId,
      actor: 'app-owned-ui',
      trace_id: `trace-current-${stateRef.current.sessionId}`,
    });
    applyPatch({
      currentState: currentState as WorkbenchState['currentState'],
      running: !['Completed', 'Failed', 'Cancelled'].includes(String(currentState.lifecycle_state || '')),
    });
  }

  async function sendControl(command: string, reasonCode: string) {
    if (!stateRef.current.sessionId || !stateRef.current.runId) return;
    const controlId = crypto.randomUUID();
    try {
      const result = await callAppExecution(bridgeRef.current, 'control', {
        scope: { application_id: applicationIdFromLocation(), session_id: stateRef.current.sessionId, run_id: stateRef.current.runId, tenant_id: null, actor: 'app-owned-ui' },
        command,
        control_id: controlId,
        reason_code: reasonCode,
        trace: traceContext(`trace-control-${controlId}`, stateRef.current),
        policy_context: {},
        payload: null,
        idempotency_key: controlId,
      });
      appendEvent('control_result', result);
      await refreshCurrentState();
    } catch (error) {
      appendEvent('bridge_error', { error: errorMessage(error) });
    }
  }

  function handleHostMessage(event: MessageEvent) {
    if (event.source !== window.parent) return;
    if (bridgeRef.current.hostOrigin !== '*' && event.origin !== bridgeRef.current.hostOrigin) return;
    const message = event.data as Record<string, unknown>;
    if (message.type === 'macaca.session.changed') {
      void switchSession(typeof message.session_id === 'string' ? message.session_id : null, typeof message.trace_id === 'string' ? message.trace_id : null);
      return;
    }
    resolveBridgeMessage(bridgeRef.current, message);
  }

  async function switchSession(nextSessionId: string | null, traceId: string | null) {
    const normalizedSessionId = nextSessionId?.trim() || null;
    if (normalizedSessionId === stateRef.current.sessionId) return;
    console.info('[codex-wasm-workbench] host session context received', { previousSessionId: stateRef.current.sessionId, nextSessionId: normalizedSessionId, traceId });
    mementosRef.current.save(stateRef.current);
    closeExecutionSocket(false);
    const snapshot = mementosRef.current.restore(normalizedSessionId);
    applyPatch({
      sessionId: snapshot.sessionId,
      runId: snapshot.runId,
      eventCursor: snapshot.eventCursor,
      currentState: snapshot.currentState as WorkbenchState['currentState'],
      events: snapshot.events as TimelineEntry[],
      result: snapshot.result,
      running: snapshot.running,
      tokenSummary: snapshot.tokenSummary || 'No run yet',
    });
    if (!stateRef.current.debugToolLoop && snapshot.sessionId) {
      try {
        await replayEvents();
        await refreshCurrentState();
        openExecutionWebSocket();
      } catch (error) {
        appendEvent('bridge_error', { error: errorMessage(error) });
      }
    }
  }

  function clearRun() {
    closeExecutionSocket(false);
    applyPatch({ events: [], result: '', currentState: null, tokenSummary: 'No run yet' });
  }

  const hasScope = Boolean(state.sessionId && state.runId);
  const hasApproval = (state.currentState?.pending_approval_refs || []).length > 0;
  return (
    <main className="workspace-shell">
      <section className="topbar" aria-label="Workbench status">
        <div><p className="eyebrow">Macaca WASM Application</p><h1>Codex Workbench</h1><p className="subtitle">Application-owned coding surface backed by Macaca OS services.</p></div>
        <div className="run-status" aria-live="polite"><span>{state.running ? 'Running' : 'Ready'}</span><span>{state.sessionId ? `Session ${state.sessionId.slice(0, 8)}` : 'No session'}</span></div>
      </section>
      <section className="composer-panel" aria-label="Coding task composer">
        <div className="composer-header">
          <div><p className="eyebrow">Task</p><h2>Send a real programming task</h2></div>
          <select id="taskTemplate" aria-label="Task template" onChange={(event) => setTask(event.target.value)}>{taskTemplates.map((template) => <option key={template.label} value={template.value}>{template.label}</option>)}</select>
        </div>
        <textarea id="taskInput" rows={5} value={task} onChange={(event) => setTask(event.target.value)} placeholder="Describe the coding task for CODEX-WASM-WORKBENCH..." />
        <div className="composer-actions">
          <div className="model-selector" aria-label="Model route selector">
            <label>Provider<select id="providerSelect" value={selectedProvider} onChange={(event) => { setSelectedProvider(event.target.value); void resolveSelectedRoute(event.target.value, selectedModel); }}>{state.providers.map((provider) => <option key={provider.provider_id} disabled={!provider.healthy} value={provider.provider_id}>{provider.healthy ? provider.provider_id : `${provider.provider_id} unavailable`}</option>)}</select></label>
            <label>Model<select id="modelSelect" value={selectedModel} onChange={(event) => { setSelectedModel(event.target.value); void resolveSelectedRoute(selectedProvider, event.target.value); }}>{providerModels.map((model) => <option key={model.model} value={model.model}>{model.display_name || model.model}</option>)}</select></label>
            <span id="routeSummary">{routeSummary}</span>
          </div>
          <button type="button" disabled={state.running} onClick={startTask}>{state.running ? 'Running...' : 'Run task'}</button>
          <button type="button" disabled={!hasScope || !hasApproval} onClick={() => void sendControl('Approve', 'ui_approved')}>Approve</button>
          <button type="button" disabled={!hasScope || !hasApproval} onClick={() => void sendControl('Reject', 'ui_rejected')}>Reject</button>
          <button type="button" disabled={!hasScope || !state.running} onClick={() => void sendControl('Cancel', 'ui_cancelled')}>Cancel</button>
        </div>
      </section>
      <section className="service-strip" aria-label="Declared services">{declaredServices.map((service) => <span className="status-pill" key={service}>{service}</span>)}</section>
      <section className="workspace-grid" aria-label="Coding workbench">
        <section className="editor-panel">
          <div className="panel-header"><h2>Execution Stream</h2><button type="button" onClick={clearRun}>Clear</button></div>
          <Timeline events={state.events} />
        </section>
        <aside className="side-panel"><Thread state={state} /><Diagnostics state={state} /></aside>
      </section>
      <section className="result-panel" aria-label="Execution result">
        <div className="panel-header"><h2>Result</h2><span>{state.tokenSummary}</span></div>
        <div id="resultOutput"><MarkdownView markdown={state.result || 'No assistant result yet.'} /></div>
      </section>
    </main>
  );
}

function Thread({ state }: { state: WorkbenchState }) {
  const items = [state.sessionId ? `Session ${state.sessionId}` : 'No execution session yet.', state.runId ? `Run ${state.runId}` : 'No execution run yet.', state.currentState?.lifecycle_state ? `State ${state.currentState.lifecycle_state}` : 'State unavailable.', state.running ? 'Task is running through service.application_execution.' : 'Ready for a real coding task.'];
  return <section className="thread-panel"><h2>Thread</h2><ol>{items.map((item) => <li key={item}>{item}</li>)}</ol></section>;
}

function Diagnostics({ state }: { state: WorkbenchState }) {
  const errors = state.events.filter((entry) => entry.type === 'error' || entry.type === 'bridge_error').length;
  const toolResults = state.events.filter((entry) => entry.type === 'tool_result').length;
  const diagnostics = [`Bridge: ${window.parent === window ? 'unhosted' : 'hosted iframe'}`, `Tool results: ${toolResults}`, `Errors: ${errors}`, state.route ? `Route: ${state.route.provider_id}:${state.route.model}` : 'Route: unresolved', state.eventCursor ? `Replay cursor: ${state.eventCursor}` : 'Replay cursor: none', state.debugToolLoop ? 'Debug loop: enabled' : 'Debug loop: disabled'];
  return <section className="diagnostics-panel"><h2>Diagnostics</h2><ul>{diagnostics.map((item) => <li key={item}>{item}</li>)}</ul></section>;
}

function applicationIdFromLocation() {
  return window.location.pathname.match(/\/api\/apps\/([^/]+)\/ui\//)?.[1] ?? '00000000-0000-0000-0000-000000000000';
}

function traceContext(traceId: string, state: WorkbenchState) {
  return { trace_id: traceId, session_id: state.sessionId, task_id: state.runId, agent: 'codex-wasm-workbench-ui', emitted_at: new Date().toISOString() };
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
