import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const source = await readFile(new URL("../app.js", import.meta.url), "utf8");

test("production UI uses service.application_execution APIs instead of browser tool loop", () => {
  assert.match(source, /\/execution\/start/);
  assert.match(source, /\/execution\/events\/ws/);
  assert.match(source, /\/execution\/replay/);
  assert.match(source, /\/execution\/current-state/);
  assert.match(source, /\/execution\/control/);
  assert.match(source, /debug_tool_loop/);
  assert.doesNotMatch(source, /import \{ WorkbenchToolLoopController \}/);
});

test("UI keeps event arrays as render caches reconstructed from replay", () => {
  assert.match(source, /const replayedEvents = \(replay\.events \|\| \[\]\)\.map/);
  assert.match(source, /state\.events = replayedEvents/);
  assert.match(source, /state\.currentState = replay\.current_state/);
  assert.match(source, /new WebSocket/);
  assert.doesNotMatch(source, /new EventSource/);
});

test("UI falls back to generic session history when protocol replay is empty", () => {
  assert.match(source, /async function loadGenericSessionHistoryEvents/);
  assert.match(source, /getJson\(`\/api\/sessions\/\$\{encodeURIComponent\(state\.sessionId\)\}\/events\?limit=100`\)/);
  assert.match(source, /getJson\(`\/api\/sessions\/detail\/\$\{encodeURIComponent\(state\.sessionId\)\}`\)/);
  assert.match(source, /type: "session_event"/);
  assert.match(source, /type: "session_turn"/);
  assert.match(source, /state\.events = await loadGenericSessionHistoryEvents\(\)/);
});

test("UI switches app-owned execution streams when host session changes", () => {
  assert.match(source, /WorkbenchSessionMementoStore/);
  assert.match(source, /message\.type === "macaca\.session\.changed"/);
  assert.match(source, /async function switchSession/);
  assert.match(source, /await replayEvents\(\{ preserveLocalOnEmpty: true \}\)/);
  assert.match(source, /closeExecutionSocket\(false\)/);
});

test("UI refreshes durable replay before opening websocket increments", () => {
  assert.match(source, /replayEvents\(\)\s*\n\s*\.then\(refreshCurrentState\)\s*\n\s*\.then\(openExecutionWebSocket\)/);
  assert.match(source, /await replayEvents\(\);\s*\n\s*await refreshCurrentState\(\);\s*\n\s*openExecutionWebSocket\(\);/);
});
