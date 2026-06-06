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
  return expandRecoverablePipeTables([text])
    .flatMap(splitCompactFenceBoundaries)
    .flatMap((part) => (isFormattedTableRow(part) ? [part] : splitCompactHeadingsAndRules(part)));
}

/**
 * Repeatedly expands recoverable pipe tables in one compact provider line.
 *
 * A single assistant response can contain several tables after transport
 * compaction, for example a file list followed by a verification matrix.  The
 * first expansion may leave another compact table in the tail text, so this
 * bounded pass applies the same generic table recovery until the line is stable.
 * The depth limit keeps the adapter deterministic and prevents malformed
 * provider text from causing unbounded UI work.
 */
function expandRecoverablePipeTables(parts: string[], depth = 0): string[] {
  if (depth >= 6) return parts;

  let changed = false;
  const expandedParts = parts.flatMap((part) => {
    if (isFormattedTableRow(part)) return [part];
    const expanded = expandCompactPipeTable(part);
    if (expanded.length !== 1 || expanded[0] !== part) changed = true;
    return expanded;
  });

  return changed ? expandRecoverablePipeTables(expandedParts, depth + 1) : expandedParts;
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
    .replace(/\s+(-\s+)/g, '\n$1')
    .replace(/\s+(\d+\.\s+)/g, '\n$1')
    .split('\n')
    .map((token) => token.trim())
    .filter(Boolean);
  return tokens.length > 0 ? tokens : [line];
}

function expandCompactPipeTable(line: string): string[] {
  const text = String(line || '').trim();
  const hasExplicitCompactDelimiter = text.includes('||');
  const compactText = hasExplicitCompactDelimiter ? normalizeLooseCompactTableDelimiters(text) : text;
  if (!compactText.includes('|') || !/\|\s*:?-{3,}:?\s*\|/.test(compactText)) return [line];
  if (!hasExplicitCompactDelimiter) return expandInlineCompactPipeTable(compactText, line);

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
 * Recovers a GFM table that was flattened into a normal prose line.
 *
 * Some provider streams collapse Markdown line breaks while preserving pipe
 * characters, producing text like `Intro | A | B | |---|---| | 1 | 2 | Tail`.
 * `remark-gfm` cannot infer table rows from that shape.  This adapter uses the
 * separator row as the only structural anchor, treats the cells immediately
 * before it as the header, and groups following cells by the inferred column
 * count.  Any leftover text is returned as normal Markdown so prose after the
 * table remains visible instead of being forced into a malformed row.
 */
function expandInlineCompactPipeTable(markdown: string, fallbackLine: string): string[] {
  const separatorMatch = markdown.match(/\|(?:\s*:?-{3,}:?\s*\|){2,}/);
  if (!separatorMatch || separatorMatch.index === undefined) return [fallbackLine];

  const separatorRow = separatorMatch[0];
  const expectedCellCount = splitTableCells(separatorRow).length;
  if (expectedCellCount < 2) return [fallbackLine];

  const beforeSeparator = markdown.slice(0, separatorMatch.index).trim();
  const afterSeparator = markdown.slice(separatorMatch.index + separatorRow.length).trim();
  const beforeCells = beforeSeparator
    .split('|')
    .map((cell) => cell.trim())
    .filter(Boolean);
  if (beforeCells.length <= expectedCellCount) return [fallbackLine];

  const prefix = beforeCells.slice(0, beforeCells.length - expectedCellCount).join(' | ').trim();
  const header = beforeCells.slice(-expectedCellCount);
  const expandedRows = [
    formatTableRow(header.join(' | '), expectedCellCount),
    formatTableRow(separatorRow, expectedCellCount),
  ];
  const tailCells = splitInlineTableCells(afterSeparator);
  const remainingTail = groupInlineTableCells(tailCells, expectedCellCount, expandedRows);

  return [
    ...splitCompactHeadingsAndRules(prefix),
    ...expandedRows,
    ...splitCompactHeadingsAndRules(remainingTail),
  ].filter((part) => part.trim().length > 0);
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

function splitInlineTableCells(markdown: string): string[] {
  return String(markdown || '')
    .replace(/^\|/, '')
    .split('|')
    .map((cell) => cell.trim())
    .filter(Boolean);
}

function groupInlineTableCells(
  cells: string[],
  expectedCellCount: number,
  expandedRows: string[],
): string {
  const pending = [...cells];
  while (pending.length >= expectedCellCount) {
    expandedRows.push(formatTableRow(pending.splice(0, expectedCellCount).join(' | '), expectedCellCount));
  }
  return pending.join(' | ').trim();
}

function mergeOverflowTableCells(cells: string[], expectedCellCount: number): string[] {
  if (!expectedCellCount || cells.length <= expectedCellCount) return cells;
  const fixedCells = cells.slice(0, expectedCellCount - 1);
  fixedCells.push(cells.slice(expectedCellCount - 1).join(' \\| '));
  return fixedCells;
}
