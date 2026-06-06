import assert from "node:assert/strict";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { join } from "node:path";
import test from "node:test";
import ts from "typescript";

const markdownSource = await readFile(new URL("../src/markdown.tsx", import.meta.url), "utf8");
const styleSource = await readFile(new URL("../styles.css", import.meta.url), "utf8");
const packageSource = await readFile(new URL("../package.json", import.meta.url), "utf8");

async function loadMarkdownModule() {
  const uiDir = fileURLToPath(new URL("../", import.meta.url));
  const dir = await mkdtemp(join(uiDir, ".tmp-markdown-test-"));
  const source = markdownSource.replace(/^import 'highlight\.js\/styles\/github-dark\.css';\n/m, "");
  const output = ts.transpileModule(source, {
    compilerOptions: {
      jsx: ts.JsxEmit.ReactJSX,
      module: ts.ModuleKind.ESNext,
      target: ts.ScriptTarget.ES2022,
    },
  }).outputText;
  const path = join(dir, "markdown.mjs");
  await writeFile(path, output);
  const module = await import(`${new URL(`file://${path}`).href}?t=${Date.now()}`);
  await rm(dir, { force: true, recursive: true });
  return module;
}

test("markdown renderer delegates parsing and code highlighting to open-source libraries", () => {
  assert.match(packageSource, /"react-markdown"/);
  assert.match(packageSource, /"remark-gfm"/);
  assert.match(packageSource, /"remark-breaks"/);
  assert.match(packageSource, /"rehype-highlight"/);
  assert.match(packageSource, /"highlight\.js"/);
  assert.match(markdownSource, /import ReactMarkdown from 'react-markdown'/);
  assert.match(markdownSource, /import remarkGfm from 'remark-gfm'/);
  assert.match(markdownSource, /import remarkBreaks from 'remark-breaks'/);
  assert.match(markdownSource, /import rehypeHighlight from 'rehype-highlight'/);
  assert.match(markdownSource, /highlight\.js\/styles\/github-dark\.css/);
});

test("markdown renderer keeps Workbench logic as a presentation adapter", () => {
  assert.match(markdownSource, /export function MarkdownView/);
  assert.match(markdownSource, /export function normalizeMarkdownInput/);
  assert.match(markdownSource, /remarkPlugins=\{\[remarkGfm, remarkBreaks\]\}/);
  assert.match(markdownSource, /rehypePlugins=\{\[rehypeHighlight\]\}/);
  assert.doesNotMatch(markdownSource, /normalizeWorkbenchMarkdown/);
  assert.doesNotMatch(markdownSource, /type MarkdownBlock/);
  assert.doesNotMatch(markdownSource, /function parseMarkdown/);
  assert.doesNotMatch(markdownSource, /function renderInline/);
  assert.doesNotMatch(markdownSource, /splitCompactMarkdownLine/);
});

test("markdown input normalization restores generic block boundaries before library rendering", () => {
  assert.match(markdownSource, /function restoreMarkdownBlockBoundaries/);
  assert.match(markdownSource, /function repairTitledPipeTables/);
  assert.match(markdownSource, /function repairTitledPipeTableAt/);
  assert.match(markdownSource, /function repairInlineTitledPipeTable/);
  assert.match(markdownSource, /function collectCompactTableData/);
  assert.match(markdownSource, /function parsePipeCells/);
  assert.match(markdownSource, /function restoreCompactPipeTableRows/);
  assert.match(markdownSource, /function restoreFirstCompactPipeTable/);
  assert.match(markdownSource, /function restoreOrderedListBoundaries/);
  assert.match(markdownSource, /function restoreStructuredMarkupBlock/);
  assert.match(markdownSource, /function mergeCompactTableContinuations/);
  assert.match(markdownSource, /function looksLikeOpenPipeTableHeader/);
  assert.match(markdownSource, /function looksLikePipeTableContinuation/);
  assert.match(markdownSource, /function joinPipeTableContinuation/);
  assert.match(markdownSource, /function normalizeCompactTableSegments/);
  assert.match(markdownSource, /function tableColumnCount/);
  assert.match(markdownSource, /function normalizeSeparatorRow/);
  assert.match(markdownSource, /function looksLikeCompactMarkup/);
  assert.match(markdownSource, /function startsNewMarkdownBlock/);
  assert.match(markdownSource, /isTableRow && previous && !previous\.startsWith\('\\|'\)/);
  assert.match(markdownSource, /!isTableRow && index > 0 && previous && previous\.startsWith\('\\|'\)/);
  assert.match(markdownSource, /rows\.flatMap\(\(row\) => row\.trim\(\)\.startsWith\('\\|'\) \? \[row\] : restoreCompactPipeTableRows\(row\)\)/);
  assert.match(markdownSource, /looksLikeOpenPipeTableHeader\(current\) && looksLikePipeTableContinuation\(next\)/);
  assert.match(markdownSource, /replace\(\/\^\\\|\\s\*\/, ''\)/);
  assert.match(markdownSource, /Math\.max\(headerCellCount, separatorCellCount\)/);
  assert.match(markdownSource, /while \(row\.length < columnCount\) row\.push\('---'\)/);
  assert.ok(markdownSource.includes(".replace(/(?<!\\|)\\s+(#{1,6}\\s+)/g, '\\n$1')"));
  assert.ok(markdownSource.includes("/\\|\\s*:?-{3,}:?\\s*\\|/"));
  assert.doesNotMatch(markdownSource, /codex-wasm-workbench/i);
  assert.doesNotMatch(markdownSource, /weather-app/i);
  assert.doesNotMatch(markdownSource, /LlmCompleted/);
  assert.doesNotMatch(markdownSource, /tool_call/);
});

test("markdown input normalization repairs titled compact pipe tables", async () => {
  const { normalizeMarkdownInput } = await loadMarkdownModule();
  const fileList = normalizeMarkdownInput("预计文件清单 | 文件 | 说明 | 状态 |\n| --- | --- | --- | --- | --- | --- | --- | --- | --- |\nshared/app.py | Flask 主应用 | 待创建 | shared/README.md | 项目文档 | 待创建 |");
  assert.match(fileList, /### 预计文件清单/);
  assert.match(fileList, /\| 文件 \| 说明 \| 状态 \|/);
  assert.match(fileList, /\| shared\/app\.py \| Flask 主应用 \| 待创建 \|/);
  assert.doesNotMatch(fileList, /\| --- \| --- \| --- \| --- \| --- \|/);

  const review = normalizeMarkdownInput("后端 (app.py) | 审查维度 | 评分 |\n| 备注 | --- | --- | \n | --- | 代码结构 | 🟢 优秀 | 清晰的模块拆分 | 错误处理 | 🟢 优秀 | 双数据源容错 |");
  assert.match(review, /### 后端 \(app\.py\)/);
  assert.match(review, /\| 审查维度 \| 评分 \| 备注 \|/);
  assert.match(review, /\| 代码结构 \| 🟢 优秀 \| 清晰的模块拆分 \|/);
  assert.match(review, /\| 错误处理 \| 🟢 优秀 \| 双数据源容错 \|/);

  const inline = normalizeMarkdownInput("### 前端 (index.html) | 审查维度 | 评分 | 备注 | --- | --- | --- | UI 视觉效果 | 🟢 优秀 | 动画过渡 | 用户体验 | 🟢 优秀 | 骨架屏完整 |");
  assert.match(inline, /### 前端 \(index\.html\)/);
  assert.match(inline, /\| 审查维度 \| 评分 \| 备注 \|/);
  assert.match(inline, /\| UI 视觉效果 \| 🟢 优秀 \| 动画过渡 \|/);
  assert.match(inline, /\| 用户体验 \| 🟢 优秀 \| 骨架屏完整 \|/);
});

test("markdown renderer wraps GFM tables in a readable scroll region", () => {
  assert.match(markdownSource, /function MarkdownTable/);
  assert.match(markdownSource, /function MarkdownParagraph/);
  assert.match(markdownSource, /function MarkdownH1/);
  assert.match(markdownSource, /function MarkdownH5/);
  assert.match(markdownSource, /function MarkdownListItem/);
  assert.match(markdownSource, /function MarkdownRawTable/);
  assert.match(markdownSource, /function looksLikeUnparsedPipeTable/);
  assert.match(markdownSource, /function normalizeRawTableText/);
  assert.match(markdownSource, /markdown-table-scroll/);
  assert.match(markdownSource, /markdown-raw-table/);
  assert.match(markdownSource, /markdown-raw-table-item/);
  assert.match(markdownSource, /pipeCount >= 3/);
  assert.match(markdownSource, /\.split\('\\n'\)/);
  assert.match(markdownSource, /\.join\('\\n'\)/);
  assert.match(styleSource, /\.markdown-table-scroll[\s\S]*overflow-x: auto/);
  assert.match(styleSource, /\.markdown-raw-table[\s\S]*overflow: auto/);
  assert.match(styleSource, /\.markdown-raw-table-item[\s\S]*list-style: none/);
  assert.match(styleSource, /\.markdown-raw-table[\s\S]*font-family: ui-monospace/);
  assert.match(styleSource, /\.markdown-body table[\s\S]*min-width: 720px/);
  assert.match(styleSource, /\.markdown-body table[\s\S]*table-layout: auto/);
  assert.match(styleSource, /\.markdown-body table[\s\S]*width: 100%/);
  assert.match(styleSource, /\.markdown-body th:first-child,[\s\S]*white-space: nowrap/);
  assert.match(styleSource, /\.markdown-body th:nth-child\(2\),[\s\S]*min-width: 260px/);
  assert.match(styleSource, /\.markdown-body th[\s\S]*text-transform: uppercase/);
  assert.match(styleSource, /\.markdown-body td code[\s\S]*display: inline-block/);
  assert.match(styleSource, /\.markdown-body td code[\s\S]*overflow-wrap: anywhere/);
  assert.match(styleSource, /\.markdown-body tbody tr:nth-child\(even\) td/);
});

test("markdown renderer preserves readable code blocks with syntax highlighting styles", () => {
  assert.match(styleSource, /\.markdown-body pre[\s\S]*overflow-x: auto/);
  assert.match(styleSource, /\.markdown-body pre[\s\S]*white-space: pre-wrap/);
  assert.match(styleSource, /\.markdown-body pre code[\s\S]*background: transparent/);
  assert.match(styleSource, /\.markdown-body code[\s\S]*color: #d9f7e8/);
});
