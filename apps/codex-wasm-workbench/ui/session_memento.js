// Session memento storage for the app-owned Workbench UI.
//
// The host shell can change the active session while keeping this iframe alive.
// This helper keeps the Workbench presentation state isolated per session
// without asking Macaca OS to understand Codex-specific execution streams.  The
// pattern is intentionally local to the application package: the shell only
// publishes generic session context, and the application owns how its UI state
// is captured and restored.

export class WorkbenchSessionMementoStore {
  constructor({ fallbackSessionId = "workbench-sessionless" } = {}) {
    this.fallbackSessionId = fallbackSessionId;
    this.snapshots = new Map();
  }

  // Resolve a stable map key for optional host session context.  Sessionless
  // states remain isolated under a fallback key so bridge catalog calls made
  // before a task starts do not pollute a later task session.
  keyFor(sessionId) {
    const trimmed = typeof sessionId === "string" ? sessionId.trim() : "";
    return trimmed.length > 0 ? trimmed : this.fallbackSessionId;
  }

  // Capture only presentation-safe fields.  Pending bridge promises and service
  // clients are deliberately excluded because they belong to the live iframe
  // runtime, not to a refreshable UI memento.
  save(state) {
    const key = this.keyFor(state.sessionId);
    this.snapshots.set(key, {
      sessionId: state.sessionId || null,
      events: cloneJson(state.events || []),
      result: state.result || "",
      running: Boolean(state.running),
      commandId: state.commandId || null,
      runId: state.runId || null,
      eventCursor: state.eventCursor || null,
      currentState: cloneJson(state.currentState || null),
      tokenSummary: state.tokenSummary || null,
    });
  }

  // Restore a previously captured session, or create a clean view for a session
  // that has no Workbench-local memento yet.  This keeps every sidebar session
  // visually distinct even before richer backend replay is attached.
  restore(sessionId) {
    const key = this.keyFor(sessionId);
    const saved = this.snapshots.get(key);
    if (saved) return cloneJson(saved);
    return {
      sessionId: sessionId || null,
      events: [],
      result: "",
      running: false,
      commandId: null,
      runId: null,
      eventCursor: null,
      currentState: null,
      tokenSummary: sessionId ? "No local stream replay for this session yet" : "No run yet",
    };
  }
}

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}
