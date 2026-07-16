# Office Spreadsheet Pack Design

## Context

`pack.office.spreadsheet.v1` exposes spreadsheet capabilities as a Macaca OS
serviceized capability. It lets applications create, import, open, inspect,
update, calculate, export, and replay spreadsheets without embedding Google
Sheets, Microsoft Excel, Office.js, OpenXML, LibreOffice Calc, cloud-drive APIs,
workbook names, model conventions, or application-specific reporting workflows
into generic OS layers.

Spreadsheets are executable business models. Reads can leak private financial
data; writes can corrupt formulas, pivots, charts, or protected ranges; formulas
can reference external data or execute risky functions depending on provider.
The pack therefore models writes and recalculation as validated plans and
requests with range/version preconditions, formula safety, external-link policy,
resource bounds, redaction, approval, trace/audit evidence, replay, and provider
replacement.

## Supplier Capability Matrix

| Supplier/platform | Industrial capability | Macaca abstraction |
| --- | --- | --- |
| Google Sheets API | Spreadsheets, sheets, values, grid data, batchUpdate, charts, pivots, filters, protected ranges, metadata | Workbook, worksheet, range, value matrix, batch update plan, chart/pivot/filter/protection DTOs |
| Microsoft Graph Excel | Workbook sessions, worksheets, ranges, tables, charts, names, filters, pivots, formulas | Workbook session, worksheet, range, table, chart, named range, pivot, formula, provider capability |
| Office JavaScript Excel API | Workbook/worksheet/range objects, tables, charts, pivot tables, formulas, calculation, protection, events | Workbook model, calculation plan, protection metadata, event stream, service command DTOs |
| OpenXML SpreadsheetML | Workbook packages, worksheets, cells, shared strings, formulas, tables, charts, pivots, styles, calculation metadata | Spreadsheet package adapter, cell/value/formula model, style/table/chart/pivot metadata, export artifact |

The pack exposes provider-neutral contracts. Provider adapters translate to
cloud spreadsheet APIs, local file packages, desktop automation bridges, or
conversion services. OS layers must not branch on provider names, workbook names,
sheet names, cell addresses, formulas, formats, or business modeling workflows.

## Goals

- Provide stable pack id `pack.office.spreadsheet.v1` and command namespace
  `spreadsheet.*`.
- Support provider inspection, workbook create/import/open, worksheet listing,
  structure inspection, range reading, formula/table/chart/pivot/filter/
  protection inspection, batch update planning, update requests, formula setting,
  recalculation planning/request, table/chart/pivot/filter/sort/data-validation/
  protection operations, named range metadata, export planning/request,
  collaboration event inspection, snapshots, health, and replay.
- Preserve safety with workbook/sheet/range scope validation, formula safety,
  external-link policy, version preconditions, batch validation, recalculation
  bounds, artifact retention, approval, quotas, and sanitized audit.
- Keep concrete spreadsheet providers behind replaceable service providers.
- Require developer documentation at
  `docs/developer-packs/office/spreadsheet.md`.

## Non-Goals

- Do not implement concrete Google Sheets, Excel, Office.js, OpenXML,
  LibreOffice Calc, PDF, cloud-drive, OCR, or conversion providers in this
  proposal.
- Do not define application-specific finance, reporting, dashboard, analytics,
  forecasting, accounting, modeling, or workbook-template workflows.
- Do not execute document, presentation, PDF, finance, storage, email, or
  notification semantics directly; those belong to separate packs/services and
  may be linked by handles.
- Do not expose raw credentials, private workbook data, hidden sheet content,
  formula secrets, external-link secrets, raw exports, raw provider payloads,
  prompts, manifests, package bytes, private keys, signatures, or unbounded grid
  data in observability.
- Do not silently write ranges, set formulas, recalculate, update charts/pivots,
  export, overwrite protected ranges, or notify collaborators without typed
  request, policy checks, version preconditions, and approval where required.

## Ownership And Boundaries

- Pack id: `pack.office.spreadsheet.v1`.
- Family: `office`.
- Backing service owner: spreadsheet service provider.
- SDK surface: `sdk.packs.office.spreadsheet`.
- Command namespace: `spreadsheet.*`.
- Microkernel owns identity, policy facade, resource primitives, service-call
  evidence, trace/audit primitives, and registry metadata only.
- Application framework owns manifest declarations, app-scoped permissions, and
  effective capability projection.
- Runtime host owns provider adapter registration, credential bridges, artifact
  stores, conversion/desktop bridges, decorators, and sanitized diagnostics
  through approved composition roots.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `spreadsheet.inspect_provider` | Inspect provider/format capability | Returns sanitized workbook, sheet, range, formula, chart, pivot, export, quota, and health metadata |
| `spreadsheet.create_workbook_request` | Create a workbook from metadata/template handle | Requires idempotency key, format policy, write permission, and audit |
| `spreadsheet.import_workbook_request` | Import workbook from file/artifact handle | Requires artifact permission, format validation, conversion policy, and audit |
| `spreadsheet.open_workbook` | Resolve workbook handle and version metadata | Requires workbook scope and bounded metadata |
| `spreadsheet.list_worksheets` | List worksheets and bounded visibility metadata | Requires sheet permission and redaction |
| `spreadsheet.inspect_structure` | Inspect sheets, dimensions, names, tables, charts, pivots, filters, protections, and calculation metadata | Requires projection limits and redaction |
| `spreadsheet.read_range` | Read bounded cells/values/formulas from a range | Requires range scope, content bounds, formula policy, and redaction |
| `spreadsheet.inspect_formulas` | Inspect formulas, dependencies, external links, volatile/risky functions, and calculation state | Requires formula permission and redaction |
| `spreadsheet.inspect_tables` | Inspect table definitions and ranges | Requires table permission and bounded output |
| `spreadsheet.inspect_charts` | Inspect chart definitions and source ranges | Requires chart permission and redaction |
| `spreadsheet.inspect_pivots` | Inspect pivot definitions, source ranges, and refresh state | Requires pivot permission and bounded output |
| `spreadsheet.plan_update` | Plan batch value/formula/style/table/chart/pivot/filter/protection updates | Validates ranges, formulas, versions, recalculation, notifications, and approvals |
| `spreadsheet.update_request` | Request validated batch updates | Requires plan handle, idempotency key, write permission, version preconditions, and audit |
| `spreadsheet.plan_recalculate` | Plan recalculation or refresh | Validates calculation mode, affected ranges, external links, pivots, volatile functions, resources, and approvals |
| `spreadsheet.recalculate_request` | Request validated recalculation/refresh | Requires plan handle, idempotency key, and audit |
| `spreadsheet.plan_export` | Plan workbook/sheet/range export or render artifact generation | Validates format, scope, sensitivity, retention, and approvals |
| `spreadsheet.export_request` | Request export artifact from a validated plan | Returns bounded artifact handle and audit metadata |
| `spreadsheet.inspect_events` | Inspect collaboration/change events where supported | Requires event filters, redaction, paging, and retention |
| `spreadsheet.get_artifact_handle` | Resolve export/import artifact metadata | Requires artifact permission, retention, and redaction |

Every command must define typed command DTOs, typed success results, typed
paged/partial results, typed denied/unavailable/unsupported/conflict/
stale-version/schema-mismatch/format-unsupported/formula-denied/
calculation-denied/export-denied/write-denied/protection-denied/quota/timeout/
cancellation/approval-required/failure results, redaction profile, idempotency
semantics for side effects, and replay metadata.

## DTO Model

Core DTOs:

- `SpreadsheetScope`: provider scope handle, workbook handle, workspace/file
  handle, credential reference, network policy, artifact policy, permission
  state, rate-limit profile, and health.
- `SpreadsheetProviderCapability`: provider class, create/open/import support,
  worksheet support, range support, formula support, table support, chart
  support, pivot support, filter/sort support, validation/protection support,
  calculation support, export support, collaboration event support, auth modes,
  rate limits, lifecycle, and health.
- `WorkbookHandle`: workbook handle, provider scope, title handle, format,
  version hash, freshness, permission state, sensitivity class, and redaction
  class.
- `WorksheetHandle`: worksheet handle, workbook handle, name handle, index,
  visibility, dimension summary, protection state, version hash, and redaction
  class.
- `SpreadsheetRange`: range handle, workbook/sheet handle, address handle,
  row/column bounds, named-range handle, version precondition, and redaction
  class.
- `SpreadsheetCell`: cell handle, range handle, row/column index class, value
  handle, formula handle, format handle, validation handle, protection state,
  and sensitivity class.
- `SpreadsheetValueMatrix`: range handle, row/column count class, value handles,
  formula handles, type metadata, truncation flags, and redaction class.
- `SpreadsheetFormula`: formula handle, expression handle, dependency handles,
  external link handles, volatile/risky function metadata, locale, calculation
  state, and sensitivity class.
- `SpreadsheetNamedRange`: name handle, range handle, scope, version hash, and
  redaction class.
- `SpreadsheetTable`: table handle, range handle, header metadata, total row
  metadata, style handle, filter metadata, and redaction class.
- `SpreadsheetChart`: chart handle, chart kind, source range handles, axis/series
  metadata, artifact policy, and redaction class.
- `SpreadsheetPivot`: pivot handle, source range handle, row/column/value/filter
  field handles, refresh state, and redaction class.
- `SpreadsheetFilterSort`: filter/sort handle, target range/table handle,
  predicate handles, sort keys, and compatibility metadata.
- `SpreadsheetValidationProtection`: validation/protection handle, target range,
  rule handles, lock/permission state, and sensitivity class.
- `SpreadsheetUpdateOperation`: operation handle, operation kind, target range/
  table/chart/pivot/protection handle, payload handle, and validation metadata.
- `SpreadsheetUpdatePlan`: plan handle, workbook handle, operation list hash,
  version preconditions, formula policy, recalculation policy, notification
  policy, required approvals, idempotency key, and validation diagnostics.
- `SpreadsheetCalculationPlan`: plan handle, workbook/sheet/range handles,
  calculation mode, dependency scope, external-link policy, volatile function
  policy, pivot refresh policy, resource reservation, approvals, and diagnostics.
- `SpreadsheetExportPlan`: plan handle, workbook/sheet/range scope, output
  format, rendering profile, retention, redaction, required approvals,
  idempotency key, and validation diagnostics.
- `SpreadsheetArtifactHandle`: artifact handle, source workbook/sheet/range
  handle, artifact kind, content type, size class, checksum handle, retention,
  redaction class, and replay pointer.
- `SpreadsheetCollaborationEvent`: event handle, workbook/sheet/range handle,
  event kind, actor handle, timestamp, changed fields, redaction class, and
  cursor.

Provider-specific extensions may appear only as bounded `adapter_metadata`
behind capability hashes and must not drive OS-layer routing.

## Permission, Policy, Resource, Entitlement, And Approval Model

Permission scopes:

- `spreadsheet.provider.inspect`
- `spreadsheet.workbook.create`
- `spreadsheet.workbook.import`
- `spreadsheet.workbook.open`
- `spreadsheet.worksheet.read`
- `spreadsheet.range.read`
- `spreadsheet.formula.read`
- `spreadsheet.formula.write`
- `spreadsheet.range.write`
- `spreadsheet.table.manage`
- `spreadsheet.chart.manage`
- `spreadsheet.pivot.manage`
- `spreadsheet.filter.manage`
- `spreadsheet.protection.manage`
- `spreadsheet.calculate`
- `spreadsheet.export`
- `spreadsheet.events.read`
- `spreadsheet.artifact.read`

Policy defaults:

- Every command is scoped to application id, tenant id, session id, task id,
  trace id, provider scope, workbook handle, worksheet/range handle when
  applicable, and actor handle when available.
- Update, formula write, recalculation, chart/pivot/table/protection operations,
  and export commands require plan/request separation, idempotency key, version
  preconditions, formula safety, external-link policy, artifact policy,
  notification policy, credential reference, and audit reason.
- Private workbooks, hidden sheets, protected ranges, financial data, personal
  data, formulas, external links, volatile functions, pivot refreshes, exports,
  destructive edits, and collaborator-visible changes may require approval.
- Range data, formulas, hidden sheets, exports, and artifacts require redaction
  and bounded output. Raw full workbook data must not enter observability.
- Remote operations require network permission, provider quota, rate limits,
  timeout, cancellation, and structured unavailable behavior.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, family, version, lifecycle, command schemas,
format support, worksheet support, range support, formula support, table/chart/
pivot/filter/protection support, calculation support, export support,
collaboration event support, permission scopes, policy templates, resource
limits, approval rules, provider capability hashes, health, compatibility,
diagnostics, examples, redaction profiles, and documentation links.

The developer guide at `docs/developer-packs/office/spreadsheet.md` must cover:

- manifest declaration and optional/required behavior
- provider scopes, workbook handles, supported formats, worksheets, ranges,
  cells, values, formulas, named ranges, tables, charts, pivots, filters, sorts,
  validation, protection, calculations, update plans, export plans, artifacts,
  events, provider capabilities, and unavailable states
- batch update plan/request lifecycle, recalculation lifecycle, export lifecycle,
  formula safety, external links, version conflicts, schema/format mismatch,
  artifact redaction, notification policy, approvals, quotas, provider
  replacement, trace/audit interpretation, and conformance tests

Examples must use synthetic workbooks, sheets, ranges, formulas, charts, pivots,
and artifacts. They must not include provider names, real credentials, private
financial data, hidden sheet content, raw formulas with secrets, raw exports, or
workflow-specific conventions.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `spreadsheet_pack_declared`
- `spreadsheet_pack_admission_validated`
- `spreadsheet_provider_inspected`
- `spreadsheet_workbook_created`
- `spreadsheet_workbook_imported`
- `spreadsheet_workbook_opened`
- `spreadsheet_worksheets_listed`
- `spreadsheet_structure_inspected`
- `spreadsheet_range_read`
- `spreadsheet_formulas_inspected`
- `spreadsheet_tables_inspected`
- `spreadsheet_charts_inspected`
- `spreadsheet_pivots_inspected`
- `spreadsheet_update_planned`
- `spreadsheet_update_requested`
- `spreadsheet_recalculation_planned`
- `spreadsheet_recalculation_requested`
- `spreadsheet_export_planned`
- `spreadsheet_export_requested`
- `spreadsheet_events_inspected`
- `spreadsheet_artifact_handle_resolved`
- `spreadsheet_pack_policy_decision`
- `spreadsheet_pack_service_call_requested`
- `spreadsheet_pack_service_call_succeeded`
- `spreadsheet_pack_service_call_failed`
- `spreadsheet_pack_unavailable`
- `spreadsheet_pack_snapshot_recorded`

Snapshots include descriptor version, provider capability hashes, workbook
format/version hashes, calculation state hashes, command availability, provider
health, policy template hash, resource counters, bounded workbook/sheet/range/
formula/chart/pivot summaries, artifact summaries, event cursors, and sanitized
replay pointers. Snapshots must exclude raw credentials, tokens, private
financial data, hidden sheet content, raw formulas with secrets, raw exports,
raw provider payloads, prompts, manifests, package bytes, private keys,
signatures, and unbounded grid data.

## Design Patterns

- **Facade**: SDK clients expose discovery and command builders only.
- **Command**: every operation is a typed command/result DTO.
- **Strategy**: provider adapters, format readers, range resolvers, formula
  validators, calculation strategies, table/chart/pivot adapters, export
  renderers, redaction, artifact retention, and unavailable behavior are
  replaceable.
- **Decorator**: trace, policy, entitlement, resource, approval, metering,
  network policy, credential redaction, formula redaction, artifact redaction,
  and mutation safety wrap service calls.
- **Specification**: admission validates provider scope, workbook/format support,
  command availability, permissions, version preconditions, formula safety,
  provider state, quota, and compatibility.
- **Observer**: workbook changes, recalculation events, collaboration events,
  provider health, trace, and audit events are subscribable.
- **Memento**: workbook version hashes, range anchors, update plans, calculation
  plans, export plans, artifact handles, event cursors, snapshots, and replay
  pointers preserve recovery state.
- **Abstract Factory**: concrete spreadsheet providers are created only by
  approved runtime-host composition roots.

## Risks And Mitigations

- Risk: pack becomes a Google Sheets or Excel wrapper. Mitigation:
  provider-neutral workbook/worksheet/range/formula/table/chart/pivot/export
  DTOs and Strategy adapters.
- Risk: sensitive workbook data or formula secrets leak. Mitigation: handles,
  redaction, bounded summaries, and strict observability exclusions.
- Risk: formula or recalculation changes corrupt models. Mitigation: validated
  update/calculation plans, version preconditions, formula safety, idempotency,
  approval, and audit.
- Risk: provider formula and chart semantics diverge. Mitigation: explicit
  capability DTO, compatibility hashes, schema-mismatch diagnostics, and
  conformance tests.
- Risk: SDK helpers become a second execution path. Mitigation: helpers build
  canonical service commands and never call spreadsheet APIs directly.
