/**
 * Normalize model-produced Markdown before it is delegated to react-markdown.
 *
 * The Workbench receives assistant output through application-execution events.
 * Provider transports may compact Markdown into a single physical line while
 * preserving headings, fences, list markers, and GFM table pipes.  This module
 * is a presentation Adapter: it repairs generic Markdown transport artifacts
 * without inspecting application names, business domains, or file semantics.
 */
export function normalizeWorkbenchMarkdown(markdown) {
  const source = String(markdown || '').replace(/\r\n/g, '\n');
  return source
    .split('\n')
    .flatMap(splitCompactMarkdownLine)
    .join('\n')
    .replace(/\n{3,}/g, '\n\n')
    .trim();
}

function splitCompactMarkdownLine(line) {
  const text = String(line || '');
  if (!text.trim()) return [text];
  return expandRecoverablePipeTables([text])
    .flatMap(splitCompactFenceBoundaries)
    .flatMap((part) => (isFormattedTableRow(part) ? [part] : splitCompactHeadingsAndRules(part)));
}

/**
 * Repeatedly expands recoverable inline pipe tables in a compact provider line.
 *
 * A single assistant response can contain several GFM tables after transport
 * compaction.  The bounded loop makes recovery deterministic and auditable:
 * every pass must reveal at least one new table row, and malformed text falls
 * back to normal Markdown instead of blocking the UI.
 */
function expandRecoverablePipeTables(parts, depth = 0) {
  if (depth >= 8) return parts;

  let changed = false;
  const expandedParts = parts.flatMap((part) => {
    if (isFormattedTableRow(part)) return [part];
    const expanded = expandCompactPipeTable(part);
    if (expanded.length !== 1 || expanded[0] !== part) changed = true;
    return expanded;
  });

  return changed ? expandRecoverablePipeTables(expandedParts, depth + 1) : expandedParts;
}

function splitCompactFenceBoundaries(line) {
  if (!line.includes('```')) return [line];
  const tokens = [];
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

function splitCompactHeadingsAndRules(line) {
  const text = String(line || '').trim();
  if (!text) return [line];
  const structuralTokens = text
    .replace(/\s+(---)\s+/g, '\n$1\n')
    .replace(/\s+(#{1,6}\s+)/g, '\n$1')
    .split('\n')
    .map((token) => token.trim())
    .filter(Boolean);

  const tokens = structuralTokens.flatMap((token) => {
    /*
     * Keep numbered text inside headings intact.
     *
     * Assistant review reports often compact section titles as
     * `### ✅ 1. 计划对齐 ...`.  Treating the `1.` as an ordered-list marker
     * wraps the following recovered table in a list item, which prevents
     * remark-gfm from recognizing it as a block-level table.  Headings are
     * already structural blocks, so nested list recovery is intentionally
     * skipped for them.
     */
    if (/^#{1,6}\s+/.test(token)) return [token];
    return token
      .replace(/\s+(-\s+)/g, '\n$1')
      .replace(/\s+(\d+\.\s+)/g, '\n$1')
      .split('\n')
      .map((part) => part.trim())
      .filter(Boolean);
  });
  return tokens.length > 0 ? tokens : [line];
}

function expandCompactPipeTable(line) {
  const text = String(line || '').trim();
  const hasExplicitCompactDelimiter = text.includes('||');
  const compactText = hasExplicitCompactDelimiter ? normalizeLooseCompactTableDelimiters(text) : text;
  if (!compactText.includes('|') || !hasTableSeparator(compactText)) return [line];
  if (!hasExplicitCompactDelimiter) return expandInlineCompactPipeTable(compactText, line);

  const rows = compactText
    .split(/\s*\|\|\s*/g)
    .map((row) => row.trim())
    .filter(Boolean);
  if (rows.length < 2) return expandInlineCompactPipeTable(compactText, line);

  const normalizedRows = [];
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

function hasTableSeparator(markdown) {
  return findSeparatorMatch(markdown) !== null;
}

/**
 * Find a compact GFM separator row.
 *
 * Transport-compacted rows often appear as either `|---|---|` or
 * `| |---|---| |`, where the extra empty cell is the closing pipe of the
 * previous row followed by the opening pipe of the separator row.  This matcher
 * intentionally keys only on Markdown separator syntax so it remains generic.
 */
function findSeparatorMatch(markdown) {
  return String(markdown || '').match(/\|\s*\|?\s*:?-{3,}:?\s*(?:\|\s*:?-{3,}:?\s*)+\|/);
}

function formatTableRow(row, expectedCellCount = 0) {
  const cells = mergeOverflowTableCells(splitTableCells(row), expectedCellCount);
  if (cells.length === 0) return '| |';
  if (isSeparatorCells(cells)) return `| ${cells.map(() => '---').join(' | ')} |`;
  return `| ${cells.join(' | ')} |`;
}

function isFormattedTableRow(row) {
  return /^\|\s*.+\s*\|$/.test(String(row || '').trim());
}

function splitTableCells(row) {
  return String(row || '')
    .replace(/^\|/, '')
    .replace(/\|$/, '')
    .split('|')
    .map((cell) => cell.trim())
    .filter(Boolean);
}

function isSeparatorCells(cells) {
  return cells.length > 0 && cells.every((cell) => /^:?-{3,}:?$/.test(cell));
}

function formatCompactTableHeading(heading) {
  const text = String(heading || '').replace(/^[-*]\s+/, '').trim();
  if (!text) return '';
  return /^#{1,6}\s+/.test(text) ? text : `### ${text}`;
}

function normalizeLooseCompactTableDelimiters(markdown) {
  return String(markdown || '').replace(/\|\s+\|(?=\s*(?:\*\*|:?-{3,}:?))/g, '||');
}

/**
 * Recover a GFM table that was flattened into one prose line.
 *
 * The separator row is the structural anchor.  Cells immediately before it
 * become the header, cells after it are grouped by the inferred column count,
 * and leftover text is returned to the generic Markdown splitter.  This keeps
 * table recovery resilient without creating an application-specific parser.
 */
function expandInlineCompactPipeTable(markdown, fallbackLine) {
  const separatorMatch = findSeparatorMatch(markdown);
  if (!separatorMatch || separatorMatch.index === undefined) return [fallbackLine];

  const separatorRow = separatorMatch[0].replace(/^\|\s*\|/, '|');
  const expectedCellCount = splitTableCells(separatorRow).length;
  if (expectedCellCount < 2) return [fallbackLine];

  const beforeSeparator = markdown.slice(0, separatorMatch.index).trim();
  const afterSeparator = markdown.slice(separatorMatch.index + separatorMatch[0].length).trim();
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

function expandCompactTableRows(rows, expectedCellCount) {
  if (!expectedCellCount) return rows.map((row) => formatTableRow(row, expectedCellCount));

  const expandedRows = [];
  const pendingCells = [];

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

function flushPendingCompactCells(pendingCells, expectedCellCount, expandedRows, flushRemainder = false) {
  while (pendingCells.length >= expectedCellCount) {
    expandedRows.push(formatTableRow(pendingCells.splice(0, expectedCellCount).join(' | '), expectedCellCount));
  }

  if (flushRemainder && pendingCells.length > 0) {
    expandedRows.push(formatTableRow(pendingCells.splice(0).join(' | '), expectedCellCount));
  }
}

function splitInlineTableCells(markdown) {
  return String(markdown || '')
    .replace(/^\|/, '')
    .split('|')
    .map((cell) => cell.trim())
    .filter(Boolean);
}

function groupInlineTableCells(cells, expectedCellCount, expandedRows) {
  const pending = [...cells];
  while (pending.length >= expectedCellCount) {
    expandedRows.push(formatTableRow(pending.splice(0, expectedCellCount).join(' | '), expectedCellCount));
  }
  return pending.join(' | ').trim();
}

function mergeOverflowTableCells(cells, expectedCellCount) {
  if (!expectedCellCount || cells.length <= expectedCellCount) return cells;
  const fixedCells = cells.slice(0, expectedCellCount - 1);
  fixedCells.push(cells.slice(expectedCellCount - 1).join(' \\| '));
  return fixedCells;
}
