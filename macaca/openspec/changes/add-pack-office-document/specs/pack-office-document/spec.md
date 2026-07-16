## ADDED Requirements

### Requirement: Macaca SHALL provide the Office Document Pack as a serviceized capability

Macaca SHALL provide `pack.office.document.v1` as a provider-neutral industrial pack for document creation, import, opening, structural inspection, range reading, style inspection, comment inspection and writing, revision inspection and redline operations, batch edit planning and requests, revision resolution planning and requests, export planning and requests, collaboration event inspection, artifact handles, snapshots, and replay diagnostics. The pack SHALL be declared by applications, resolved by admission/catalog services, and invoked only through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.office.document.v1` as required and office document service provider is registered, healthy, entitled, document-scoped, format-capable, and policy-admissible
- **THEN** admission SHALL expose `pack.office.document.v1` in the effective capability set with command schemas, permission scopes, document/format scope metadata, policy template hash, provider capability hash, health, and replay metadata
- **AND** SDK discovery SHALL mark callable `document.*` commands as available without exposing provider secrets, raw credentials, private comments, full document text, raw embedded media, raw exports, raw provider payloads, or application-specific workflow names

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.office.document.v1` as required but provider, format support, credential reference, document permission, entitlement, resource, approval, network, host support, or policy admission is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, mutate documents, comment, redline, accept revisions, export documents, notify collaborators, contact a provider, or fake success

#### Scenario: Optional declaration degrades explicitly
- **WHEN** an application declares `pack.office.document.v1` as optional and the pack or a sub-capability is unavailable
- **THEN** admission SHALL produce a degraded effective capability memento naming unavailable commands and bounded reason codes
- **AND** SDK command helpers SHALL refuse to build callable service calls for unavailable commands while preserving discoverability and diagnostics

### Requirement: Document commands SHALL use typed canonical service calls

Every `pack.office.document.v1` operation SHALL be represented as a typed command/result DTO and SHALL traverse the canonical service runtime path with trace, policy, resource, entitlement, approval, health, snapshot, and structured error behavior. SDK, WASM ABI, shell, and application-framework helpers SHALL only build canonical service commands and SHALL NOT construct concrete document providers or call document APIs directly.

#### Scenario: Read command succeeds through service runtime
- **WHEN** a declared and policy-allowed command such as `document.open_document`, `document.inspect_structure`, `document.read_range`, or `document.inspect_comments` is invoked
- **THEN** Macaca SHALL route the command through SDK/facade helpers into the service runtime and office document service provider
- **AND** it SHALL emit sanitized admission, policy, service-call, result, and replay events with stable trace identifiers and bounded projection metadata

#### Scenario: Batch edit or export is planned before request
- **WHEN** an application wants to edit, redline, resolve revisions, comment, or export a document
- **THEN** Macaca SHALL require the applicable typed request or planning command with document/range validation, format compatibility, version preconditions, notification policy, artifact policy, resource reservation, idempotency key, approval state where required, and provider capability validation
- **AND** planning commands SHALL be replay-addressable and SHALL NOT mutate documents, comment, redline, export, or notify collaborators

#### Scenario: Command is denied before provider invocation
- **WHEN** policy, permission, entitlement, approval, resource, quota, document scope, range anchor, format, schema, version, revision support, export, artifact, provider capability, or timeout checks reject a `document.*` command
- **THEN** Macaca SHALL return a typed denied, validation, conflict, stale-version, schema-mismatch, format-unsupported, export-denied, write-denied, revision-unsupported, approval-required, quota, timeout, unavailable, or unsupported result before invoking the concrete provider
- **AND** the audit trail SHALL include only bounded reason codes and sanitized handles

### Requirement: Document DTOs SHALL model provider-neutral rich document concepts

`pack.office.document.v1` SHALL define provider-neutral DTOs for document scope, provider capability, document handle, document structure, range, paragraph, run, table, style, comment, revision, edit operation, edit plan, export plan, artifact handle, collaboration event, version/freshness metadata, and diagnostics. Provider-specific fields SHALL be exposed only as bounded `adapter_metadata` guarded by capability hashes and SHALL NOT drive OS-layer routing branches.

#### Scenario: Provider capability is inspected
- **WHEN** `document.inspect_provider` is invoked for a provider or document scope
- **THEN** Macaca SHALL return provider-neutral `DocumentProviderCapability` metadata for create/open/import support, structure support, range support, style support, table/list support, comment support, revision support, export support, collaboration event support, auth modes, rate limits, lifecycle, health, and compatibility
- **AND** it SHALL include stable descriptor, provider capability, policy template, and compatibility hashes for validation and replay

#### Scenario: Structure is inspected
- **WHEN** `document.inspect_structure` returns document structure
- **THEN** the result SHALL use `DocumentStructure`, section summaries, heading outline, body summary, table/list/style/comment/revision summaries, version hash, projection metadata, and redaction class
- **AND** it SHALL NOT expose raw full document bodies, raw embedded media, private comments, personal data, raw provider payloads, or unbounded document trees

#### Scenario: Provider-specific capability exists
- **WHEN** an active provider supports a document concept not present in the canonical DTO model
- **THEN** the provider MAY expose bounded `adapter_metadata` and compatibility diagnostics through `DocumentProviderCapability`
- **AND** the OS, SDK, shell, and generic application framework SHALL NOT branch on provider names, document names, templates, styles, formats, or business document workflows

### Requirement: Edits, comments, redlines, revisions, and exports SHALL be version-safe, approval-aware, and auditable

All document side effects SHALL use typed requests or plan/request separation, document/range scope validation, range anchor freshness, format compatibility, provider capability validation, resource reservations, idempotency, version preconditions, collaboration notification policy, approval gates where required, and sanitized audit.

#### Scenario: Batch edit is requested
- **WHEN** `document.plan_edit` validates edit operations, ranges, styles, tables, lists, comments, version preconditions, format compatibility, notification policy, quota, and approvals
- **THEN** `document.edit_request` MAY use the validated plan handle and idempotency key to request edits
- **AND** Macaca SHALL record sanitized plan, request, document version hash, range anchor hash, provider capability hash, policy decision, audit reason, result handles, and replay pointer

#### Scenario: Comment or redline is requested
- **WHEN** `document.comment_request` or `document.redline_request` is invoked
- **THEN** Macaca SHALL validate comment/revision permission, visibility, range anchors, provider support, notification policy, content bounds, version preconditions, and approval requirements
- **AND** traces, audits, snapshots, and SDK diagnostics SHALL use sanitized handles or bounded summaries rather than raw private comment or tracked-change content

#### Scenario: Revision resolution detects stale version
- **WHEN** `document.revision_resolution_request` receives a plan whose document version hash or revision handle no longer matches provider state
- **THEN** Macaca SHALL return a typed stale-version or revision-unsupported result
- **AND** it SHALL NOT accept or reject revisions unless version preconditions and provider support are valid

#### Scenario: Export is requested
- **WHEN** `document.plan_export` validates document/range/page scope, output format, rendering profile, retention, sensitivity, resource budget, and approvals
- **THEN** `document.export_request` MAY request export through the service provider
- **AND** it SHALL return bounded `DocumentArtifactHandle` metadata rather than raw document exports in traces, audits, snapshots, examples, or diagnostics

### Requirement: Document content, comments, revisions, events, and artifacts SHALL be bounded and policy-controlled

`pack.office.document.v1` SHALL treat full document text, comments, revisions, embedded media, exports, collaboration events, personal data, and provider payloads as policy-controlled resources with explicit permissions, quotas, redaction, retention, and provider capability checks.

#### Scenario: Range is read
- **WHEN** `document.read_range` is invoked
- **THEN** Macaca SHALL validate range scope, range anchor freshness, content bounds, sensitivity, redaction, resource budget, and provider capability
- **AND** it SHALL return bounded range content handles or sanitized snippets rather than unbounded full document text

#### Scenario: Collaboration events are inspected
- **WHEN** `document.inspect_events` is invoked
- **THEN** Macaca SHALL return bounded `DocumentCollaborationEvent` records with event kind, actor handle, timestamp, changed fields, comment/revision handle, cursor, and redaction class
- **AND** it SHALL enforce event permission, page size, redaction, retention, timeout, and replay bounds

#### Scenario: Artifact handle is resolved
- **WHEN** `document.get_artifact_handle` is invoked
- **THEN** Macaca SHALL validate artifact permission, source document/range handle, content type, size class, retention, redaction class, provider capability, resource budget, and approval requirements
- **AND** it SHALL return bounded artifact metadata rather than raw exports, embedded media, or raw provider payloads

### Requirement: Document Pack SHALL enforce permissions, scopes, resources, entitlements, approvals, and redaction

`pack.office.document.v1` SHALL enforce explicit permission scopes for provider inspection, create, import, open, structure reading, range reading, style reading, comment reading, comment writing, revision reading, revision writing, editing, export, event reading, and artifact reading. Every command SHALL carry application id, tenant id, session id, task id, trace id, provider scope, document handle, range handle where applicable, and actor handle when available.

#### Scenario: Permission is missing
- **WHEN** an application invokes a `document.*` command without the required permission scope
- **THEN** Macaca SHALL return a typed denied result before provider invocation
- **AND** the denied result SHALL identify the missing permission scope using sanitized identifiers

#### Scenario: Resource budget is exceeded
- **WHEN** structure inspection, range reading, comments, revisions, edit planning, export, event inspection, or artifact retrieval exceeds document size, range text size, structure depth, paragraph/table/list count, comment count, revision count, edit operation count, export size, artifact size, provider quota, network transfer, timeout, memory, storage, or snapshot retention budgets
- **THEN** Macaca SHALL return typed quota, timeout, cancellation, export-denied, artifact-denied, or resource-denied diagnostics
- **AND** it SHALL preserve replayable audit evidence without raw document text, raw embedded media, or provider payloads

#### Scenario: Sensitive operation requires approval
- **WHEN** policy marks private documents, contracts, personal data, comments, revisions, embedded media, collaborator-visible edits, destructive edits, exports, revision accept/reject, or external notifications as approval-required
- **THEN** Macaca SHALL return an approval-required result until a valid approval token is supplied
- **AND** no document mutation, comment, redline, revision resolution, export, collaborator notification, or raw artifact retrieval SHALL happen before approval

### Requirement: Document Pack SHALL expose industrial metadata and developer documentation

`pack.office.document.v1` SHALL expose descriptor metadata for command schemas, permission scopes, policy templates, resource budgets, approval rules, redaction profiles, provider capability hashes, SDK examples, lifecycle state, compatibility, health probes, snapshots, unavailable diagnostics, and documentation links. The implementation SHALL include detailed developer documentation at `docs/developer-packs/office/document.md`.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.office.document.v1`
- **THEN** it SHALL return command namespace `document.*`, command schemas, permissions, format support, structure support, range support, style support, comment support, revision support, export support, collaboration event support, examples, lifecycle, availability, health, diagnostics, compatibility metadata, redaction profiles, and documentation link
- **AND** examples SHALL use synthetic documents, ranges, comments, revisions, artifacts, and events rather than provider names, credentials, private comments, personal data, full document text, raw exports, or workflow-specific conventions

#### Scenario: Developer documentation is complete
- **WHEN** the pack implementation is marked complete
- **THEN** `docs/developer-packs/office/document.md` SHALL document manifest declarations, required versus optional behavior, permissions, provider scopes, document handles, formats, structures, sections, paragraphs, runs, tables, lists, ranges, styles, comments, revisions, edit plans, export plans, artifacts, events, command DTOs, result DTOs, idempotency, pagination/streaming, timeout/cancellation, redaction, approvals, artifact retention, version preconditions, format compatibility, unavailable diagnostics, provider replacement, trace/audit interpretation, conformance tests, and supplier/API mapping
- **AND** the guide SHALL be linked from SDK discovery metadata and the industrial pack catalog index

### Requirement: Document Pack SHALL be traceable, auditable, replayable, and sanitized

`pack.office.document.v1` SHALL emit sanitized trace and audit events for declaration, admission, provider inspection, create/import/open, structure inspection, range reading, style inspection, comment inspection, revision inspection, edit planning, edit requests, comment requests, redline requests, revision resolution planning, revision resolution requests, export planning, export requests, event inspection, artifact handle resolution, policy decisions, service-call lifecycle, failures, unavailable states, and snapshots.

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a `pack.office.document.v1` snapshot
- **THEN** the snapshot SHALL include descriptor version, provider capability hashes, document format/version hashes, command availability, provider health, policy template hash, resource counters, bounded document/range/comment/revision summaries, artifact summaries, event cursors, and sanitized replay pointers
- **AND** it SHALL exclude raw credentials, tokens, private comments, personal data, raw full document text, raw embedded media, raw exports, raw provider payloads, prompts, manifests, package bytes, private keys, signatures, and unbounded document trees

#### Scenario: Replay reconstructs command evidence
- **WHEN** replay inspects a past `document.*` command
- **THEN** Macaca SHALL reconstruct descriptor version, command DTO hash, policy decision, resource decision, approval state, provider capability hash, document version hash, range anchor hash where applicable, plan handle where applicable, artifact/event cursor where applicable, result classification, and sanitized provider class metadata
- **AND** replay SHALL NOT require raw provider payloads, raw document text, embedded media, private comments, credentials, tokens, or application-specific workflow code

### Requirement: Document implementation SHALL preserve Macaca boundaries

The `pack.office.document.v1` implementation SHALL remain owned by office document service providers and service-runtime contracts. The microkernel, SDK, shells, and generic application framework SHALL remain provider-neutral and free of application-specific, supplier-specific, format-specific, template-specific, style-specific, or workflow-specific routing branches.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, canonical execution-path, and serviceization gates scan the implementation
- **THEN** they SHALL find no concrete Word, Google Docs, OpenXML, LibreOffice, PDF, cloud-drive, OCR, conversion, credential-manager, artifact-provider, or provider-adapter imports in the microkernel, SDK helpers, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.office.document.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, capability hashes, and bounded diagnostics rather than provider-specific business branches
