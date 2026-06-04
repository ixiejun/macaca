import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const source = await readFile(new URL("../app.js", import.meta.url), "utf8");
const bridgeCallsSource = await readFile(new URL("../bridge_calls.js", import.meta.url), "utf8");
const sessionHistorySource = await readFile(new URL("../session_history.js", import.meta.url), "utf8");
const renderSource = await readFile(new URL("../render.js", import.meta.url), "utf8");

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
  assert.match(sessionHistorySource, /callSessionRead\("events", \{ session_id: state\.sessionId, limit: 1000 \}\)/);
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
  assert.match(source, /if \(!state\.eventSocket && state\.running\) openExecutionWebSocket\(\);/);
});

test("UI subscribes to execution events before awaiting hosted start completion", () => {
  const immediateSocketIndex = source.indexOf("openExecutionWebSocket();");
  const startBridgeIndex = source.indexOf('const result = await callAppExecution("start"');
  assert.ok(immediateSocketIndex > 0, "startTask should open the websocket");
  assert.ok(startBridgeIndex > 0, "startTask should still call the application execution bridge");
  assert.ok(
    immediateSocketIndex < startBridgeIndex,
    "the websocket must open before awaiting the hosted start call so live EventLog rows render while the backend is still running",
  );
  assert.match(source, /appendEvent\("stream_connected"/);
});

test("timeline presenter renders user-readable execution events instead of raw protocol JSON", () => {
  assert.match(renderSource, /export function presentTimelineEvent/);
  assert.match(renderSource, /function presentExecutionEvent/);
  assert.match(renderSource, /data\.display_title \|\| eventTitle/);
  assert.match(renderSource, /data\.display_body \|\|/);
  assert.match(renderSource, /payload\.summary \|\|/);
  assert.match(renderSource, /function renderMarkdownDocument/);
  assert.match(renderSource, /renderDisplayBody\(view\)/);
  assert.doesNotMatch(renderSource, /body\.textContent = typeof entry\.data === "string" \? entry\.data : JSON\.stringify\(entry\.data, null, 2\)/);
});

test("timeline presenter keeps the complete visible stream and markdown body", () => {
  assert.match(renderSource, /\.\.\.state\.events\.map/);
  assert.doesNotMatch(renderSource, /slice\(-80\)/);
  assert.match(renderSource, /format: "markdown"/);
  assert.match(renderSource, /appendInlineMarkdown/);
});
