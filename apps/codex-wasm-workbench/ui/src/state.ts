import type { WorkbenchState } from './types';

export type WorkbenchAction =
  | { type: 'merge'; patch: Partial<WorkbenchState> }
  | { type: 'appendEvent'; event: WorkbenchState['events'][number] }
  | { type: 'replaceEvents'; events: WorkbenchState['events'] }
  | { type: 'clearRun' };

export function createInitialState(): WorkbenchState {
  const params = new URLSearchParams(window.location.search);
  return {
    running: false,
    commandId: null,
    sessionId: params.get('session_id'),
    runId: params.get('run_id'),
    eventCursor: params.get('cursor'),
    currentState: null,
    events: [],
    result: '',
    providers: [],
    models: [],
    route: null,
    tokenSummary: 'No run yet',
    debugToolLoop: params.get('debug_tool_loop') === '1',
  };
}

/**
 * Keep Workbench state transitions explicit and auditable.
 *
 * React owns only presentation state.  Durable execution state remains in
 * `service.application_execution`; this reducer stores the current UI snapshot
 * and replay cursor so refresh can rehydrate through the app-local Memento.
 */
export function workbenchReducer(state: WorkbenchState, action: WorkbenchAction): WorkbenchState {
  if (action.type === 'merge') return { ...state, ...action.patch };
  if (action.type === 'replaceEvents') return { ...state, events: action.events };
  if (action.type === 'clearRun') {
    return { ...state, events: [], result: '', currentState: null, tokenSummary: 'No run yet' };
  }
  const event = action.event;
  const next = { ...state, events: [...state.events, event] };
  if (event.type === 'execution_event') {
    const execution = normalizeExecutionEvent(event.data);
    next.eventCursor = execution?.seq ? `event/${execution.seq}` : next.eventCursor;
    if (execution?.event_type === 'ExecutionCompleted') {
      const sanitizedPayload = isRecord(execution.sanitized_payload) ? execution.sanitized_payload : {};
      next.result = typeof sanitizedPayload.summary === 'string' ? sanitizedPayload.summary : '';
      next.running = false;
      next.tokenSummary = 'Completed through service.application_execution';
    }
    if (execution?.event_type === 'ExecutionFailed' || execution?.event_type === 'ExecutionCancelled') next.running = false;
  }
  if (event.type === 'final_answer') {
    const data = event.data as Record<string, unknown>;
    next.result = String(data?.content || '');
    next.running = false;
    next.tokenSummary = 'Completed through debug LLM/tool loop';
  }
  if (event.type === 'loop_failed' || event.type === 'bridge_error') next.running = false;
  return next;
}

export function normalizeExecutionEvent(rawEvent: unknown): Record<string, unknown> {
  const raw = isRecord(rawEvent) ? rawEvent : {};
  const event = raw.payload || raw.event || raw;
  if (isRecord(event) && raw.event_ref) return { ...event, event_ref: raw.event_ref, seq: event.seq || raw.seq };
  if (isRecord(event) && raw.seq && !event.seq) return { ...event, seq: raw.seq };
  return isRecord(event) ? event : {};
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
