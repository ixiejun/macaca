import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const markdownSource = await readFile(new URL("../src/markdown.tsx", import.meta.url), "utf8");
const normalizerSource = await readFile(new URL("../src/markdown_normalizer.js", import.meta.url), "utf8");
const styleSource = await readFile(new URL("../styles.css", import.meta.url), "utf8");
const packageSource = await readFile(new URL("../package.json", import.meta.url), "utf8");
const { normalizeWorkbenchMarkdown } = await import("../src/markdown_normalizer.js");

test("markdown renderer delegates parsing and code highlighting to open-source libraries", () => {
  assert.match(packageSource, /"react-markdown"/);
  assert.match(packageSource, /"remark-gfm"/);
  assert.match(packageSource, /"rehype-highlight"/);
  assert.match(packageSource, /"highlight\.js"/);
  assert.match(markdownSource, /import ReactMarkdown from 'react-markdown'/);
  assert.match(markdownSource, /import remarkGfm from 'remark-gfm'/);
  assert.match(markdownSource, /import rehypeHighlight from 'rehype-highlight'/);
  assert.match(markdownSource, /highlight\.js\/styles\/github-dark\.css/);
});

test("markdown renderer keeps Workbench logic as a presentation adapter", () => {
  assert.match(markdownSource, /export function MarkdownView/);
  assert.match(markdownSource, /remarkPlugins=\{\[remarkGfm\]\}/);
  assert.match(markdownSource, /rehypePlugins=\{\[rehypeHighlight\]\}/);
  assert.match(markdownSource, /export \{ normalizeWorkbenchMarkdown \}/);
  assert.match(normalizerSource, /export function normalizeWorkbenchMarkdown/);
  assert.doesNotMatch(markdownSource, /type MarkdownBlock/);
  assert.doesNotMatch(markdownSource, /function parseMarkdown/);
  assert.doesNotMatch(markdownSource, /function renderInline/);
});

test("markdown renderer wraps GFM tables in a readable scroll region", () => {
  assert.match(markdownSource, /function MarkdownTable/);
  assert.match(markdownSource, /markdown-table-scroll/);
  assert.match(normalizerSource, /headingWithTableHeader/);
  assert.match(normalizerSource, /function formatTableRow/);
  assert.match(normalizerSource, /isFormattedTableRow\(part\) \? \[part\] : splitCompactHeadingsAndRules\(part\)/);
  assert.match(normalizerSource, /firstRowCells\.length > expectedCellCount/);
  assert.match(normalizerSource, /function formatCompactTableHeading/);
  assert.match(normalizerSource, /function isSeparatorCells/);
  assert.match(normalizerSource, /function normalizeLooseCompactTableDelimiters/);
  assert.match(normalizerSource, /function expandInlineCompactPipeTable/);
  assert.match(normalizerSource, /function expandCompactTableRows/);
  assert.match(normalizerSource, /function flushPendingCompactCells/);
  assert.match(normalizerSource, /function splitInlineTableCells/);
  assert.match(normalizerSource, /function groupInlineTableCells/);
  assert.match(normalizerSource, /function mergeOverflowTableCells/);
  assert.match(normalizerSource, /cells\.map\(\(\) => '---'\)/);
  assert.match(normalizerSource, /findSeparatorMatch/);
  assert.match(normalizerSource, /separator row is the structural anchor/);
  assert.match(normalizerSource, /Keep numbered text inside headings intact/);
  assert.match(normalizerSource, /\\s\+\(-\\s\+\)/);
  assert.match(normalizerSource, /\\s\+\(\\d\+\\.\\s\+\)/);
  assert.match(styleSource, /\.markdown-table-scroll[\s\S]*overflow-x: auto/);
  assert.match(styleSource, /\.markdown-body table[\s\S]*table-layout: auto/);
  assert.match(styleSource, /\.markdown-body table[\s\S]*min-width: 920px/);
  assert.match(styleSource, /\.markdown-body th,[\s\S]*min-width: 180px/);
  assert.match(styleSource, /\.markdown-body th,[\s\S]*word-break: keep-all/);
  assert.match(styleSource, /\.markdown-body th[\s\S]*text-transform: uppercase/);
  assert.match(styleSource, /\.markdown-body td code[\s\S]*white-space: nowrap/);
  assert.match(styleSource, /\.markdown-body tbody tr:nth-child\(even\) td/);
});

test("markdown normalizer restores compact assistant review tables into GFM rows", () => {
  const compact = "## 🔍 结构化审查报告 --- ### ✅ 1. 计划对齐 — 对照 Planner Handoff | 计划要求 | 实现状态 | 证据 | |---|---|---| | Flask 后端，含 4 个直辖市定义 | ✅ **完全符合** | `MUNICIPALITIES` 字典包含北京/上海/天津/重庆，坐标精确 | | `GET /api/weather/all` 全部城市 JSON | ✅ **完全符合** | 返回完整 JSON，含容错字段 | ---";
  const normalized = normalizeWorkbenchMarkdown(compact);
  assert.match(normalized, /\| 计划要求 \| 实现状态 \| 证据 \|/);
  assert.match(normalized, /\| --- \| --- \| --- \|/);
  assert.match(normalized, /### ✅ 1\. 计划对齐 — 对照 Planner Handoff/);
  assert.doesNotMatch(normalized, /### ✅\n1\. 计划对齐/);
  assert.match(normalized, /\| Flask 后端，含 4 个直辖市定义 \| ✅ \*\*完全符合\*\* \| `MUNICIPALITIES` 字典包含北京\/上海\/天津\/重庆，坐标精确 \|/);
  assert.doesNotMatch(normalized, /\| 计划要求 \| 实现状态 \| 证据 \| \|---\|---\|---\|/);
});

test("markdown renderer preserves readable code blocks with syntax highlighting styles", () => {
  assert.match(styleSource, /\.markdown-body pre[\s\S]*overflow-x: auto/);
  assert.match(styleSource, /\.markdown-body pre[\s\S]*white-space: pre-wrap/);
  assert.match(styleSource, /\.markdown-body pre code[\s\S]*background: transparent/);
  assert.match(styleSource, /\.markdown-body code[\s\S]*color: #d9f7e8/);
});
