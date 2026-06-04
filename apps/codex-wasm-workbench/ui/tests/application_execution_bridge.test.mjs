import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const appSource = await readFile(new URL("../src/WorkbenchApp.tsx", import.meta.url), "utf8");
const bridgeSource = await readFile(new URL("../src/bridge.ts", import.meta.url), "utf8");
const presenterSource = await readFile(new URL("../src/presenter.ts", import.meta.url), "utf8");
const markdownSource = await readFile(new URL("../src/markdown.tsx", import.meta.url), "utf8");
const styleSource = await readFile(new URL("../styles.css", import.meta.url), "utf8");
const indexSource = await readFile(new URL("../index.html", import.meta.url), "utf8");
const appYamlSource = await readFile(new URL("../../app.yaml", import.meta.url), "utf8");
const viteConfigSource = await readFile(new URL("../vite.config.ts", import.meta.url), "utf8");

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

test("timeline presenter renders user-readable execution events and markdown", () => {
  assert.match(presenterSource, /export function presentTimelineEvent/);
  assert.match(presenterSource, /function presentExecutionEvent/);
  assert.match(presenterSource, /data\.display_title \|\| eventTitle/);
  assert.match(presenterSource, /data\.display_body/);
  assert.match(markdownSource, /export function MarkdownView/);
  assert.match(markdownSource, /function normalizeMarkdownLines/);
});

test("markdown display wraps generated content instead of forcing horizontal scrolling", () => {
  assert.match(styleSource, /\.markdown-body pre[\s\S]*white-space: pre-wrap/);
  assert.match(styleSource, /\.markdown-body pre[\s\S]*overflow-wrap: anywhere/);
  assert.match(styleSource, /\.markdown-body table[\s\S]*table-layout: fixed/);
  assert.match(styleSource, /\.markdown-body th,[\s\S]*word-break: break-word/);
});
