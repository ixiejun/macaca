import type { ReactNode } from 'react';

type MarkdownBlock =
  | { kind: 'paragraph'; text: string }
  | { kind: 'heading'; level: 3 | 4 | 5; text: string }
  | { kind: 'list'; ordered: boolean; items: string[] }
  | { kind: 'code'; language: string; text: string }
  | { kind: 'rule' }
  | { kind: 'table'; header: string[]; rows: string[][] };

/**
 * Render sanitized markdown-like model output as React elements.
 *
 * This parser intentionally supports only the display features needed by the
 * Workbench: headings, lists, fenced code, compact tables, links, bold text,
 * and inline code.  React escapes all text nodes, so model output can be shown
 * without granting it HTML execution authority.
 */
export function MarkdownView({ markdown }: { markdown: string }) {
  return (
    <div className="markdown-body">
      {parseMarkdown(markdown).map((block, index) => renderBlock(block, index))}
    </div>
  );
}

export function parseMarkdown(markdown: string): MarkdownBlock[] {
  const blocks: MarkdownBlock[] = [];
  const lines = normalizeMarkdownLines(markdown);
  let paragraph: string[] = [];
  let list: { ordered: boolean; items: string[] } | null = null;
  let codeLines: string[] | null = null;
  let codeLanguage = '';

  const flushParagraph = () => {
    if (paragraph.length > 0) blocks.push({ kind: 'paragraph', text: paragraph.join('\n') });
    paragraph = [];
  };
  const flushList = () => {
    if (list) blocks.push({ kind: 'list', ordered: list.ordered, items: list.items });
    list = null;
  };

  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    const fence = line.match(/^```(.*)$/);
    if (fence) {
      if (codeLines) {
        blocks.push({ kind: 'code', language: codeLanguage, text: codeLines.join('\n') });
        codeLines = null;
        codeLanguage = '';
      } else {
        flushParagraph();
        flushList();
        codeLines = [];
        codeLanguage = fence[1].trim();
      }
      continue;
    }
    if (codeLines) {
      codeLines.push(line);
      continue;
    }
    if (isMarkdownRule(line)) {
      flushParagraph();
      flushList();
      blocks.push({ kind: 'rule' });
      continue;
    }
    if (isMarkdownTableStart(lines, index)) {
      flushParagraph();
      flushList();
      const { table, nextIndex } = parseMarkdownTable(lines, index);
      blocks.push(table);
      index = nextIndex - 1;
      continue;
    }
    if (!line.trim()) {
      flushParagraph();
      flushList();
      continue;
    }
    const heading = line.match(/^(#{1,3})\s+(.+)$/);
    if (heading) {
      flushParagraph();
      flushList();
      blocks.push({ kind: 'heading', level: (String(heading[1]).length + 2) as 3 | 4 | 5, text: heading[2] });
      continue;
    }
    const bullet = line.match(/^\s*[-*]\s+(.+)$/);
    if (bullet) {
      flushParagraph();
      if (!list || list.ordered) {
        flushList();
        list = { ordered: false, items: [] };
      }
      list.items.push(bullet[1]);
      continue;
    }
    const ordered = line.match(/^\s*\d+\.\s+(.+)$/);
    if (ordered) {
      flushParagraph();
      if (!list || !list.ordered) {
        flushList();
        list = { ordered: true, items: [] };
      }
      list.items.push(ordered[1]);
      continue;
    }
    paragraph.push(line.trim());
  }
  flushParagraph();
  flushList();
  if (codeLines) blocks.push({ kind: 'code', language: codeLanguage, text: codeLines.join('\n') });
  return blocks;
}

function renderBlock(block: MarkdownBlock, index: number): ReactNode {
  if (block.kind === 'paragraph') return <p key={index}>{renderInline(block.text)}</p>;
  if (block.kind === 'heading') {
    if (block.level === 3) return <h3 key={index}>{renderInline(block.text)}</h3>;
    if (block.level === 4) return <h4 key={index}>{renderInline(block.text)}</h4>;
    return <h5 key={index}>{renderInline(block.text)}</h5>;
  }
  if (block.kind === 'list') {
    const List = block.ordered ? 'ol' : 'ul';
    return <List key={index}>{block.items.map((item, itemIndex) => <li key={itemIndex}>{renderInline(item)}</li>)}</List>;
  }
  if (block.kind === 'code') {
    return <pre key={index}><code data-language={block.language || undefined}>{block.text}</code></pre>;
  }
  if (block.kind === 'rule') return <hr key={index} />;
  return (
    <table key={index}>
      <thead><tr>{block.header.map((cell, cellIndex) => <th key={cellIndex}>{renderInline(cell)}</th>)}</tr></thead>
      <tbody>{block.rows.map((row, rowIndex) => <tr key={rowIndex}>{row.map((cell, cellIndex) => <td key={cellIndex}>{renderInline(cell)}</td>)}</tr>)}</tbody>
    </table>
  );
}

function renderInline(text: string): ReactNode[] {
  const nodes: ReactNode[] = [];
  const pattern = /(`[^`]+`|\*\*[^*]+\*\*|\[[^\]]+\]\([^)]+\))/g;
  let offset = 0;
  for (const match of text.matchAll(pattern)) {
    if (match.index > offset) nodes.push(text.slice(offset, match.index));
    const token = match[0];
    if (token.startsWith('`')) nodes.push(<code key={nodes.length}>{token.slice(1, -1)}</code>);
    else if (token.startsWith('**')) nodes.push(<strong key={nodes.length}>{token.slice(2, -2)}</strong>);
    else {
      const link = token.match(/^\[([^\]]+)\]\(([^)]+)\)$/);
      nodes.push(<a key={nodes.length} href={link?.[2] || '#'} rel="noreferrer" target="_blank">{link?.[1] || token}</a>);
    }
    offset = match.index + token.length;
  }
  if (offset < text.length) nodes.push(text.slice(offset));
  return nodes;
}

function normalizeMarkdownLines(markdown: string): string[] {
  const normalized: string[] = [];
  let inFence = false;
  for (const rawLine of String(markdown || '').split(/\r?\n/)) {
    let line = rawLine;
    while (line.includes('```')) {
      const fenceIndex = line.indexOf('```');
      const before = line.slice(0, fenceIndex).trimEnd();
      if (before) normalized.push(...(inFence ? [before] : splitInlineMarkdownStructure(before)));
      const after = line.slice(fenceIndex + 3).trimStart();
      if (inFence) {
        normalized.push('```');
        inFence = false;
        line = after;
        continue;
      }
      const languageOnly = after && /^[A-Za-z0-9_-]+$/.test(after);
      normalized.push(languageOnly ? `\`\`\`${after}` : '```');
      inFence = true;
      line = !after || languageOnly ? '' : splitCompactFenceTail(after);
    }
    normalized.push(...(inFence ? [line] : splitInlineMarkdownStructure(line)));
  }
  return normalized;
}

function splitInlineMarkdownStructure(line: string): string[] {
  return expandCompactPipeTable(line).flatMap(splitInlineRulesAndHeadings).flatMap(splitCompactFieldRows);
}

function splitInlineRulesAndHeadings(line: string): string[] {
  const text = String(line || '');
  if (!text.trim()) return [text];
  const tokens: string[] = [];
  const pattern = /(?:^|\s)(---|\#{1,3}\s+[\s\S]+?)(?=(?:\s---|\s#{1,3}\s+|\s```|$))/g;
  let offset = 0;
  for (const match of text.matchAll(pattern)) {
    const tokenStart = match.index + (match[0].startsWith(' ') ? 1 : 0);
    const before = text.slice(offset, tokenStart).trim();
    if (before) tokens.push(before);
    tokens.push(match[1].trim());
    offset = tokenStart + match[1].length;
  }
  const after = text.slice(offset).trim();
  if (after) tokens.push(after);
  return tokens.length > 0 ? tokens : [line];
}

function splitCompactFenceTail(afterFence: string): string {
  return afterFence.replace(/\s+(---|#{1,3}\s+)/g, '\n$1');
}

function splitCompactFieldRows(line: string): string[] {
  const text = String(line || '').trim();
  if (!text.includes(' | ') || !/(\*\*[^*]+\*\*|[|｜-]{3,})/.test(text)) return [line];
  const parts = text.split(/\s+\|\s+/g).map((part) => part.trim()).filter(Boolean);
  if (parts.length < 3) return [line];
  return parts.map((part) => (/^\|/.test(part) ? part : `- ${part}`));
}

function expandCompactPipeTable(line: string): string[] {
  const text = String(line || '').trim();
  if (!text.includes('||') || !/\|\s*-{3,}/.test(text)) return [line];
  const rows = text.split(/\s*\|\|\s*/g).map((row) => row.trim()).filter(Boolean);
  return rows.length >= 2 ? rows.map((row) => `${row.startsWith('|') ? '' : '| '}${row}${row.endsWith('|') ? '' : ' |'}`) : [line];
}

function isMarkdownRule(line: string): boolean {
  return /^\s*(-{3,}|\*{3,}|_{3,})\s*$/.test(line);
}

function isMarkdownTableStart(lines: string[], index: number): boolean {
  return isTableRow(lines[index]) && isTableSeparator(lines[index + 1] || '');
}

function isTableRow(line: string): boolean {
  return /^\s*\|.*\|\s*$/.test(line || '');
}

function isTableSeparator(line: string): boolean {
  return /^\s*\|?(\s*:?-{3,}:?\s*\|)+\s*:?-{3,}:?\s*\|?\s*$/.test(line || '');
}

function parseMarkdownTable(lines: string[], startIndex: number): { table: MarkdownBlock; nextIndex: number } {
  const header = splitTableCells(lines[startIndex]);
  const rows: string[][] = [];
  let index = startIndex + 2;
  while (index < lines.length && isTableRow(lines[index])) {
    rows.push(splitTableCells(lines[index]));
    index += 1;
  }
  return { table: { kind: 'table', header, rows }, nextIndex: index };
}

function splitTableCells(line: string): string[] {
  return String(line || '').trim().replace(/^\|/, '').replace(/\|$/, '').split('|').map((cell) => cell.trim());
}
