// Workbench bridge calls are small Command helpers over Macaca's generic
// app-owned UI postMessage ABI.  They centralize timeout handling, structured
// logging, and response unwrapping so the UI entrypoint can focus on state
// transitions instead of transport mechanics.

export function createWorkbenchBridgeCalls({ state, pendingBridgeCalls, hostOrigin, bridgeOutput }) {
  function callAppExecution(operation, payload = {}) {
    // App-owned UI bundles run in strict iframes, so production application
    // execution transport goes through the generic host bridge. The Workbench
    // owns the execution DTOs it sends and the events it renders; the shell only
    // adapts the declared `app.execution` capability to backend HTTP endpoints.
    const commandId = crypto.randomUUID();
    return new Promise((resolve, reject) => {
      const timeout = window.setTimeout(() => {
        pendingBridgeCalls.delete(commandId);
        reject(new Error(`app.execution/${operation} timed out`));
      }, 120000);
      pendingBridgeCalls.set(commandId, {
        resolve: (response) => {
          const output = bridgeOutput(response);
          if (response?.accepted === false) {
            reject(new Error(JSON.stringify(output || { error: "Application execution bridge rejected" })));
            return;
          }
          resolve(output);
        },
        reject,
        timeout,
      });
      console.info("[codex-wasm-workbench] app.execution bridge call", {
        operation,
        session_id: payload?.session_id || state.sessionId,
        run_id: payload?.run_id || payload?.scope?.run_id || state.runId,
      });
      window.parent.postMessage(
        {
          type: "macaca.call",
          command_id: commandId,
          session_id: state.sessionId,
          capability: "app.execution",
          operation,
          payload,
        },
        hostOrigin,
      );
    });
  }

  function callSessionRead(operation, payload = {}) {
    // App-owned UI bundles run under a strict iframe CSP, so they cannot assume
    // direct access to shell HTTP endpoints. The generic `session.read` bridge
    // lets the host adapt durable session history through declared capabilities
    // while the Workbench remains the sole owner of how that history is
    // rendered.
    const commandId = crypto.randomUUID();
    return new Promise((resolve, reject) => {
      const timeout = window.setTimeout(() => {
        pendingBridgeCalls.delete(commandId);
        reject(new Error(`session.read/${operation} timed out`));
      }, 120000);
      pendingBridgeCalls.set(commandId, {
        resolve: (response) => resolve(bridgeOutput(response)),
        reject,
        timeout,
      });
      window.parent.postMessage(
        {
          type: "macaca.call",
          command_id: commandId,
          session_id: state.sessionId,
          capability: "session.read",
          operation,
          payload,
        },
        hostOrigin,
      );
    });
  }

  return { callAppExecution, callSessionRead };
}
