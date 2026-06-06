import type { ComponentProps } from 'react';
import ReactMarkdown from 'react-markdown';
import rehypeHighlight from 'rehype-highlight';
import remarkGfm from 'remark-gfm';
import 'highlight.js/styles/github-dark.css';

/**
 * Renders assistant-facing markdown with open-source markdown and syntax
 * highlighting libraries instead of a Workbench-owned markdown parser.
 *
 * The Workbench is an application-owned UI surface, so this component acts as a
 * presentation Adapter: it receives already-sanitized execution-event text from
 * the application execution service and delegates Markdown/GFM parsing to
 * `react-markdown`, GFM table parsing to `remark-gfm`, and code highlighting to
 * `rehype-highlight`. Keeping parsing out of Macaca OS code preserves the
 * microkernel boundary while still giving the app a richer user experience.
 */
export function MarkdownView({ markdown }: { markdown: string }) {
  return (
    <div className="markdown-body">
      <ReactMarkdown
        components={{
          a: MarkdownLink,
          table: MarkdownTable,
        }}
        rehypePlugins={[rehypeHighlight]}
        remarkPlugins={[remarkGfm]}
      >
        {normalizeWorkbenchMarkdown(markdown)}
      </ReactMarkdown>
    </div>
  );
}

/**
 * Repairs transport/model formatting artifacts before the content reaches the
 * library renderer.
 *
 * This is deliberately not a Markdown parser. It only inserts missing line
 * breaks around structural tokens that large language models frequently emit as
 * one long line inside execution events. Rendering semantics remain owned by
 * the open-source Markdown pipeline above, which keeps this app UI flexible and
 * avoids hardcoded application-business behavior.
 */
export function normalizeWorkbenchMarkdown(markdown: string): string {
  const source = String(markdown || '').replace(/\r\n/g, '\n');
  return source
    .split('\n')
    .flatMap(splitCompactMarkdownLine)
    .join('\n')
    .replace(/\n{3,}/g, '\n\n')
    .trim();
}

function MarkdownLink({ children, href, ...props }: ComponentProps<'a'>) {
  return (
    <a href={href} rel="noreferrer" target="_blank" {...props}>
      {children}
    </a>
  );
}

function MarkdownTable({ children, ...props }: ComponentProps<'table'>) {
  return (
    <div className="markdown-table-scroll" role="region" aria-label="Markdown table">
      <table {...props}>{children}</table>
    </div>
  );
}

function splitCompactMarkdownLine(line: string): string[] {
  const text = String(line || '');
  if (!text.trim()) return [text];
  return expandCompactPipeTable(text)
    .flatMap(splitCompactFenceBoundaries)
    .flatMap((part) => (isFormattedTableRow(part) ? [part] : splitCompactHeadingsAndRules(part)));
}

function splitCompactFenceBoundaries(line: string): string[] {
  if (!line.includes('```')) return [line];
  const tokens: string[] = [];
  let rest = line;
  while (rest.includes('```')) {
    const fenceIndex = rest.indexOf('```');
    const before = rest.slice(0, fenceIndex).trim();
    if (before) tokens.push(before);
    tokens.push('```');
    rest = rest.slice(fenceIndex + 3).trim();
  }
  if (rest) tokens.push(rest);
  return tokens;
}

function splitCompactHeadingsAndRules(line: string): string[] {
  const text = String(line || '').trim();
  if (!text) return [line];
  const tokens = text
    .replace(/\s+(---)\s+/g, '\n$1\n')
    .replace(/\s+(#{1,6}\s+)/g, '\n$1')
    .split('\n')
    .map((token) => token.trim())
    .filter(Boolean);
  return tokens.length > 0 ? tokens : [line];
}

function expandCompactPipeTable(line: string): string[] {
  const text = String(line || '').trim();
  const compactText = normalizeLooseCompactTableDelimiters(text);
  if (!compactText.includes('|') || !/\|\s*:?-{3,}:?\s*\|/.test(compactText)) return [line];
  const rows = compactText
    .split(/\s*\|\|\s*/g)
    .map((row) => row.trim())
    .filter(Boolean);
  if (rows.length < 2) return [line];

  const normalizedRows: string[] = [];
  const firstRowCells = splitTableCells(rows[0]);
  const separatorCells = splitTableCells(rows[1]);
  const expectedCellCount = isSeparatorCells(separatorCells) ? separatorCells.length : 0;
  const headingWithTableHeader = rows[0].match(/^(#{1,6}\s+[^|]+?)\s*\|\s*(.+)$/);

  if (headingWithTableHeader) {
    normalizedRows.push(headingWithTableHeader[1].trim());
    normalizedRows.push(formatTableRow(headingWithTableHeader[2], expectedCellCount));
  } else if (expectedCellCount > 0 && firstRowCells.length > expectedCellCount) {
    const headingCellCount = firstRowCells.length - expectedCellCount;
    const heading = firstRowCells.slice(0, headingCellCount).join(' | ');
    normalizedRows.push(formatCompactTableHeading(heading));
    normalizedRows.push(formatTableRow(firstRowCells.slice(headingCellCount).join(' | '), expectedCellCount));
  } else {
    normalizedRows.push(formatTableRow(rows[0], expectedCellCount));
  }

  normalizedRows.push(...expandCompactTableRows(rows.slice(1), expectedCellCount));
  return normalizedRows;
}

function formatTableRow(row: string, expectedCellCount = 0): string {
  const cells = mergeOverflowTableCells(splitTableCells(row), expectedCellCount);
  if (cells.length === 0) return '| |';
  if (isSeparatorCells(cells)) {
    return `| ${cells.map(() => '---').join(' | ')} |`;
  }
  return `| ${cells.join(' | ')} |`;
}

function isFormattedTableRow(row: string): boolean {
  return /^\|\s*.+\s*\|$/.test(String(row || '').trim());
}

function splitTableCells(row: string): string[] {
  return String(row || '')
    .replace(/^\|/, '')
    .replace(/\|$/, '')
    .split('|')
    .map((cell) => cell.trim())
    .filter(Boolean);
}

function isSeparatorCells(cells: string[]): boolean {
  return cells.length > 0 && cells.every((cell) => /^:?-{3,}:?$/.test(cell));
}

function formatCompactTableHeading(heading: string): string {
  const text = String(heading || '').replace(/^[-*]\s+/, '').trim();
  if (!text) return '';
  return /^#{1,6}\s+/.test(text) ? text : `### ${text}`;
}

function normalizeLooseCompactTableDelimiters(markdown: string): string {
  return String(markdown || '').replace(/\|\s+\|(?=\s*(?:\*\*|:?-{3,}:?))/g, '||');
}

/**
 * Expands table rows that were compacted into one transport line.
 *
 * Model output often reaches the Workbench as a single line such as
 * `| A | B || | --- | --- | 1 | x | 2 | y |`.  `remark-gfm` can render the
 * table correctly only after those cells become separate physical Markdown
 * rows.  This helper uses the separator row to infer the stable column count,
 * then groups subsequent cells into rows of that width.  It does not inspect
 * domain terms, file names, tools, or application-specific business content;
 * it is a generic presentation adapter for malformed-but-recoverable GFM.
 */
function expandCompactTableRows(rows: string[], expectedCellCount: number): string[] {
  if (!expectedCellCount) return rows.map((row) => formatTableRow(row, expectedCellCount));

  const expandedRows: string[] = [];
  const pendingCells: string[] = [];

  for (const row of rows) {
    const cells = splitTableCells(row);
    if (cells.length === 0) continue;

    if (isSeparatorCells(cells)) {
      flushPendingCompactCells(pendingCells, expectedCellCount, expandedRows);
      expandedRows.push(formatTableRow(cells.join(' | '), expectedCellCount));
      continue;
    }

    pendingCells.push(...cells);
    flushPendingCompactCells(pendingCells, expectedCellCount, expandedRows);
  }

  flushPendingCompactCells(pendingCells, expectedCellCount, expandedRows, true);
  return expandedRows;
}

/**
 * Moves accumulated compact table cells into normalized Markdown rows.
 *
 * The function mutates `pendingCells` intentionally so callers can stream cells
 * from several compact fragments without allocating intermediate row objects.
 * Remainder cells are emitted only at the end; that preserves partial data for
 * user visibility while preventing incomplete fragments from shifting the
 * following rows during normal processing.
 */
function flushPendingCompactCells(
  pendingCells: string[],
  expectedCellCount: number,
  expandedRows: string[],
  flushRemainder = false,
) {
  while (pendingCells.length >= expectedCellCount) {
    expandedRows.push(formatTableRow(pendingCells.splice(0, expectedCellCount).join(' | '), expectedCellCount));
  }

  if (flushRemainder && pendingCells.length > 0) {
    expandedRows.push(formatTableRow(pendingCells.splice(0).join(' | '), expectedCellCount));
  }
}

function mergeOverflowTableCells(cells: string[], expectedCellCount: number): string[] {
  if (!expectedCellCount || cells.length <= expectedCellCount) return cells;
  const fixedCells = cells.slice(0, expectedCellCount - 1);
  fixedCells.push(cells.slice(expectedCellCount - 1).join(' \\| '));
  return fixedCells;
}
