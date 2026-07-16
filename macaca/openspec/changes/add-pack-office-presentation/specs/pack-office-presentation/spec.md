## ADDED Requirements

### Requirement: Macaca SHALL expose Office Presentation as a serviceized industrial pack

Macaca SHALL expose `pack.office.presentation.v1` as a provider-neutral pack for
deck creation, deck import, deck opening, slide listing, structure inspection,
slide inspection, asset inspection, speaker notes, review metadata, edit
planning, edit requests, export planning, export requests, collaboration event
inspection, artifact handles, health, snapshots, and replay diagnostics. The
pack SHALL be declared by applications, resolved by catalog/admission services,
and invoked only through descriptor-owned `presentation.*` service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.office.presentation.v1` as required and a presentation provider is registered, healthy, entitled, permissioned, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, permission scopes, policy template hash, resource limits, approval rules, health metadata, compatibility metadata, and replay metadata
- **AND** SDK discovery SHALL expose callable `presentation.*` commands without leaking credentials, private deck content, raw slide trees, raw notes, raw comments, raw media, raw exports, raw provider payloads, or provider secrets

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.office.presentation.v1` as required but provider registration, host support, credential reference, permission, entitlement, resource, policy, or approval prerequisites are absent
- **THEN** admission SHALL block readiness with typed unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, contact a concrete provider, mutate a deck, export an artifact, notify collaborators, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.office.presentation.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability memento
- **AND** SDK helpers and WASM ABI descriptors SHALL mark unavailable commands as non-callable while preserving structured diagnostics for application recovery

### Requirement: Office Presentation commands SHALL use typed canonical service calls

Every `pack.office.presentation.v1` operation SHALL be represented as a typed
command/result DTO and SHALL traverse the canonical service runtime path with
trace context, policy, resource, entitlement, approval, lifecycle, health,
snapshot, structured error, and audit behavior. SDK helpers, WASM ABI handlers,
application admission, web, CLI, and frontend code SHALL only build or submit
canonical service calls and SHALL NOT call presentation providers directly.

#### Scenario: Provider capability is inspected
- **WHEN** `presentation.inspect_provider` is invoked with declared scope and trace context
- **THEN** Macaca SHALL return sanitized provider capability metadata for deck, slide, shape, layout, master, theme, text, table, media, notes, review, animation, transition, export, event, auth, quota, lifecycle, health, and compatibility support
- **AND** the result SHALL include typed unavailable, unsupported, degraded, retired, format-limited, notes-limited, media-limited, animation-limited, export-limited, collaboration-limited, network-limited, and quota-limited states when applicable

#### Scenario: Deck and slide reads use bounded projections
- **WHEN** `presentation.open_deck`, `presentation.list_slides`, `presentation.inspect_structure`, `presentation.inspect_slide`, `presentation.inspect_assets`, `presentation.inspect_notes`, `presentation.inspect_reviews`, `presentation.inspect_events`, or `presentation.get_artifact_handle` is invoked
- **THEN** Macaca SHALL enforce deck, slide, shape, asset, notes, review, event, artifact, permission, resource, and redaction scopes before provider access
- **AND** results SHALL be bounded, paged or partial when needed, redacted according to policy, and represented by handles and summaries rather than raw deck bodies or unbounded slide trees

#### Scenario: Unsupported command is requested
- **WHEN** a descriptor exists but the active provider does not support the requested `presentation.*` command, format, slide feature, notes feature, review feature, animation feature, transition feature, export format, or artifact mode
- **THEN** Macaca SHALL return a typed unsupported or format-unsupported result with descriptor and capability diagnostics
- **AND** SDK discovery SHALL report the command or feature as non-callable for the current effective capability set

### Requirement: Office Presentation DTOs SHALL be provider-neutral and hash-stable

`pack.office.presentation.v1` SHALL define provider-neutral DTOs for
`PresentationScope`, `PresentationProviderCapability`, `DeckHandle`,
`SlideHandle`, `SlideLayout`, `SlideMaster`, `PresentationTheme`,
`PresentationShape`, `PresentationTextRange`, `PresentationTable`,
`PresentationMedia`, `PresentationNotes`, `PresentationReviewEvent`,
`PresentationAnimation`, `PresentationTransition`,
`PresentationEditOperation`, `PresentationEditPlan`,
`PresentationExportPlan`, and `PresentationArtifactHandle`. DTOs SHALL use
stable handles, version hashes, compatibility hashes, redaction classes,
sensitivity classes, capability hashes, event cursors, and artifact handles
rather than provider object references as OS-layer semantics.

#### Scenario: Provider-specific concepts are mapped
- **WHEN** a provider exposes Google Slides page elements, PowerPoint shapes, OpenXML slide parts, Microsoft 365 drive items, or another provider-specific presentation object
- **THEN** the provider adapter SHALL map those concepts into Macaca provider-neutral DTOs
- **AND** provider-specific extensions SHALL appear only as bounded `adapter_metadata` protected by capability hashes and SHALL NOT drive OS-layer routing

#### Scenario: Hashes preserve compatibility and replay
- **WHEN** Macaca serializes descriptors, provider capabilities, deck formats, deck versions, slide anchors, shape anchors, layouts, masters, themes, media assets, edit plans, export plans, artifact handles, event cursors, and redaction profiles
- **THEN** it SHALL produce stable hashes suitable for compatibility checks, stale-version detection, audit correlation, and replay diagnostics
- **AND** schema evolution tests SHALL prove older compatible snapshots remain readable or return typed schema-mismatch diagnostics

### Requirement: Office Presentation writes SHALL use plan/request separation

Mutating or externally visible operations SHALL be split into non-mutating plan
commands and side-effecting request commands. `presentation.plan_edit` and
`presentation.plan_export` SHALL validate operations, versions, format support,
notification policy, resource use, redaction, approvals, and idempotency before
`presentation.edit_request` or `presentation.export_request` can perform side
effects.

#### Scenario: Edit plan validates a batch before mutation
- **WHEN** `presentation.plan_edit` receives deck, slide, shape, text, table, media, notes, animation, transition, layout, master, or theme operations
- **THEN** Macaca SHALL validate operation schema, target handles, deck version hash, slide and shape anchor freshness, format compatibility, provider support, resource budget, notification policy, and required approvals
- **AND** it SHALL return a `PresentationEditPlan` with validation diagnostics without mutating the deck or notifying collaborators

#### Scenario: Edit request executes a validated plan
- **WHEN** `presentation.edit_request` is invoked with a valid plan handle, idempotency key, trace context, audit reason, current version preconditions, granted approval state, and sufficient permissions
- **THEN** Macaca SHALL execute the batch through the presentation service provider and return typed success, partial, conflict, stale-version, write-denied, approval-required, quota, timeout, cancellation, or failure results
- **AND** repeated requests with the same idempotency key SHALL NOT duplicate side effects

#### Scenario: Export request executes a validated plan
- **WHEN** `presentation.export_request` is invoked with a valid export plan, retention policy, redaction profile, artifact scope, idempotency key, and approval state
- **THEN** Macaca SHALL generate or request only a bounded `PresentationArtifactHandle`
- **AND** raw exported bytes SHALL remain in the artifact boundary and SHALL NOT enter trace, audit, snapshots, SDK diagnostics, or examples

### Requirement: Office Presentation SHALL enforce permission, policy, resource, entitlement, and approval gates

Every `presentation.*` command SHALL be scoped to application id, tenant id,
session id, task id, trace id, provider scope, deck handle, slide or shape
handle when applicable, actor handle when available, credential reference,
network policy, artifact policy, and permission state. Side-effecting commands
SHALL run policy, resource, entitlement, approval, version, and idempotency
checks before concrete provider calls.

#### Scenario: Permission is denied before provider access
- **WHEN** an application lacks `presentation.provider.inspect`, `presentation.deck.create`, `presentation.deck.import`, `presentation.deck.open`, `presentation.slide.read`, `presentation.structure.read`, `presentation.asset.read`, `presentation.notes.read`, `presentation.review.read`, `presentation.deck.write`, `presentation.asset.write`, `presentation.notes.write`, `presentation.export`, `presentation.events.read`, or `presentation.artifact.read`
- **THEN** Macaca SHALL return a typed denied result before invoking any provider
- **AND** audit evidence SHALL include bounded reason codes and sanitized scope handles only

#### Scenario: Sensitive operation requires approval
- **WHEN** a command touches private decks, speaker notes, comments, customer screenshots, embedded media, unreleased branding, collaborator-visible edits, destructive edits, exports, media insertion, notes writes, or operations that notify collaborators or external systems
- **THEN** Macaca SHALL require approval when policy marks the operation approval-gated
- **AND** denial, expiration, or missing approval SHALL return typed approval-required diagnostics without side effects

#### Scenario: Resource or entitlement is unavailable
- **WHEN** deck size, slide count, shape count, media count, media bytes, notes count, review count, edit operation count, export size, artifact size, provider quota, network transfer, timeout, memory, storage, streaming output, retained snapshots, entitlement, or host support is insufficient
- **THEN** Macaca SHALL return typed quota, unavailable, denied, timeout, cancellation, or host-resource diagnostics
- **AND** the provider SHALL NOT be called for side-effecting operations after a failed gate

### Requirement: Office Presentation artifacts, notes, reviews, media, and events SHALL be bounded and redacted

`pack.office.presentation.v1` SHALL treat speaker notes, comments, review
events, embedded media, thumbnails, exported files, linked assets, and
collaboration events as sensitive data. The pack SHALL expose handles, bounded
summaries, cursors, redaction classes, retention metadata, and replay pointers
rather than raw sensitive payloads in observability surfaces.

#### Scenario: Speaker notes and reviews are inspected
- **WHEN** `presentation.inspect_notes` or `presentation.inspect_reviews` is invoked with sufficient permission
- **THEN** Macaca SHALL return paged or bounded notes/review DTOs with author/source metadata redacted according to policy
- **AND** raw private notes, raw comments, personal data, customer content, and unbounded review threads SHALL NOT enter traces, audits, snapshots, or SDK diagnostics

#### Scenario: Media and artifact metadata is inspected
- **WHEN** `presentation.inspect_assets` or `presentation.get_artifact_handle` is invoked
- **THEN** Macaca SHALL return media/artifact kind, content type, size class, checksum handle, retention state, sensitivity class, and redaction class
- **AND** raw media bytes, thumbnails, exported decks, rendered images, and linked asset payloads SHALL remain behind artifact boundaries

#### Scenario: Collaboration events are inspected
- **WHEN** `presentation.inspect_events` reads collaboration or change events from a capable provider
- **THEN** Macaca SHALL return bounded event cursors, event kinds, affected handle references, changed field classes, sanitized actor handles, timestamps, and redaction classes
- **AND** provider-specific event payloads SHALL NOT become OS-layer event semantics

### Requirement: Office Presentation SHALL preserve Macaca architecture boundaries

The Office Presentation pack implementation SHALL preserve the microkernel,
service runtime, SDK/SystemFacade, application framework, runtime-host, plugin,
and shell boundaries defined by Macaca governance. Concrete presentation
providers SHALL be replaceable Strategy adapters created only in approved
runtime-host or plugin composition roots.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, serviceization, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete Google Slides, PowerPoint, Office.js, OpenXML, LibreOffice Impress, cloud-drive, conversion, rendering, credential, or artifact provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.office.presentation.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract, permission model, trace/audit schema, snapshot shape, and structured unavailable behavior
- **AND** OS layers SHALL NOT branch on provider names, deck names, slide titles, layout names, theme names, file names, application names, or workflow names

### Requirement: Office Presentation SHALL emit sanitized trace, audit, health, snapshot, and replay evidence

`pack.office.presentation.v1` SHALL emit sanitized declaration, admission,
provider-inspection, deck-create, deck-import, deck-open, slide-list,
structure-inspection, slide-inspection, asset-inspection, notes-inspection,
review-inspection, edit-plan, edit-request, export-plan, export-request,
event-inspection, artifact-handle, policy, entitlement, resource, approval,
health, snapshot, unavailable, and failure events. Snapshots SHALL contain
enough bounded metadata to diagnose and replay service behavior without storing
raw sensitive content.

#### Scenario: Service call evidence is recorded
- **WHEN** any `presentation.*` command is submitted
- **THEN** Macaca SHALL record trace-required service-call evidence with command name, descriptor version, sanitized scope handles, policy decision, resource decision, provider capability hash, result class, and replay pointer
- **AND** the evidence SHALL exclude raw credentials, tokens, private notes, comments, customer data, raw media, raw exports, raw provider payloads, prompts, manifests, package bytes, private keys, signatures, and unbounded slide trees

#### Scenario: Snapshot supports recovery diagnostics
- **WHEN** the service runtime records a presentation snapshot
- **THEN** the snapshot SHALL include descriptor version, provider capability hashes, deck format and version hashes, command availability, provider health, policy template hash, resource counters, bounded deck/slide/shape/notes/media summaries, artifact summaries, event cursors, and sanitized replay pointers
- **AND** replay tests SHALL prove every `presentation.*` command can be correlated through the canonical service path after restart

### Requirement: Office Presentation SHALL provide industrial developer documentation

The implementation SHALL include a detailed developer guide at
`docs/developer-packs/office/presentation.md` before `pack.office.presentation.v1`
is marked complete. The guide SHALL be linked from SDK discovery metadata and
the industrial pack catalog index.

#### Scenario: Developer reads the guide
- **WHEN** a developer opens `docs/developer-packs/office/presentation.md`
- **THEN** the guide SHALL explain purpose, manifest declaration, required versus optional behavior, permissions, provider scopes, deck handles, formats, slides, layouts, masters, themes, shapes, text ranges, tables, media, notes, comments/reviews, animations, transitions, edit plans, export plans, artifacts, events, unavailable diagnostics, provider replacement, operational limits, and conformance expectations
- **AND** it SHALL document every command DTO and result DTO with field-level behavior, idempotency, redaction, pagination, streaming, timeout, cancellation, approval, artifact retention, version preconditions, format compatibility, media policy, structured errors, and trace/audit interpretation

#### Scenario: Supplier mapping is documented
- **WHEN** the documentation describes supplier/API mapping
- **THEN** it SHALL map Google Slides API presentations/pages/page elements/layouts/masters/notes/thumbnails/batch updates, PowerPoint JavaScript presentation/slides/shapes/tables/text host APIs, OpenXML PresentationML packages/slides/masters/layouts/notes/comments/transitions/animations/themes/media parts, and Microsoft 365 file/permission concepts to Macaca abstractions
- **AND** it SHALL explicitly document what is intentionally not exposed as OS semantics

#### Scenario: Examples are provided
- **WHEN** the guide provides examples
- **THEN** examples SHALL use only synthetic decks, slides, shapes, media, notes, review events, artifacts, and unavailable diagnostics
- **AND** examples SHALL NOT include provider names, real credentials, private notes, customer data, raw media, raw exports, or workflow-specific conventions
