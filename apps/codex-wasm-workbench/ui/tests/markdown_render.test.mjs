import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const markdownSource = await readFile(new URL("../src/markdown.tsx", import.meta.url), "utf8");
const styleSource = await readFile(new URL("../styles.css", import.meta.url), "utf8");
const packageSource = await readFile(new URL("../package.json", import.meta.url), "utf8");

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
  assert.match(markdownSource, /export function normalizeWorkbenchMarkdown/);
  assert.doesNotMatch(markdownSource, /type MarkdownBlock/);
  assert.doesNotMatch(markdownSource, /function parseMarkdown/);
  assert.doesNotMatch(markdownSource, /function renderInline/);
});

test("markdown renderer wraps GFM tables in a readable scroll region", () => {
  assert.match(markdownSource, /function MarkdownTable/);
  assert.match(markdownSource, /markdown-table-scroll/);
  assert.match(markdownSource, /headingWithTableHeader/);
  assert.match(markdownSource, /function formatTableRow/);
  assert.match(markdownSource, /isFormattedTableRow\(part\) \? \[part\] : splitCompactHeadingsAndRules\(part\)/);
  assert.match(markdownSource, /firstRowCells\.length > expectedCellCount/);
  assert.match(markdownSource, /function formatCompactTableHeading/);
  assert.match(markdownSource, /function isSeparatorCells/);
  assert.match(markdownSource, /function normalizeLooseCompactTableDelimiters/);
  assert.match(markdownSource, /function expandInlineCompactPipeTable/);
  assert.match(markdownSource, /function expandCompactTableRows/);
  assert.match(markdownSource, /function flushPendingCompactCells/);
  assert.match(markdownSource, /function splitInlineTableCells/);
  assert.match(markdownSource, /function groupInlineTableCells/);
  assert.match(markdownSource, /function mergeOverflowTableCells/);
  assert.match(markdownSource, /cells\.map\(\(\) => '---'\)/);
  assert.match(markdownSource, /separator row as the only structural anchor/);
  assert.match(markdownSource, /\\s\+\(-\\s\+\)/);
  assert.match(markdownSource, /\\s\+\(\\d\+\\.\\s\+\)/);
  assert.match(styleSource, /\.markdown-table-scroll[\s\S]*overflow: auto/);
  assert.match(styleSource, /\.markdown-body table[\s\S]*table-layout: fixed/);
  assert.match(styleSource, /\.markdown-body th[\s\S]*text-transform: uppercase/);
  assert.match(styleSource, /\.markdown-body td code[\s\S]*white-space: normal/);
  assert.match(styleSource, /\.markdown-body tbody tr:nth-child\(even\) td/);
});

test("markdown renderer preserves readable code blocks with syntax highlighting styles", () => {
  assert.match(styleSource, /\.markdown-body pre[\s\S]*overflow-x: auto/);
  assert.match(styleSource, /\.markdown-body pre[\s\S]*white-space: pre-wrap/);
  assert.match(styleSource, /\.markdown-body pre code[\s\S]*background: transparent/);
  assert.match(styleSource, /\.markdown-body code[\s\S]*color: #d9f7e8/);
});
