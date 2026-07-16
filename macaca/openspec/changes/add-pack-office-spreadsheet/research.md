# Office Spreadsheet Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
explicit non-goals, existing platform inventory, and GitNexus memo evidence for
`pack.office.spreadsheet.v1`. Spreadsheet support must expose workbook, sheet,
range, table, formula, chart, pivot, filter, protection, metadata, calculation,
and import/export operations through typed service commands, not raw provider
APIs or application-specific finance/reporting flows.

## Source Baseline

- Google Sheets API ranges, protected ranges, pivots, and batch updates:
  <https://developers.google.com/workspace/sheets/api/samples/ranges>
  and <https://developers.google.com/workspace/sheets/api/samples/pivot-tables>
- Microsoft Graph Excel workbook APIs:
  <https://learn.microsoft.com/en-us/graph/api/resources/excel>
  and <https://learn.microsoft.com/en-us/graph/excel-concept-overview>
- Office JavaScript Excel core object model and PivotTables:
  <https://learn.microsoft.com/en-us/office/dev/add-ins/excel/excel-add-ins-core-concepts>
  and <https://learn.microsoft.com/en-us/office/dev/add-ins/excel/excel-add-ins-pivottables>
- OpenXML SpreadsheetML structure and worksheet APIs:
  <https://learn.microsoft.com/en-us/office/open-xml/spreadsheet/structure-of-a-spreadsheetml-document>
  and <https://learn.microsoft.com/en-us/office/open-xml/spreadsheet/working-with-sheets>

## Supplier API Notes

- Google Sheets contributes spreadsheets, sheets, grid data, values APIs,
  `spreadsheets.batchUpdate`, charts, pivots, filters, named/protected ranges,
  developer metadata, and batch value operations. Macaca should map these to
  workbook/sheet/range/value/mutation/pivot/filter/protection metadata.
- Microsoft Graph Excel contributes workbook sessions, worksheets, ranges,
  tables, charts, named items, filters, pivot tables, formulas, and workbook
  operations over OneDrive/SharePoint drive items. Macaca should model session
  handles, cloud-file identity, and provider calculation constraints.
- Office JavaScript Excel contributes host-scoped workbook, worksheet, range,
  table, chart, pivot, formula, calculation, protection, and event objects.
  Macaca should expose host capability reports and not assume every workbook
  provider has interactive event support.
- OpenXML SpreadsheetML contributes offline package parts for workbooks,
  worksheets, cells, shared strings, formulas, tables, charts, pivots, styles,
  and calculation metadata. Macaca should support package import/export and
  deterministic structural mutation without exposing XML part names as SDK API.

## Macaca-Owned Abstractions

`pack.office.spreadsheet.v1` should define `SpreadsheetWorkbook`,
`SpreadsheetSheet`, `SpreadsheetRange`, `SpreadsheetCell`,
`SpreadsheetCellValue`, `SpreadsheetFormula`, `SpreadsheetTable`,
`SpreadsheetChart`, `SpreadsheetPivot`, `SpreadsheetFilter`,
`SpreadsheetProtection`, `SpreadsheetMetadata`, `SpreadsheetCalculationState`,
`SpreadsheetMutation`, and `SpreadsheetProviderCapability`.

The DTOs must carry workbook identity, sheet/range coordinates, value typing,
formula policy, recalculation behavior, table/chart/pivot metadata, protection
state, session/revision preconditions, import/export handles, redaction profile,
and replay pointers. Raw provider payloads, private workbook content, raw
formulas beyond policy, credentials, package bytes, and unbounded grid exports
are rejected.

## Explicit Non-Goals

- Do not implement concrete Google Sheets, Excel, Office.js, OpenXML,
  LibreOffice Calc, PDF, cloud-drive, OCR, or conversion providers in this
  research phase.
- Do not define finance, accounting, reporting, KPI dashboard, template, or
  workflow-specific spreadsheet semantics in OS layers.
- Do not pass raw formula strings, provider requests, workbook package bytes,
  or provider-specific object ids as stable cross-boundary contracts.

## Existing Macaca Platform Inventory

- Generic descriptors, `SystemFacade`, trace-required service calls,
  unavailable/null-object behavior, policy/resource gates, persistence
  snapshots, and domain-pack registration can support future spreadsheet
  serviceization.
- Finance/accounting packs may consume spreadsheet outputs later, but
  spreadsheet must remain provider-neutral and application-neutral.
- Current evidence does not prove spreadsheet DTOs, providers, SDK helpers,
  WASM ABI metadata, tests, dependency gates, or developer docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
