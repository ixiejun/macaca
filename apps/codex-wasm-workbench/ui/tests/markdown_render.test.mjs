import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const markdownSource = await readFile(new URL("../src/markdown.tsx", import.meta.url), "utf8");

test("markdown renderer preserves inline fenced file trees as code blocks", () => {
  assert.match(markdownSource, /while \(line\.includes\('```'\)\)/);
  assert.match(markdownSource, /codeLines\.push\(line\)/);
  assert.match(markdownSource, /data-language/);
});

test("markdown renderer builds tables as block elements", () => {
  assert.match(markdownSource, /function parseMarkdownTable/);
  assert.match(markdownSource, /<table key=\{index\}>/);
  assert.match(markdownSource, /<thead>/);
  assert.match(markdownSource, /<tbody>/);
});

test("markdown renderer separates inline rules and headings from prose", () => {
  assert.match(markdownSource, /function splitInlineRulesAndHeadings/);
  assert.match(markdownSource, /\(---\|\\#\{1,3\}\\s\+/);
  assert.match(markdownSource, /kind: 'rule'/);
});

test("markdown renderer expands compact pipe tables emitted on one line", () => {
  assert.match(markdownSource, /function expandCompactPipeTable/);
  assert.match(markdownSource, /text\.includes\('\|\|'\)/);
  assert.match(markdownSource, /split\(\/\\s\*\\\|\\\|\\s\*\/g\)/);
});
