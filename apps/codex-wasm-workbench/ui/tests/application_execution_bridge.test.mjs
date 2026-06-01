import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const source = await readFile(new URL("../app.js", import.meta.url), "utf8");

test("production UI uses service.application_execution APIs instead of browser tool loop", () => {
  assert.match(source, /\/execution\/start/);
  assert.match(source, /\/execution\/events/);
  assert.match(source, /\/execution\/replay/);
  assert.match(source, /\/execution\/current-state/);
  assert.match(source, /\/execution\/control/);
  assert.match(source, /debug_tool_loop/);
  assert.doesNotMatch(source, /import \{ WorkbenchToolLoopController \}/);
});

test("UI keeps event arrays as render caches reconstructed from replay", () => {
  assert.match(source, /state\.events = \(replay\.events \|\| \[\]\)\.map/);
  assert.match(source, /state\.currentState = replay\.current_state/);
  assert.match(source, /new EventSource/);
});
