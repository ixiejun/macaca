import assert from "node:assert/strict";
import test from "node:test";

const storage = new Map();
globalThis.window = {
  localStorage: {
    getItem: (key) => storage.get(key) || null,
    setItem: (key, value) => storage.set(key, value),
  },
};

const { WorkbenchSessionMementoStore } = await import("../session_memento.js");

test("session mementos restore the latest execution session after iframe refresh", () => {
  const firstStore = new WorkbenchSessionMementoStore({ storageKey: "test-mementos" });
  firstStore.save({
    sessionId: "workbench-refresh-session",
    runId: "run-refresh-session",
    eventCursor: "event/42",
    currentState: { lifecycle_state: "Running" },
    events: [{ type: "execution_event", data: { seq: 42 } }],
    result: "previous visible result",
    running: true,
    tokenSummary: "Running through service.application_execution",
  });

  const refreshedStore = new WorkbenchSessionMementoStore({ storageKey: "test-mementos" });
  const restored = refreshedStore.restore(null);

  assert.equal(restored.sessionId, "workbench-refresh-session");
  assert.equal(restored.runId, "run-refresh-session");
  assert.equal(restored.eventCursor, "event/42");
  assert.equal(restored.events.length, 1);
});
