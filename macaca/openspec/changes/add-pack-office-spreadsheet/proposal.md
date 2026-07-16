# Change: Add Office Spreadsheet Pack

## Why

Developers need `pack.office.spreadsheet.v1` as an industrial spreadsheet
capability for workbook creation/import/opening, worksheet discovery, range and
cell reading, batch value/formula/style writes, table operations, chart
operations, pivot operations, filters/sorts, named ranges, data validation,
protection, recalculation, export, collaboration events, and replay diagnostics.
It must not be a thin wrapper around Google Sheets, Microsoft Excel, Office.js,
OpenXML, LibreOffice Calc, or one spreadsheet format.

Spreadsheets often contain financial models, personal data, pricing tables,
business forecasts, formulas, external links, hidden sheets, protected ranges,
charts, pivots, macros, and collaborator comments. Mutating formulas or ranges
can alter financial meaning, trigger external data access, or corrupt models.
Macaca must therefore expose spreadsheet operations only through provider-neutral
typed service commands with permission, policy, entitlement, resource, approval,
formula safety, version preconditions, redaction, trace, audit, health,
snapshot, replay, and structured unavailable behavior.

## Research And Supplier/API Baseline

Official references considered for this pack:

- Google Sheets API exposes spreadsheets, sheets, grid data, values,
  `spreadsheets.batchUpdate`, charts, pivot tables, filters, developer metadata,
  protected ranges, and values batch operations. Reference:
  https://developers.google.com/sheets/api/reference/rest
- Microsoft Graph Excel APIs expose workbook sessions, worksheets, ranges,
  tables, charts, names, filters, pivots, formulas, and workbook operations.
  Reference: https://learn.microsoft.com/en-us/graph/api/resources/excel
- Office JavaScript Excel APIs expose workbook, worksheet, range, table, chart,
  pivot table, formula, calculation, protection, and event objects. Reference:
  https://learn.microsoft.com/en-us/office/dev/add-ins/reference/overview/excel-add-ins-reference-overview
- OpenXML SpreadsheetML exposes workbook packages, worksheets, cells, shared
  strings, formulas, tables, charts, pivots, styles, and calculation metadata.
  Reference: https://learn.microsoft.com/en-us/office/open-xml/spreadsheet/working-with-spreadsheets

Macaca maps these supplier concepts into provider-neutral workbook, worksheet,
range, cell, value matrix, formula, named range, table, chart, pivot, filter,
sort, validation, protection, calculation plan, batch update plan, export plan,
artifact handle, collaboration event, version/freshness metadata, and provider
capability DTOs. Concrete Google Sheets, Excel, Office.js, OpenXML, LibreOffice
Calc, cloud-drive, and conversion providers stay behind replaceable providers.

## What Changes

- Add provider-neutral `pack.office.spreadsheet.v1` under the `office` family.
- Define command namespace `spreadsheet.*` for:
  - provider and format capability inspection
  - workbook creation/import/opening and worksheet discovery
  - range/cell/value/formula/table/chart/pivot/filter/protection inspection
  - batch update planning and write requests
  - formula setting, formula validation, and recalculation planning
  - table/chart/pivot/filter/sort/data-validation/protection operations
  - named range and metadata operations
  - export/render artifact planning and requests
  - collaboration/change event inspection
  - workbook snapshots and replay diagnostics
- Define DTOs for spreadsheet scope, provider capability, workbook, worksheet,
  range, cell, value matrix, formula, named range, table, chart, pivot, filter,
  sort, data validation, protection, batch update plan, calculation plan, export
  plan, artifact handle, collaboration event, version/freshness metadata, and
  diagnostics.
- Define permission scopes, policy defaults, workbook/sheet/range scopes, formula
  safety, external-link policy, version-precondition behavior, recalculation
  safety, artifact redaction, resource/entitlement behavior, approval rules, SDK
  discovery, developer documentation, trace/audit events, snapshots, replay, and
  boundary gates.
- Require detailed developer documentation at
  `docs/developer-packs/office/spreadsheet.md` before implementation completion.

## Impact

- Affected specs: `pack-office-spreadsheet`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral protocol DTOs, pack descriptors,
  admission validators, SDK discovery and command builders, spreadsheet service
  provider or unavailable provider, runtime-host provider adapters,
  formula/calculation/artifact/redaction support, trace/audit schemas, replay
  tests, dependency-boundary gates, and developer documentation.
- Non-goals: no concrete Google Sheets/Excel/Office.js/OpenXML/LibreOffice/PDF/
  cloud-drive/conversion provider implementation in this proposal; no
  app-specific finance/reporting/dashboard/modeling workflow; no provider-name,
  workbook-name, sheet-name, range-name, formula-name, or workflow-name routing
  in OS layers; no raw credentials, private workbook data, hidden sheet content,
  external-link secrets, raw exports, raw provider payloads, prompts, manifests,
  or unbounded grid data in observability; no SDK/shell/kernel provider
  construction; no fake success when provider, format support, permission,
  entitlement, approval, resource, version, calculation, or host support is
  absent.
