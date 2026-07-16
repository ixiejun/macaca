# Office Spreadsheet Pack

`pack.office.spreadsheet.v1` describes provider-neutral workbook and grid
capabilities. The pack is descriptor-only until a spreadsheet provider is
installed through the runtime composition root.

## Manifest Declaration

Declare the pack as required only when workbook access is mandatory for
readiness. Optional declarations degrade with structured unavailable diagnostics.

```toml
[service_contract]
optional_packs = ["pack.office.spreadsheet.v1"]
```

## Permissions

Use the narrowest scope: `spreadsheet.provider.inspect`,
`spreadsheet.workbook.create`, `spreadsheet.workbook.import`,
`spreadsheet.workbook.open`, `spreadsheet.worksheet.read`,
`spreadsheet.range.read`, `spreadsheet.formula.read`,
`spreadsheet.formula.write`, `spreadsheet.range.write`,
`spreadsheet.table.manage`, `spreadsheet.chart.manage`,
`spreadsheet.pivot.manage`, `spreadsheet.filter.manage`,
`spreadsheet.protection.manage`, `spreadsheet.calculate`,
`spreadsheet.export`, `spreadsheet.events.read`, and
`spreadsheet.artifact.read`.

## Capability Model

Macaca models spreadsheets as scoped workbook handles, worksheets, bounded
ranges, cells by reference, value matrices, formulas, named ranges, tables,
charts, pivots, filters, sort rules, validation/protection records, update
plans, calculation plans, export plans, artifact handles, and collaboration
events. Raw cell values, hidden sheets, private formulas, provider-native
formula engines, credentials, and provider payloads stay behind adapters.

## Platform Comparison

Google Sheets API spreadsheets, sheets, ranges, values, batch updates, filters,
pivots, and charts map to workbook, worksheet, range, update, table, pivot, and
chart DTOs. Microsoft Graph Excel workbooks, sessions, tables, ranges, charts,
and worksheets map to handles and plan/request commands. LibreOffice Calc UNO
and OpenXML spreadsheet package concepts map to provider adapters and portable
projections. Native formula syntax remains provider-owned unless exposed through
portable references.

## Commands

`spreadsheet.inspect_provider`, `spreadsheet.create_workbook_request`,
`spreadsheet.import_workbook_request`, `spreadsheet.open_workbook`,
`spreadsheet.list_worksheets`, `spreadsheet.inspect_structure`,
`spreadsheet.read_range`, `spreadsheet.inspect_formulas`,
`spreadsheet.inspect_tables`, `spreadsheet.inspect_charts`,
`spreadsheet.inspect_pivots`, `spreadsheet.plan_update`,
`spreadsheet.update_request`, `spreadsheet.plan_recalculate`,
`spreadsheet.recalculate_request`, `spreadsheet.plan_export`,
`spreadsheet.export_request`, `spreadsheet.inspect_events`, and
`spreadsheet.get_artifact_handle` are descriptor-owned schema names.

## App-Facing Examples

- Inspect provider capabilities before opening a workbook.
- Open or import a workbook and list worksheets through scoped handles.
- Read bounded ranges and use cursor metadata for paging large grids.
- Inspect formulas, tables, charts, and pivots only when capability metadata
  reports support.
- Use update and recalculation plans before mutating cells or formulas.
- Export through artifact handles and never log raw workbook values.
- Treat stale version hashes, protection failures, formula denials, and quota
  failures as structured results.

## App-Facing Example Matrix

Generic examples cover provider inspection, workbook create/import/open,
worksheet listing, structure inspection, range reading, formula/table/chart/pivot
inspection, update planning/request, recalculation planning/request, export
planning/request, event inspection, and artifact handles with synthetic
workbook, worksheet, range, formula, event, and artifact refs.

Diagnostic examples cover unavailable provider, missing workbook permission,
stale version, range-anchor stale, unsupported format, formula denied,
calculation denied, schema mismatch, export denied, write approval, protection
denied, provider quota, network denied, hidden sheet redacted, and artifact
denied. Diagnostics must not include provider names, credentials, private
workbook data, hidden sheet content, raw formulas with secrets, raw exports, or
workflow-specific conventions.

## Trace And Audit

Traces should record declaration, admission decision, command name, workbook id,
worksheet id, range anchor hash, provider class, capability hash, result status,
calculation mode, and artifact id. They must not record raw cell values, hidden
sheet contents, private formulas, credentials, raw exports, or provider payloads.

## Provider Authors

Descriptors must report formats, max cells, formula support, table/chart/pivot
support, filter and sort limits, validation/protection behavior, export formats,
calculation modes, rate limits, health, and snapshot metadata. Providers must
return structured denied, unavailable, unsupported, conflict, stale-version,
schema-mismatch, format-unsupported, formula-denied, calculation-denied,
export-denied, write-denied, protection-denied, quota, timeout, cancellation,
approval-required, and failure results.

Conformance tests should cover descriptor completeness, workbook and range scope
validation, formula safety, update validation, calculation behavior, protection
policy, export validation, artifact redaction, resource bounds, policy hooks,
trace and audit events, unavailable behavior, snapshot/replay, and redaction.
