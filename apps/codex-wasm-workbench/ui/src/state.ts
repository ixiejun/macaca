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
    taskBoard: [],
    agents: [],
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
  if (action.type === 'replaceEvents') return { ...state, events: deduplicateTimelineEvents(action.events) };
  if (action.type === 'clearRun') {
    return { ...state, events: [], result: '', currentState: null, tokenSummary: 'No run yet' };
  }
  const event = action.event;
  if (state.events.some((existingEvent) => timelineEventKey(existingEvent) === timelineEventKey(event))) {
    return state;
  }
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
    next.tokenSummary = 'Completed through application execution';
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

/**
 * Collapse duplicate timeline entries before they reach React rendering.
 *
 * The backend EventLog is append-only and ordered, but the app-owned UI can see
 * the same durable row through several presentation paths: local Memento
 * restore, replay, and WebSocket delivery after a reconnect.  This helper is a
 * UI-level Observer guard.  It does not change backend persistence or Macaca OS
 * event semantics; it only prevents a single durable event coordinate from
 * being rendered twice in this Workbench surface.
 */
export function deduplicateTimelineEvents(events: WorkbenchState['events']): WorkbenchState['events'] {
  const seen = new Set<string>();
  const deduplicated: WorkbenchState['events'] = [];
  for (const event of events) {
    const key = timelineEventKey(event);
    if (seen.has(key)) continue;
    seen.add(key);
    deduplicated.push(event);
  }
  return deduplicated;
}

/**
 * Build a stable identity for a timeline event.
 *
 * Application-execution events prefer the durable EventLog coordinate:
 * `event_ref`, `seq`, or `idempotency_key`.  Local UI-only events, such as
 * model catalog notifications, intentionally keep their timestamp in the key so
 * independent UI observations are not collapsed by accident.
 */
export function timelineEventKey(event: WorkbenchState['events'][number]): string {
  if (event.type === 'execution_event') {
    const execution = normalizeExecutionEvent(event.data);
    const eventRef = stringValue(execution.event_ref);
    if (eventRef) return `execution:event_ref:${eventRef}`;
    const sessionId = stringValue(execution.session_id);
    const seq = execution.seq;
    if (sessionId && (typeof seq === 'number' || typeof seq === 'string')) return `execution:seq:${sessionId}:${seq}`;
    const idempotencyKey = stringValue(execution.idempotency_key);
    if (idempotencyKey) return `execution:idempotency:${idempotencyKey}`;
  }
  return `local:${event.type}:${event.at}:${stableJson(event.data)}`;
}

function stableJson(value: unknown): string {
  if (!isRecord(value) && !Array.isArray(value)) return String(value ?? '');
  try {
    return JSON.stringify(value, objectKeySorter);
  } catch {
    return String(value);
  }
}

function objectKeySorter(_key: string, value: unknown): unknown {
  if (!isRecord(value)) return value;
  return Object.fromEntries(Object.entries(value).sort(([left], [right]) => left.localeCompare(right)));
}

function stringValue(value: unknown): string {
  return typeof value === 'string' ? value : '';
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
