import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const source = await readFile(new URL("../app.js", import.meta.url), "utf8");
const bridgeCallsSource = await readFile(new URL("../bridge_calls.js", import.meta.url), "utf8");
const sessionHistorySource = await readFile(new URL("../session_history.js", import.meta.url), "utf8");

test("production UI uses service.application_execution APIs instead of browser tool loop", () => {
  assert.match(bridgeCallsSource, /capability: "app\.execution"/);
  assert.match(source, /callAppExecution\("start"/);
  assert.match(source, /\/execution\/events\/ws/);
  assert.match(sessionHistorySource, /callAppExecution\("replay"/);
  assert.match(source, /callAppExecution\("current-state"/);
  assert.match(source, /callAppExecution\("control"/);
  assert.doesNotMatch(source, /fetch\(/);
  assert.doesNotMatch(sessionHistorySource, /fetch\(/);
  assert.match(source, /debug_tool_loop/);
  assert.doesNotMatch(source, /import \{ WorkbenchToolLoopController \}/);
});

test("UI keeps event arrays as render caches reconstructed from replay", () => {
  assert.match(sessionHistorySource, /const replayedEvents = \(replay\.events \|\| \[\]\)\.map/);
  assert.match(sessionHistorySource, /state\.events = replayedEvents/);
  assert.match(sessionHistorySource, /state\.currentState = replay\.current_state/);
  assert.match(source, /new WebSocket/);
  assert.doesNotMatch(source, /new EventSource/);
});

test("UI falls back to generic session history when protocol replay is empty", () => {
  assert.match(sessionHistorySource, /async function loadGenericSessionHistoryEvents/);
  assert.match(sessionHistorySource, /function hasLocalExecutionEvents/);
  assert.match(sessionHistorySource, /!preserveLocalOnEmpty \|\| !hasLocalExecutionEvents\(\)/);
  assert.match(bridgeCallsSource, /function callSessionRead/);
  assert.match(bridgeCallsSource, /capability: "session\.read"/);
  assert.match(sessionHistorySource, /callSessionRead\("events", \{ session_id: state\.sessionId, limit: 100 \}\)/);
  assert.match(sessionHistorySource, /callSessionRead\("detail", \{ session_id: state\.sessionId \}\)/);
  assert.match(sessionHistorySource, /type: "session_event"/);
  assert.match(sessionHistorySource, /type: "session_turn"/);
  assert.match(sessionHistorySource, /state\.events = await loadGenericSessionHistoryEvents\(\)/);
});

test("UI switches app-owned execution streams when host session changes", () => {
  assert.match(source, /WorkbenchSessionMementoStore/);
  assert.match(source, /message\.type === "macaca\.session\.changed"/);
  assert.match(source, /async function switchSession/);
  assert.match(source, /await sessionHistory\.replayEvents\(\{ preserveLocalOnEmpty: true \}\)/);
  assert.match(source, /closeExecutionSocket\(false\)/);
});

test("UI refreshes durable replay before opening websocket increments", () => {
  assert.match(source, /sessionHistory\.replayEvents\(\)\s*\n\s*\.then\(refreshCurrentState\)\s*\n\s*\.then\(openExecutionWebSocket\)/);
  assert.match(source, /await sessionHistory\.replayEvents\(\);\s*\n\s*await refreshCurrentState\(\);\s*\n\s*openExecutionWebSocket\(\);/);
});
