import type { ComponentProps } from 'react';
import ReactMarkdown from 'react-markdown';
import rehypeHighlight from 'rehype-highlight';
import remarkBreaks from 'remark-breaks';
import remarkGfm from 'remark-gfm';
import 'highlight.js/styles/github-dark.css';

/**
 * Renders assistant-facing markdown with open-source markdown and syntax
 * highlighting libraries instead of a Workbench-owned markdown parser.
 *
 * The Workbench is an application-owned UI surface, so this component acts as a
 * presentation Adapter: it receives already-sanitized execution-event text from
 * the application execution service and delegates Markdown/GFM parsing to
 * `react-markdown`, GFM table parsing to `remark-gfm`, chat-style soft line
 * breaks to `remark-breaks`, and code highlighting to `rehype-highlight`.
 * Keeping parsing out of Macaca OS code preserves the microkernel boundary
 * while still giving the app a richer user experience.
 */
export function MarkdownView({ markdown }: { markdown: string }) {
  return (
    <div className="markdown-body">
      <ReactMarkdown
        components={{
          a: MarkdownLink,
          h1: MarkdownH1,
          h2: MarkdownH2,
          h3: MarkdownH3,
          h4: MarkdownH4,
          h5: MarkdownH5,
          li: MarkdownListItem,
          p: MarkdownParagraph,
          table: MarkdownTable,
        }}
        rehypePlugins={[rehypeHighlight]}
        remarkPlugins={[remarkGfm, remarkBreaks]}
      >
        {normalizeMarkdownInput(markdown)}
      </ReactMarkdown>
    </div>
  );
}

/**
 * Restores common Markdown block boundaries lost by transport/model compaction.
 *
 * This is a syntax-boundary normalizer, not a Workbench event parser: it only
 * looks for generic Markdown tokens so the library pipeline can parse headings,
 * rules, lists, fences, and GFM pipe tables as normal Markdown.
 */
export function normalizeMarkdownInput(markdown: string): string {
  const lines = String(markdown || '')
    .replace(/\r\n/g, '\n')
    .split('\n');
  return repairTitledPipeTables(lines)
    .flatMap((line) => mergeCompactTableContinuations([line]))
    .flatMap(normalizeMarkdownLine)
    .join('\n')
    .replace(/\n{3,}/g, '\n\n')
    .trim();
}

function repairTitledPipeTables(lines: string[]): string[] {
  const repaired: string[] = [];
  for (let index = 0; index < lines.length; index += 1) {
    const repair = repairTitledPipeTableAt(lines, index);
    if (repair) {
      repaired.push(...repair.lines);
      index = repair.nextIndex - 1;
    } else {
      repaired.push(lines[index] || '');
    }
  }
  return repaired;
}

function repairTitledPipeTableAt(lines: string[], startIndex: number): { lines: string[]; nextIndex: number } | null {
  const current = lines[startIndex] || '';
  const next = lines[startIndex + 1] || '';
  if (!looksLikeTitledPipeHeader(current) || !/\|\s*:?-{3,}:?\s*\|/.test(next)) return null;

  const titleAndHeaders = parsePipeCells(current);
  if (titleAndHeaders.length < 2) return null;
  const title = titleAndHeaders[0];
  const headers = titleAndHeaders.slice(1);
  const nextCells = parsePipeCells(next);
  const separatorIndex = nextCells.findIndex(isTableSeparatorCell);
  if (separatorIndex < 0) return null;

  const additionalHeaders = nextCells.slice(0, separatorIndex).filter((cell) => !isTableSeparatorCell(cell));
  const allHeaders = [...headers, ...additionalHeaders].filter(Boolean);
  if (allHeaders.length < 2) return null;
  const dataFromNext = nextCells.slice(separatorIndex).filter((cell) => !isTableSeparatorCell(cell));
  const collected = collectCompactTableData(lines, startIndex + 2, allHeaders.length);
  const dataCells = [...dataFromNext, ...collected.cells];
  const tableRows = chunkTableRows(dataCells, allHeaders.length);
  if (!tableRows.length) return null;

  return {
    lines: [
      `### ${title}`,
      markdownTableRow(allHeaders),
      markdownTableRow(allHeaders.map(() => '---')),
      ...tableRows.map(markdownTableRow),
    ],
    nextIndex: collected.nextIndex,
  };
}

function repairInlineTitledPipeTable(line: string): string[] | null {
  const cells = parsePipeCells(line);
  const firstSeparator = cells.findIndex(isTableSeparatorCell);
  if (firstSeparator <= 1) return null;
  const title = stripHeadingMarker(cells[0]);
  const headers = cells.slice(1, firstSeparator).filter((cell) => !isTableSeparatorCell(cell));
  if (headers.length < 2) return null;
  const dataStart = firstSeparator + countSeparatorCells(cells, firstSeparator);
  const rows = chunkTableRows(cells.slice(dataStart), headers.length);
  if (!rows.length) return null;
  return [
    `### ${title}`,
    markdownTableRow(headers),
    markdownTableRow(headers.map(() => '---')),
    ...rows.map(markdownTableRow),
  ];
}

function collectCompactTableData(lines: string[], startIndex: number, columnCount: number): { cells: string[]; nextIndex: number } {
  const cells: string[] = [];
  let index = startIndex;
  while (index < lines.length) {
    const line = lines[index] || '';
    if (!line.trim()) break;
    if (!line.includes('|')) break;
    const parsed = parsePipeCells(line).filter((cell) => !isTableSeparatorCell(cell));
    if (parsed.length === 0) break;
    const enoughForRow = cells.length + parsed.length >= columnCount;
    if (!enoughForRow && startsNewMarkdownBlock(parsed[0] || '')) break;
    cells.push(...parsed);
    index += 1;
  }
  return { cells, nextIndex: index };
}

function chunkTableRows(cells: string[], columnCount: number): string[][] {
  const rows: string[][] = [];
  for (let index = 0; index + columnCount <= cells.length; index += columnCount) {
    const row = cells.slice(index, index + columnCount);
    if (row.some(startsNewMarkdownBlock)) break;
    rows.push(row);
  }
  return rows;
}

function looksLikeTitledPipeHeader(line: string): boolean {
  const text = line.trim();
  return !text.startsWith('|')
    && text.endsWith('|')
    && parsePipeCells(text).length >= 2
    && !/\|\s*:?-{3,}:?\s*\|/.test(text);
}

function parsePipeCells(line: string): string[] {
  return String(line || '')
    .split('|')
    .map((cell) => cell.trim())
    .filter(Boolean);
}

function stripHeadingMarker(value: string): string {
  return value.replace(/^#{1,6}\s+/, '').trim();
}

function MarkdownLink({ children, href, ...props }: ComponentProps<'a'>) {
  return (
    <a href={href} rel="noreferrer" target="_blank" {...props}>
      {children}
    </a>
  );
}

function MarkdownParagraph({ children, ...props }: ComponentProps<'p'>) {
  const text = plainText(children);
  if (looksLikeUnparsedPipeTable(text)) {
    return <MarkdownRawTable text={text} />;
  }
  return <p {...props}>{children}</p>;
}

function MarkdownH1({ children, ...props }: ComponentProps<'h1'>) {
  const text = plainText(children);
  if (looksLikeUnparsedPipeTable(text)) return <MarkdownRawTable text={text} />;
  return <h1 {...props}>{children}</h1>;
}

function MarkdownH2({ children, ...props }: ComponentProps<'h2'>) {
  const text = plainText(children);
  if (looksLikeUnparsedPipeTable(text)) return <MarkdownRawTable text={text} />;
  return <h2 {...props}>{children}</h2>;
}

function MarkdownH3({ children, ...props }: ComponentProps<'h3'>) {
  const text = plainText(children);
  if (looksLikeUnparsedPipeTable(text)) return <MarkdownRawTable text={text} />;
  return <h3 {...props}>{children}</h3>;
}

function MarkdownH4({ children, ...props }: ComponentProps<'h4'>) {
  const text = plainText(children);
  if (looksLikeUnparsedPipeTable(text)) return <MarkdownRawTable text={text} />;
  return <h4 {...props}>{children}</h4>;
}

function MarkdownH5({ children, ...props }: ComponentProps<'h5'>) {
  const text = plainText(children);
  if (looksLikeUnparsedPipeTable(text)) return <MarkdownRawTable text={text} />;
  return <h5 {...props}>{children}</h5>;
}

function MarkdownListItem({ children, ...props }: ComponentProps<'li'>) {
  const text = plainText(children);
  if (looksLikeUnparsedPipeTable(text)) return <li className="markdown-raw-table-item" {...props}><MarkdownRawTable text={text} /></li>;
  return <li {...props}>{children}</li>;
}

function MarkdownRawTable({ text }: { text: string }) {
  return <pre className="markdown-raw-table" role="note">{normalizeRawTableText(text)}</pre>;
}

function MarkdownTable({ children, ...props }: ComponentProps<'table'>) {
  return (
    <div className="markdown-table-scroll" role="region" aria-label="Markdown table">
      <table {...props}>{children}</table>
    </div>
  );
}

function plainText(value: unknown): string {
  if (typeof value === 'string' || typeof value === 'number') return String(value);
  if (Array.isArray(value)) return value.map(plainText).join('');
  if (value && typeof value === 'object' && 'props' in value) {
    const element = value as { props?: { children?: unknown } };
    return plainText(element.props?.children);
  }
  return '';
}

function looksLikeUnparsedPipeTable(text: string): boolean {
  const value = String(text || '').trim();
  const pipeCount = (value.match(/\|/g) || []).length;
  return pipeCount >= 3
    && (/\|\s*:?-{3,}:?\s*\|/.test(value) || /(^|\n)\s*\|/.test(value) || /\|\s*$/.test(value));
}

function normalizeRawTableText(text: string): string {
  return String(text || '')
    .split('\n')
    .map((line) => line.trim().replace(/\s*\|\s*/g, ' | ').replace(/ {2,}/g, ' '))
    .join('\n')
    .trim();
}

function normalizeMarkdownLine(line: string): string[] {
  const text = String(line || '');
  if (!text.trim()) return [text];
  const structuredMarkupBlock = restoreStructuredMarkupBlock(text);
  if (structuredMarkupBlock) return structuredMarkupBlock;
  const inlineTitledTable = repairInlineTitledPipeTable(text);
  if (inlineTitledTable) return inlineTitledTable;
  const compactTableRows = restoreCompactPipeTableRows(text);
  if (compactTableRows.length > 1) {
    return normalizeCompactTableSegments(compactTableRows);
  }
  return restoreMarkdownBlockBoundaries(text).flatMap(restoreOrderedListBoundaries);
}

function mergeCompactTableContinuations(lines: string[]): string[] {
  const merged: string[] = [];
  for (let index = 0; index < lines.length; index += 1) {
    const current = lines[index] || '';
    const next = lines[index + 1] || '';
    if (looksLikeOpenPipeTableHeader(current) && looksLikePipeTableContinuation(next)) {
      merged.push(joinPipeTableContinuation(current, next));
      index += 1;
    } else {
      merged.push(current);
    }
  }
  return merged;
}

function restoreStructuredMarkupBlock(line: string): string[] | null {
  const text = line.trim();
  if (!looksLikeCompactMarkup(text)) return null;
  return ['```xml', text, '```'];
}

function restoreMarkdownBlockBoundaries(line: string): string[] {
  return line
    .replace(/\s+(---)(?=\s+#{1,6}\s|\s*$)/g, '\n$1\n')
    .replace(/(?<!\|)\s+(#{1,6}\s+)/g, '\n$1')
    .replace(/\s+(```[a-zA-Z0-9_-]*)(?=\s|$)/g, '\n$1\n')
    .replace(/\s+(-\s+\[[ xX]\]\s+)/g, '\n$1')
    .replace(/\s+(-\s+)(?=\S)/g, '\n$1')
    .split('\n')
    .map((part) => part.trim())
    .filter(Boolean);
}

function restoreOrderedListBoundaries(line: string): string[] {
  return line
    .replace(/\s+(\d+\.\s+)(?=\S)/g, '\n$1')
    .split('\n')
    .map((part) => part.trim())
    .filter(Boolean);
}

function restoreCompactPipeTableRows(line: string): string[] {
  const rows = restoreFirstCompactPipeTable(line);
  if (rows.length <= 1) return rows;
  return rows.flatMap((row) => row.trim().startsWith('|') ? [row] : restoreCompactPipeTableRows(row));
}

function restoreFirstCompactPipeTable(line: string): string[] {
  if (!/\|\s*:?-{3,}:?\s*\|/.test(line)) return [line];
  const firstPipe = line.indexOf('|');
  if (firstPipe < 0) return [line];
  const prefix = line.slice(0, firstPipe).trim();
  const cells = line.slice(firstPipe)
    .split('|')
    .map((row) => row.trim())
    .filter(Boolean);
  const firstSeparator = cells.findIndex(isTableSeparatorCell);
  if (firstSeparator <= 0) return [line];
  const separatorCount = countSeparatorCells(cells, firstSeparator);
  const columnCount = tableColumnCount(firstSeparator, separatorCount);
  if (columnCount <= 0 || firstSeparator < separatorCount) return [line];

  const rows: string[] = [];
  const header = cells.slice(firstSeparator - columnCount, firstSeparator);
  rows.push(markdownTableRow(header));
  rows.push(markdownTableRow(normalizeSeparatorRow(cells.slice(firstSeparator, firstSeparator + separatorCount), columnCount)));

  let cursor = firstSeparator + separatorCount;
  while (cursor + columnCount <= cells.length) {
    const row = cells.slice(cursor, cursor + columnCount);
    if (row.some(startsNewMarkdownBlock)) break;
    rows.push(markdownTableRow(row));
    cursor += columnCount;
  }

  const remainder = cells.slice(cursor).join(' | ').trim();
  return [prefix, ...rows, remainder].filter(Boolean);
}

function normalizeCompactTableSegments(rows: string[]): string[] {
  const normalized: string[] = [];
  rows.forEach((row, index) => {
    const isTableRow = row.trim().startsWith('|');
    const previous = normalized[normalized.length - 1] || '';
    if (isTableRow && previous && !previous.startsWith('|')) normalized.push('');
    if (!isTableRow && index > 0 && previous && previous.startsWith('|')) normalized.push('');
    normalized.push(...restoreMarkdownBlockBoundaries(row).flatMap(restoreOrderedListBoundaries));
  });
  return normalized;
}

function isTableSeparatorCell(cell: string): boolean {
  return /^:?-{3,}:?$/.test(cell.trim());
}

function looksLikeOpenPipeTableHeader(line: string): boolean {
  const text = line.trim();
  return text.includes('|') && !/\|\s*:?-{3,}:?\s*\|/.test(text);
}

function looksLikePipeTableContinuation(line: string): boolean {
  const text = line.trim();
  return text.startsWith('|') && /\|\s*:?-{3,}:?\s*\|/.test(text);
}

function joinPipeTableContinuation(current: string, next: string): string {
  const left = current.trim();
  const right = next.trim().replace(/^\|\s*/, '');
  return `${left} ${right}`;
}

function countSeparatorCells(cells: string[], start: number): number {
  let count = 0;
  for (let index = start; index < cells.length && isTableSeparatorCell(cells[index]); index += 1) {
    count += 1;
  }
  return count;
}

function tableColumnCount(headerCellCount: number, separatorCellCount: number): number {
  return Math.max(headerCellCount, separatorCellCount);
}

function normalizeSeparatorRow(cells: string[], columnCount: number): string[] {
  const row = [...cells];
  while (row.length < columnCount) row.push('---');
  return row.slice(0, columnCount);
}

function startsNewMarkdownBlock(cell: string): boolean {
  return /^(#{1,6}\s|`{3,}|-{3,}\s+#{1,6}\s|\d+\.\s|- \S)/.test(cell.trim());
}

function looksLikeCompactMarkup(text: string): boolean {
  return text.length > 80
    && /<[A-Za-z][A-Za-z0-9:_-]*(\s|>|\/>)/.test(text)
    && /<\/[A-Za-z][A-Za-z0-9:_-]*>/.test(text)
    && (text.match(/<[/?][A-Za-z][A-Za-z0-9:_-]*/g)?.length || 0) >= 4;
}

function markdownTableRow(cells: string[]): string {
  return `| ${cells.join(' | ')} |`;
}
