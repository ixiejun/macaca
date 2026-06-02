// This module implements the Workbench-side Session History Adapter.  The
// adapter keeps protocol replay, durable shell history fallback, and render-cache
// reconstruction inside the application UI, so Macaca OS only exposes generic
// session-history and application-execution contracts.

const EXECUTION_EVENT_TYPES = new Set([
  "execution_event",
  "execution_start_result",
  "control_result",
  "tool_dispatch",
  "tool_result",
  "final_answer",
  "loop_start",
  "loop_failed",
  "llm_call",
]);

export function createSessionHistoryAdapter(dependencies) {
  const {
    state,
    callAppExecution,
    callSessionRead,
    normalizeExecutionEvent,
    sessionContextEvent,
    sessionMementos,
    renderTimeline,
    renderResult,
  } = dependencies;

  function hasLocalExecutionEvents() {
    // Startup diagnostics such as model catalog loading are useful, but they
    // must not block durable session-history recovery after a sidebar
    // selection. Only actual task/run events are treated as local execution
    // stream content worth preserving when protocol replay is empty.
    return state.events.some((entry) => EXECUTION_EVENT_TYPES.has(entry.type));
  }

  async function loadGenericSessionHistoryEvents() {
    // Application-execution replay is the authoritative protocol view when the
    // selected session was started through `service.application_execution`.
    // Historical Workbench sessions may instead be legacy shell sessions or
    // generic app-owned bridge sessions. This Adapter keeps that compatibility
    // local to the application UI by reading Macaca's provider-neutral session
    // history endpoints; it does not ask the OS to understand Codex-specific
    // execution stream semantics.
    const [eventHistory, sessionDetail] = await Promise.all([
      callSessionRead("events", { session_id: state.sessionId, limit: 100 }).catch((error) => {
        console.warn("[codex-wasm-workbench] generic session event replay failed", {
          session_id: state.sessionId,
          error: error instanceof Error ? error.message : String(error),
        });
        return null;
      }),
      callSessionRead("detail", { session_id: state.sessionId }).catch((error) => {
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

    // EventLog rows are the most granular history. Stored turns are still useful
    // when a legacy or bridge-projected session has no EventLog rows, so they
    // are used as a bounded fallback rather than merged into a duplicated
    // stream.
    if (sessionEvents.length > 0) return sessionEvents;
    return turnEvents;
  }

  async function replayEvents({ preserveLocalOnEmpty = false } = {}) {
    if (!state.sessionId) return;
    const params = {
      session_id: state.sessionId,
      trace_id: `trace-replay-${state.sessionId}`,
      page_size: "100",
    };
    if (state.runId) params.run_id = state.runId;
    const replay = await callAppExecution("replay", params).catch((error) => {
      console.warn("[codex-wasm-workbench] protocol replay failed; falling back to generic session history", {
        session_id: state.sessionId,
        error: error instanceof Error ? error.message : String(error),
      });
      return { events: [], next_cursor: null, current_state: null };
    });
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
    } else if (!preserveLocalOnEmpty || !hasLocalExecutionEvents()) {
      state.events = await loadGenericSessionHistoryEvents();
      if (state.events.length === 0) {
        state.events = [sessionContextEvent("No durable session history is stored for this session yet.")];
      }
    }
    sessionMementos.save(state);
    renderTimeline(state);
    renderResult(state);
  }

  return { replayEvents };
}
