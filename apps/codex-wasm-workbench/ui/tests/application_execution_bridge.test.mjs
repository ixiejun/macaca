import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const appSource = await readFile(new URL("../src/WorkbenchApp.tsx", import.meta.url), "utf8");
const bridgeSource = await readFile(new URL("../src/bridge.ts", import.meta.url), "utf8");
const presenterSource = await readFile(new URL("../src/presenter.ts", import.meta.url), "utf8");
const stateSource = await readFile(new URL("../src/state.ts", import.meta.url), "utf8");
const timelineSource = await readFile(new URL("../src/timeline.tsx", import.meta.url), "utf8");
const collaborationSource = await readFile(new URL("../src/collaboration_panel.tsx", import.meta.url), "utf8");
const markdownSource = await readFile(new URL("../src/markdown.tsx", import.meta.url), "utf8");
const styleSource = await readFile(new URL("../styles.css", import.meta.url), "utf8");
const indexSource = await readFile(new URL("../index.html", import.meta.url), "utf8");
const appYamlSource = await readFile(new URL("../../app.yaml", import.meta.url), "utf8");
const viteConfigSource = await readFile(new URL("../vite.config.ts", import.meta.url), "utf8");
const hostBridgeSource = await readFile(new URL("../../../../frontend/lib/app-ui-bridge.ts", import.meta.url), "utf8");

test("workbench UI is a Vite React TypeScript app-owned bundle", () => {
  assert.match(indexSource, /<div id="root"><\/div>/);
  assert.match(indexSource, /src="\/src\/main\.tsx"/);
  assert.match(appSource, /export function WorkbenchApp/);
  assert.match(appYamlSource, /framework: react/);
  assert.match(appYamlSource, /entry: ui\/dist\/index\.html/);
  assert.match(viteConfigSource, /base: '\.\/'/);
});

test("React UI uses bridge adapter instead of direct backend fetches", () => {
  assert.match(bridgeSource, /type: 'macaca\.call'/);
  assert.match(bridgeSource, /capability: 'app\.execution'/);
  assert.match(bridgeSource, /capability: 'service\.call'/);
  assert.match(bridgeSource, /capability: 'session\.read'/);
  assert.doesNotMatch(appSource, /fetch\(/);
  assert.doesNotMatch(bridgeSource, /fetch\(/);
});

test("production UI still uses service.application_execution operations", () => {
  assert.match(appSource, /callAppExecution\(bridgeRef\.current, 'start'/);
  assert.match(appSource, /callAppExecution\(bridgeRef\.current, 'replay'/);
  assert.match(appSource, /callAppExecution\(bridgeRef\.current, 'current-state'/);
  assert.match(appSource, /callAppExecution\(bridgeRef\.current, 'control'/);
  assert.match(appSource, /new WebSocket/);
  assert.doesNotMatch(appSource, /new EventSource/);
  assert.doesNotMatch(appSource, /import\('\.\.\/loop\/controller\.js'\)/);
  assert.doesNotMatch(appSource, /startDebugToolLoop/);
});

test("execution replay uses application execution projection as the only timeline source", () => {
  assert.match(appSource, /callAppExecution\(bridgeRef\.current, 'replay'/);
  assert.doesNotMatch(appSource, /callSessionRead\(bridgeRef\.current, 'events'/);
  assert.doesNotMatch(appSource, /loadGenericSessionHistoryEvents/);
  assert.doesNotMatch(appSource, /type: 'session_event'/);
});

test("each submitted task receives a fresh execution session", () => {
  assert.match(appSource, /function beginNewExecutionSession/);
  assert.match(appSource, /sessionId: `workbench-\$\{commandId\}`/);
  assert.match(appSource, /runId: `run-\$\{commandId\}`/);
  assert.match(appSource, /const commandId = beginNewExecutionSession\(\);/);
  assert.doesNotMatch(appSource, /sessionId \|\|=/);
});

test("React UI switches app-owned execution streams when host session changes", () => {
  assert.match(appSource, /WorkbenchSessionMementoStore/);
  assert.match(appSource, /message\.type === 'macaca\.session\.changed'/);
  assert.match(appSource, /async function switchSession/);
  assert.match(appSource, /await replayEvents\(\)/);
  assert.match(appSource, /closeExecutionSocket\(false\)/);
});

test("Workbench collaboration panel reads Macaca-owned task and agent state", () => {
  assert.match(appSource, /refreshCollaborationState/);
  assert.match(appSource, /callSessionRead\(bridgeRef\.current, 'task-board'/);
  assert.match(appSource, /callSessionRead\(bridgeRef\.current, 'agents'/);
  assert.match(appSource, /CollaborationPanel tasks=\{state\.taskBoard\} agents=\{state\.agents\}/);
  assert.match(collaborationSource, /Macaca-owned task and agent collaboration state/);
  assert.match(collaborationSource, /Task board is empty/);
  assert.match(collaborationSource, /No active task reported/);
  assert.match(hostBridgeSource, /message\.operation === 'task-board'/);
  assert.ok(hostBridgeSource.includes("`/api/apps/${options.app.id}/todos?${query.toString()}`"));
  assert.match(hostBridgeSource, /message\.operation === 'agents'/);
  assert.ok(hostBridgeSource.includes("`/api/apps/${options.app.id}/agents?${query.toString()}`"));
  assert.doesNotMatch(collaborationSource, /execution_event/);
  assert.doesNotMatch(collaborationSource, /CODEX-WASM-WORKBENCH task/);
});

test("Workbench timeline deduplicates replayed and websocket execution events", () => {
  assert.match(stateSource, /export function deduplicateTimelineEvents/);
  assert.match(stateSource, /export function timelineEventKey/);
  assert.match(stateSource, /event_ref/);
  assert.match(stateSource, /execution:seq/);
  assert.match(stateSource, /idempotency_key/);
  assert.match(stateSource, /state\.events\.some/);
  assert.match(stateSource, /deduplicateTimelineEvents\(action\.events\)/);
});

test("timeline presenter renders user-readable execution events and markdown", () => {
  assert.match(presenterSource, /export function presentTimelineEvent/);
  assert.match(presenterSource, /function presentExecutionEvent/);
  assert.match(presenterSource, /function eventBody/);
  assert.match(presenterSource, /function renderEmbeddedJsonBody/);
  assert.match(presenterSource, /function extractEmbeddedJsonObject/);
  assert.match(presenterSource, /function extractJsonStringProperty/);
  assert.match(presenterSource, /function decodeJsonStringFragment/);
  assert.match(presenterSource, /JSON\.parse/);
  assert.match(presenterSource, /function markdownCodeFence/);
  assert.match(presenterSource, /function inferCodeLanguage/);
  assert.match(presenterSource, /function eventCardTitle/);
  assert.match(presenterSource, /Writing file:/);
  assert.match(presenterSource, /function eventSummary/);
  assert.match(presenterSource, /function eventTone/);
  assert.match(presenterSource, /function eventBodyFormat/);
  assert.match(presenterSource, /function suppressDuplicateBody/);
  assert.match(presenterSource, /data\.display_format === 'markdown'/);
  assert.match(presenterSource, /event\.event_type === 'LlmCompleted'/);
  assert.match(presenterSource, /data\.display_body/);
  assert.match(timelineSource, /event-summary/);
  assert.match(timelineSource, /function TimelineOverview/);
  assert.match(timelineSource, /function timelineCounts/);
  assert.match(timelineSource, /event-section-header/);
  assert.match(timelineSource, /function timelineSection/);
  assert.match(timelineSource, /Trace details/);
  assert.match(markdownSource, /export function MarkdownView/);
  assert.match(markdownSource, /ReactMarkdown/);
  assert.match(markdownSource, /remarkPlugins=\{\[remarkGfm, remarkBreaks\]\}/);
  assert.doesNotMatch(markdownSource, /normalizeWorkbenchMarkdown/);
});

test("timeline presenter turns embedded tool JSON content into fenced code", () => {
  assert.match(presenterSource, /### Tool parameters/);
  assert.match(presenterSource, /parameterList/);
  assert.match(presenterSource, /escapeInlineCode\(humanize\(key\)\)/);
  assert.match(presenterSource, /function escapeInlineCode/);
  assert.match(presenterSource, /### Content/);
  assert.match(presenterSource, /Content preview/);
  assert.match(presenterSource, /truncated by the event summary/);
  assert.match(presenterSource, /markdownCodeFence\(language, content\)/);
  assert.match(presenterSource, /case 'py': return 'python'/);
  assert.match(presenterSource, /case 'html': return 'html'/);
  assert.match(presenterSource, /return 'markdown'/);
  assert.match(presenterSource, /return 'python'/);
  assert.match(presenterSource, /return renderEmbeddedJsonBody\(rawBody, filePath\) \|\| rawBody/);
  assert.doesNotMatch(presenterSource, /weather-app/);
  assert.doesNotMatch(presenterSource, /中国直辖市/);
});

test("markdown display wraps prose while keeping dense tables inspectable", () => {
  assert.match(styleSource, /\.markdown-body pre[\s\S]*white-space: pre-wrap/);
  assert.match(styleSource, /\.markdown-body pre[\s\S]*overflow-wrap: anywhere/);
  assert.match(styleSource, /\.markdown-table-scroll[\s\S]*overflow-x: auto/);
  assert.match(styleSource, /\.markdown-raw-table[\s\S]*overflow: auto/);
  assert.match(styleSource, /\.markdown-body table[\s\S]*table-layout: auto/);
  assert.match(styleSource, /\.markdown-body table[\s\S]*width: max-content/);
  assert.match(styleSource, /\.markdown-body th:first-child,[\s\S]*white-space: nowrap/);
  assert.match(styleSource, /\.markdown-body th:first-child,[\s\S]*min-width: 168px/);
  assert.match(styleSource, /\.markdown-body th:nth-child\(2\),[\s\S]*min-width: 320px/);
  assert.match(styleSource, /\.markdown-body td code,[\s\S]*overflow-wrap: anywhere/);
});

test("workbench panels can shrink so wide content scrolls inside the panel", () => {
  assert.match(styleSource, /\.editor-panel,\s*\.thread-panel,\s*\.diagnostics-panel,\s*\.result-panel,\s*\.side-panel[\s\S]*min-width: 0/);
  assert.match(styleSource, /\.markdown-raw-table[\s\S]*white-space: pre;/);
});
