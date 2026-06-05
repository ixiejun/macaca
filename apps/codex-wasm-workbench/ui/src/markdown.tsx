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
  if (!text.includes('||') || !/\|\s*:?-{3,}:?\s*\|/.test(text)) return [line];
  const rows = text
    .split(/\s*\|\|\s*/g)
    .map((row) => row.trim())
    .filter(Boolean);
  if (rows.length < 2) return [line];

  const normalizedRows: string[] = [];
  const firstRowCells = splitTableCells(rows[0]);
  const separatorCells = splitTableCells(rows[1]);
  const headingWithTableHeader = rows[0].match(/^(#{1,6}\s+[^|]+?)\s*\|\s*(.+)$/);

  if (headingWithTableHeader) {
    normalizedRows.push(headingWithTableHeader[1].trim());
    normalizedRows.push(formatTableRow(headingWithTableHeader[2]));
  } else if (isSeparatorCells(separatorCells) && firstRowCells.length > separatorCells.length) {
    const headingCellCount = firstRowCells.length - separatorCells.length;
    const heading = firstRowCells.slice(0, headingCellCount).join(' | ');
    normalizedRows.push(formatCompactTableHeading(heading));
    normalizedRows.push(formatTableRow(firstRowCells.slice(headingCellCount).join(' | ')));
  } else {
    normalizedRows.push(formatTableRow(rows[0]));
  }

  normalizedRows.push(...rows.slice(1).map(formatTableRow));
  return normalizedRows;
}

function formatTableRow(row: string): string {
  const cells = splitTableCells(row);
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
