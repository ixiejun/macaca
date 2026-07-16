## ADDED Requirements

### Requirement: Macaca SHALL provide the Office Spreadsheet Pack as a serviceized capability

Macaca SHALL provide `pack.office.spreadsheet.v1` as a provider-neutral industrial pack for workbook creation, import, opening, worksheet listing, structure inspection, range reading, formula inspection and writing, table/chart/pivot/filter/protection inspection and updates, batch update planning and requests, recalculation planning and requests, export planning and requests, collaboration event inspection, artifact handles, snapshots, and replay diagnostics. The pack SHALL be declared by applications, resolved by admission/catalog services, and invoked only through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.office.spreadsheet.v1` as required and spreadsheet service provider is registered, healthy, entitled, workbook-scoped, format-capable, and policy-admissible
- **THEN** admission SHALL expose `pack.office.spreadsheet.v1` in the effective capability set with command schemas, permission scopes, workbook/format scope metadata, policy template hash, provider capability hash, health, and replay metadata
- **AND** SDK discovery SHALL mark callable `spreadsheet.*` commands as available without exposing provider secrets, raw credentials, private workbook data, hidden sheet content, formula secrets, raw exports, raw provider payloads, or application-specific workflow names

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.office.spreadsheet.v1` as required but provider, format support, credential reference, workbook permission, entitlement, resource, approval, network, calculation support, host support, or policy admission is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, mutate workbooks, set formulas, recalculate, refresh pivots, export workbooks, notify collaborators, contact a provider, or fake success

#### Scenario: Optional declaration degrades explicitly
- **WHEN** an application declares `pack.office.spreadsheet.v1` as optional and the pack or a sub-capability is unavailable
- **THEN** admission SHALL produce a degraded effective capability memento naming unavailable commands and bounded reason codes
- **AND** SDK command helpers SHALL refuse to build callable service calls for unavailable commands while preserving discoverability and diagnostics

### Requirement: Spreadsheet commands SHALL use typed canonical service calls

Every `pack.office.spreadsheet.v1` operation SHALL be represented as a typed command/result DTO and SHALL traverse the canonical service runtime path with trace, policy, resource, entitlement, approval, health, snapshot, and structured error behavior. SDK, WASM ABI, shell, and application-framework helpers SHALL only build canonical service commands and SHALL NOT construct concrete spreadsheet providers or call spreadsheet APIs directly.

#### Scenario: Read command succeeds through service runtime
- **WHEN** a declared and policy-allowed command such as `spreadsheet.open_workbook`, `spreadsheet.list_worksheets`, `spreadsheet.inspect_structure`, or `spreadsheet.read_range` is invoked
- **THEN** Macaca SHALL route the command through SDK/facade helpers into the service runtime and spreadsheet service provider
- **AND** it SHALL emit sanitized admission, policy, service-call, result, and replay events with stable trace identifiers and bounded range/projection metadata

#### Scenario: Update or recalculation is planned before request
- **WHEN** an application wants to write ranges, set formulas, update tables/charts/pivots/filters/protection, recalculate, refresh, or export
- **THEN** Macaca SHALL require the applicable planning command with workbook/sheet/range validation, formula safety, external-link policy, version preconditions, recalculation policy, notification policy, artifact policy, resource reservation, idempotency key, approval state where required, and provider capability validation
- **AND** planning commands SHALL be replay-addressable and SHALL NOT mutate workbooks, set formulas, recalculate, refresh pivots, export, or notify collaborators

#### Scenario: Command is denied before provider invocation
- **WHEN** policy, permission, entitlement, approval, resource, quota, workbook scope, range anchor, formula safety, external-link policy, format, schema, version, calculation support, protection, export, artifact, provider capability, or timeout checks reject a `spreadsheet.*` command
- **THEN** Macaca SHALL return a typed denied, validation, conflict, stale-version, schema-mismatch, format-unsupported, formula-denied, calculation-denied, export-denied, write-denied, protection-denied, approval-required, quota, timeout, unavailable, or unsupported result before invoking the concrete provider
- **AND** the audit trail SHALL include only bounded reason codes and sanitized handles

### Requirement: Spreadsheet DTOs SHALL model provider-neutral spreadsheet concepts

`pack.office.spreadsheet.v1` SHALL define provider-neutral DTOs for spreadsheet scope, provider capability, workbook, worksheet, range, cell, value matrix, formula, named range, table, chart, pivot, filter/sort, validation/protection, update operation, update plan, calculation plan, export plan, artifact handle, collaboration event, version/freshness metadata, and diagnostics. Provider-specific fields SHALL be exposed only as bounded `adapter_metadata` guarded by capability hashes and SHALL NOT drive OS-layer routing branches.

#### Scenario: Provider capability is inspected
- **WHEN** `spreadsheet.inspect_provider` is invoked for a provider or workbook scope
- **THEN** Macaca SHALL return provider-neutral `SpreadsheetProviderCapability` metadata for create/open/import support, worksheet support, range support, formula support, table support, chart support, pivot support, filter/sort support, validation/protection support, calculation support, export support, collaboration event support, auth modes, rate limits, lifecycle, health, and compatibility
- **AND** it SHALL include stable descriptor, provider capability, policy template, and compatibility hashes for validation and replay

#### Scenario: Range is read
- **WHEN** `spreadsheet.read_range` returns cells, values, or formulas
- **THEN** the result SHALL use `SpreadsheetRange`, `SpreadsheetCell`, and `SpreadsheetValueMatrix` metadata with row/column bounds, value handles, formula handles, type metadata, truncation flags, version preconditions, and redaction class
- **AND** it SHALL NOT expose raw full workbook data, hidden sheet content, formula secrets, private financial data, raw provider payloads, or unbounded grid data

#### Scenario: Provider-specific capability exists
- **WHEN** an active provider supports a spreadsheet concept not present in the canonical DTO model
- **THEN** the provider MAY expose bounded `adapter_metadata` and compatibility diagnostics through `SpreadsheetProviderCapability`
- **AND** the OS, SDK, shell, and generic application framework SHALL NOT branch on provider names, workbook names, sheet names, cell addresses, formulas, formats, or business modeling workflows

### Requirement: Updates, formulas, recalculation, pivots, and exports SHALL be version-safe, formula-safe, approval-aware, and auditable

All spreadsheet side effects SHALL use plan/request separation, workbook/sheet/range scope validation, range anchor freshness, format compatibility, formula safety, external-link policy, provider capability validation, resource reservations, idempotency, version preconditions, recalculation policy, collaboration notification policy, approval gates where required, and sanitized audit.

#### Scenario: Batch update is requested
- **WHEN** `spreadsheet.plan_update` validates value writes, formula writes, style changes, table/chart/pivot/filter/protection operations, ranges, version preconditions, formula safety, recalculation policy, notification policy, quota, and approvals
- **THEN** `spreadsheet.update_request` MAY use the validated plan handle and idempotency key to request updates
- **AND** Macaca SHALL record sanitized plan, request, workbook version hash, range anchor hash, formula dependency hash, provider capability hash, policy decision, audit reason, result handles, and replay pointer

#### Scenario: Formula write is denied
- **WHEN** `spreadsheet.plan_update` detects formula injection, external-link access, volatile/risky functions, credential-like literals, unsupported formula syntax, or policy-denied formula behavior
- **THEN** Macaca SHALL return typed formula-denied or approval-required diagnostics before provider invocation
- **AND** traces, audits, snapshots, and SDK diagnostics SHALL use formula handles or bounded summaries rather than raw sensitive formulas

#### Scenario: Recalculation is requested
- **WHEN** `spreadsheet.plan_recalculate` validates calculation mode, dependency scope, affected ranges, external-link policy, volatile function policy, pivot refresh policy, resource reservation, provider support, and approvals
- **THEN** `spreadsheet.recalculate_request` MAY request recalculation or refresh
- **AND** recalculation that can fetch external data, refresh pivots, or materially change workbook outputs SHALL be approval-gated when policy requires approval

#### Scenario: Export is requested
- **WHEN** `spreadsheet.plan_export` validates workbook/sheet/range scope, output format, rendering profile, retention, sensitivity, resource budget, and approvals
- **THEN** `spreadsheet.export_request` MAY request export through the service provider
- **AND** it SHALL return bounded `SpreadsheetArtifactHandle` metadata rather than raw workbook exports in traces, audits, snapshots, examples, or diagnostics

### Requirement: Spreadsheet ranges, formulas, hidden sheets, events, and artifacts SHALL be bounded and policy-controlled

`pack.office.spreadsheet.v1` SHALL treat range data, formulas, hidden sheets, protected ranges, external links, financial data, personal data, exports, collaboration events, and provider payloads as policy-controlled resources with explicit permissions, quotas, redaction, retention, and provider capability checks.

#### Scenario: Hidden or protected range is accessed
- **WHEN** a command targets hidden sheets, protected ranges, formula cells, or sensitive workbook areas
- **THEN** Macaca SHALL validate the relevant worksheet/range/formula/protection permission, sensitivity class, redaction profile, approval state, and provider capability
- **AND** it SHALL return denied, protection-denied, or redacted results when policy does not allow raw access

#### Scenario: Collaboration events are inspected
- **WHEN** `spreadsheet.inspect_events` is invoked
- **THEN** Macaca SHALL return bounded `SpreadsheetCollaborationEvent` records with event kind, actor handle, timestamp, changed fields, cursor, and redaction class
- **AND** it SHALL enforce event permission, page size, redaction, retention, timeout, and replay bounds

#### Scenario: Artifact handle is resolved
- **WHEN** `spreadsheet.get_artifact_handle` is invoked
- **THEN** Macaca SHALL validate artifact permission, source workbook/sheet/range handle, content type, size class, retention, redaction class, provider capability, resource budget, and approval requirements
- **AND** it SHALL return bounded artifact metadata rather than raw exports, workbook packages, or raw provider payloads

### Requirement: Spreadsheet Pack SHALL enforce permissions, scopes, resources, entitlements, approvals, and redaction

`pack.office.spreadsheet.v1` SHALL enforce explicit permission scopes for provider inspection, workbook create/import/open, worksheet reading, range reading, formula reading, formula writing, range writing, table management, chart management, pivot management, filter management, protection management, calculation, export, event reading, and artifact reading. Every command SHALL carry application id, tenant id, session id, task id, trace id, provider scope, workbook handle, worksheet/range handle where applicable, and actor handle when available.

#### Scenario: Permission is missing
- **WHEN** an application invokes a `spreadsheet.*` command without the required permission scope
- **THEN** Macaca SHALL return a typed denied result before provider invocation
- **AND** the denied result SHALL identify the missing permission scope using sanitized identifiers

#### Scenario: Resource budget is exceeded
- **WHEN** workbook import, worksheet listing, structure inspection, range reading, formula inspection, table/chart/pivot inspection, update planning, recalculation, export, event inspection, or artifact retrieval exceeds workbook size, sheet count, range size, cell count, formula dependency count, table/chart/pivot count, update operation count, calculation cost, export size, artifact size, provider quota, network transfer, timeout, memory, storage, or snapshot retention budgets
- **THEN** Macaca SHALL return typed quota, timeout, cancellation, calculation-denied, export-denied, artifact-denied, or resource-denied diagnostics
- **AND** it SHALL preserve replayable audit evidence without raw workbook data, hidden sheet content, formulas with secrets, or provider payloads

#### Scenario: Sensitive operation requires approval
- **WHEN** policy marks private workbooks, hidden sheets, protected ranges, financial data, personal data, formula writes, external links, volatile functions, recalculation/refresh, destructive edits, exports, or collaborator-visible changes as approval-required
- **THEN** Macaca SHALL return an approval-required result until a valid approval token is supplied
- **AND** no workbook mutation, formula write, recalculation, pivot refresh, export, collaborator notification, external-link fetch, or raw artifact retrieval SHALL happen before approval

### Requirement: Spreadsheet Pack SHALL expose industrial metadata and developer documentation

`pack.office.spreadsheet.v1` SHALL expose descriptor metadata for command schemas, permission scopes, policy templates, resource budgets, approval rules, redaction profiles, provider capability hashes, SDK examples, lifecycle state, compatibility, health probes, snapshots, unavailable diagnostics, and documentation links. The implementation SHALL include detailed developer documentation at `docs/developer-packs/office/spreadsheet.md`.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.office.spreadsheet.v1`
- **THEN** it SHALL return command namespace `spreadsheet.*`, command schemas, permissions, format support, worksheet support, range support, formula support, table/chart/pivot/filter/protection support, calculation support, export support, collaboration event support, examples, lifecycle, availability, health, diagnostics, compatibility metadata, redaction profiles, and documentation link
- **AND** examples SHALL use synthetic workbooks, sheets, ranges, formulas, charts, pivots, artifacts, and events rather than provider names, credentials, private workbook data, hidden sheet content, formulas with secrets, raw exports, or workflow-specific conventions

#### Scenario: Developer documentation is complete
- **WHEN** the pack implementation is marked complete
- **THEN** `docs/developer-packs/office/spreadsheet.md` SHALL document manifest declarations, required versus optional behavior, permissions, provider scopes, workbook handles, formats, worksheets, ranges, cells, values, formulas, named ranges, tables, charts, pivots, filters, sorts, validation, protection, calculations, update plans, export plans, artifacts, events, command DTOs, result DTOs, idempotency, pagination/streaming, timeout/cancellation, redaction, approvals, artifact retention, version preconditions, formula safety, calculation semantics, format compatibility, unavailable diagnostics, provider replacement, trace/audit interpretation, conformance tests, and supplier/API mapping
- **AND** the guide SHALL be linked from SDK discovery metadata and the industrial pack catalog index

### Requirement: Spreadsheet Pack SHALL be traceable, auditable, replayable, and sanitized

`pack.office.spreadsheet.v1` SHALL emit sanitized trace and audit events for declaration, admission, provider inspection, workbook create/import/open, worksheet listing, structure inspection, range reading, formula/table/chart/pivot inspection, update planning, update requests, recalculation planning, recalculation requests, export planning, export requests, event inspection, artifact handle resolution, policy decisions, service-call lifecycle, failures, unavailable states, and snapshots.

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a `pack.office.spreadsheet.v1` snapshot
- **THEN** the snapshot SHALL include descriptor version, provider capability hashes, workbook format/version hashes, calculation state hashes, command availability, provider health, policy template hash, resource counters, bounded workbook/sheet/range/formula/chart/pivot summaries, artifact summaries, event cursors, and sanitized replay pointers
- **AND** it SHALL exclude raw credentials, tokens, private financial data, hidden sheet content, raw formulas with secrets, raw exports, raw provider payloads, prompts, manifests, package bytes, private keys, signatures, and unbounded grid data

#### Scenario: Replay reconstructs command evidence
- **WHEN** replay inspects a past `spreadsheet.*` command
- **THEN** Macaca SHALL reconstruct descriptor version, command DTO hash, policy decision, resource decision, approval state, provider capability hash, workbook version hash, range anchor hash where applicable, formula dependency hash where applicable, calculation state hash where applicable, plan handle where applicable, artifact/event cursor where applicable, result classification, and sanitized provider class metadata
- **AND** replay SHALL NOT require raw provider payloads, raw workbook data, hidden sheet content, formulas with secrets, credentials, tokens, or application-specific workflow code

### Requirement: Spreadsheet implementation SHALL preserve Macaca boundaries

The `pack.office.spreadsheet.v1` implementation SHALL remain owned by spreadsheet service providers and service-runtime contracts. The microkernel, SDK, shells, and generic application framework SHALL remain provider-neutral and free of application-specific, supplier-specific, format-specific, workbook-specific, formula-specific, or workflow-specific routing branches.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, canonical execution-path, and serviceization gates scan the implementation
- **THEN** they SHALL find no concrete Google Sheets, Excel, Office.js, OpenXML, LibreOffice Calc, PDF, cloud-drive, OCR, conversion, credential-manager, artifact-provider, or provider-adapter imports in the microkernel, SDK helpers, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.office.spreadsheet.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, capability hashes, and bounded diagnostics rather than provider-specific business branches
