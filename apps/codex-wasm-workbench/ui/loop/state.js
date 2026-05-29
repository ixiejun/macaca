// This module owns the explicit finite-state machine for the Workbench
// LLM/tool loop.  Keeping state transitions in one small file makes loop
// behavior auditable and prevents UI rendering code from silently changing
// execution semantics.
export const LOOP_STATES = Object.freeze({
  IDLE: "idle",
  LLM_CALL: "llm_call",
  TOOL_DISPATCH: "tool_dispatch",
  TOOL_RESULT: "tool_result",
  APPROVAL_WAIT: "approval_wait",
  FAILED: "failed",
  COMPLETE: "complete",
});

const TERMINAL_STATES = new Set([
  LOOP_STATES.APPROVAL_WAIT,
  LOOP_STATES.FAILED,
  LOOP_STATES.COMPLETE,
]);

export function createLoopState(options = {}) {
  return {
    state: LOOP_STATES.IDLE,
    iteration: 0,
    maxIterations: options.maxIterations ?? 12,
    maxToolCalls: options.maxToolCalls ?? 64,
    toolCalls: 0,
    history: [],
  };
}

export function transitionLoopState(loopState, nextState, metadata = {}) {
  if (TERMINAL_STATES.has(loopState.state)) {
    throw new Error(`cannot leave terminal state ${loopState.state}`);
  }
  loopState.state = nextState;
  if (Number.isInteger(metadata.iteration)) {
    loopState.iteration = metadata.iteration;
  }
  loopState.history.push({
    state: nextState,
    metadata: boundedMetadata(metadata),
    at: new Date().toISOString(),
  });
  return loopState;
}

export function assertLoopCanContinue(loopState) {
  if (loopState.iteration >= loopState.maxIterations) {
    return {
      ok: false,
      reason: "iteration_budget_exhausted",
    };
  }
  if (loopState.toolCalls >= loopState.maxToolCalls) {
    return {
      ok: false,
      reason: "tool_call_budget_exhausted",
    };
  }
  return { ok: true };
}

function boundedMetadata(metadata) {
  const encoded = JSON.stringify(metadata);
  if (encoded.length <= 1024) return metadata;
  return {
    truncated: true,
    preview: encoded.slice(0, 1024),
  };
}
