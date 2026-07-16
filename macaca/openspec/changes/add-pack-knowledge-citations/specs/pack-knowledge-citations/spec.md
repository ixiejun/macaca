## ADDED Requirements

### Requirement: Macaca SHALL provide Knowledge Citations Pack as a serviceized capability

Macaca SHALL provide `pack.knowledge.citations.v1` as a provider-neutral
industrial pack for citation creation, identifier resolution, source span
linking, source anchor inspection, citation verification, citation/bibliography
formatting, import/export, evidence provenance, and unavailable diagnostics.
Applications SHALL declare the pack in manifests, admission SHALL resolve it
into effective capabilities, and all operations SHALL run through typed service
commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.knowledge.citations.v1` as required and a citation service provider is registered, healthy, entitled, style-compatible, identifier-compatible, selector-compatible, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, provider capability metadata, permission scopes, policy templates, style/identifier support, verification depth, health, diagnostics, documentation links, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing raw credentials, raw provider payloads, raw source documents, raw private quotes, raw style files, or unbounded formatted output

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.knowledge.citations.v1` as required but provider, identifier support, style support, source access, permission, entitlement, resource budget, or policy support is absent
- **THEN** admission SHALL block readiness with structured unavailable, denied, unsupported, validation, conflict, or quota diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.knowledge.citations.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report
- **AND** SDK command helpers SHALL return Null Object unavailable diagnostics instead of creating callable service calls

### Requirement: Citation commands SHALL use typed canonical service calls

Every `pack.knowledge.citations.v1` operation SHALL be represented as a typed
command/result DTO and SHALL traverse the canonical service runtime path with
trace, policy, resource, entitlement, approval, health, snapshot, selector
validation, identifier validation, redaction, replay, and structured error
behavior.

#### Scenario: Citation is created
- **WHEN** `citations.create_citation` is invoked with bibliographic metadata, identifiers, contributor data, tags, or source handles
- **THEN** Macaca SHALL validate metadata shape, identifier schemes, source policy, idempotency key, and provider capability before creating the citation item
- **AND** it SHALL return a typed citation handle, metadata provenance, version hash, and sanitized replay evidence

#### Scenario: Source span is linked
- **WHEN** `citations.link_source_span` is invoked with a citation handle, source handle, text/page/fragment selector, quote policy, and source state hash
- **THEN** Macaca SHALL validate source access, selector compatibility, quote redaction, and source state before linking the anchor
- **AND** the result SHALL include a stable source anchor handle and replay pointer without exposing raw private source text beyond policy

#### Scenario: Citation is verified
- **WHEN** `citations.verify_citation` is invoked
- **THEN** Macaca SHALL check supported identifier reachability, source anchor stability, quote match, metadata freshness, license state, and provider capability according to policy
- **AND** it SHALL return separate verification statuses rather than a single fake success state

#### Scenario: Command is denied before provider call
- **WHEN** policy, permission, entitlement, source access, selector validation, style validation, approval, or resource checks reject a citation command
- **THEN** Macaca SHALL return a typed denied, validation, conflict, or quota result before invoking the concrete provider
- **AND** audit evidence SHALL include a bounded reason code without raw source documents, raw private quotes, raw provider payloads, credentials, raw style files, or unbounded formatted output

### Requirement: Citation DTOs SHALL model citation items, identifiers, contributors, source anchors, selectors, evidence, styles, formatted output, and verification

`pack.knowledge.citations.v1` SHALL define portable DTOs for citation items,
identifiers, contributors, source anchors, selectors, evidence links,
bibliography styles, formatted citations, verification results, import/export
results, provider capability, and diagnostics. Provider-specific fields SHALL
remain bounded adapter metadata and SHALL NOT become OS-layer routing branches.

#### Scenario: Developer inspects citation item schema
- **WHEN** SDK schemas expose `CitationItem`
- **THEN** the schema SHALL include item type, title, contributors, issued date, publisher/container, edition/version, identifiers, URL handle, license, tags, notes handle, source anchors, metadata provenance, and version hash
- **AND** raw source documents, raw private notes, credentials, and raw provider payloads SHALL NOT be exposed

#### Scenario: Developer formats bibliography
- **WHEN** `citations.format_bibliography` is invoked with citation handles, style handle, locale, ordering policy, and output format
- **THEN** Macaca SHALL validate style support, output size, locale compatibility, and redaction policy
- **AND** it SHALL return bounded formatted output handles with warnings for unsupported item fields or style features

#### Scenario: Provider reports capability limits
- **WHEN** SDK discovery inspects the active citation provider
- **THEN** Macaca SHALL report identifier schemes, metadata enrichment, verification depth, style rendering, import/export formats, selector support, max items, rate limits, lifecycle, and health
- **AND** callers SHALL use this metadata rather than provider-name branches

### Requirement: Citation Pack SHALL enforce permissions, source access, redaction, and verification semantics

`pack.knowledge.citations.v1` SHALL define permission scopes for citation
creation, reading, updating, source linking, identifier resolution, verification,
formatting, import/export, and evidence reading. Policy SHALL run before side
effects and SHALL account for source access, selector validation, identifier
scheme support, network resolver policy, style capability, output limits,
metadata freshness, and approval.

#### Scenario: Missing source permission blocks source span linking
- **WHEN** an application can create citations but lacks source access for a document, page, URL, message, dataset, or code span
- **THEN** Macaca SHALL return a typed denied result and SHALL NOT create a source anchor
- **AND** trace/audit evidence SHALL identify the missing scope by stable code

#### Scenario: Private quote is redacted
- **WHEN** a citation includes a quote selector or source snippet from private material
- **THEN** Macaca SHALL store only handles, hashes, selectors, and redacted quote metadata according to policy
- **AND** raw private quote text SHALL NOT enter traces, audits, snapshots, SDK diagnostics, or examples

#### Scenario: Verification status is decomposed
- **WHEN** citation verification runs
- **THEN** Macaca SHALL separately report identifier status, source anchor status, quote match status, metadata freshness, license status, confidence, and diagnostics
- **AND** consumers SHALL NOT infer truth of a claim from identifier reachability alone

### Requirement: Citation Pack SHALL expose industrial metadata and developer documentation

`pack.knowledge.citations.v1` SHALL expose descriptor metadata for identifier
schemes, style support, selector support, import/export formats, command
schemas, permission scopes, policy templates, verification depth, resource
budgets, SDK examples, lifecycle state, compatibility, health probes, snapshots,
unavailable diagnostics, redaction profiles, and developer documentation.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.knowledge.citations.v1`
- **THEN** it SHALL return command namespace `citations.*`, provider capabilities, supported commands, permissions, policy templates, identifier schemes, style support, selector support, import/export formats, examples, lifecycle, availability, health, diagnostics, compatibility, redaction profile, and documentation links
- **AND** examples SHALL use generic handles and synthetic data rather than application-specific workflows, provider names, credentials, raw source documents, or business routing

#### Scenario: Developer documentation is published
- **WHEN** the pack implementation is marked complete
- **THEN** `docs/developer-packs/knowledge/citations.md` SHALL document manifest declaration, permissions, citation metadata, identifiers, contributors, CSL-compatible data, source anchors, W3C-style selectors, quote policies, bibliography styles, verification statuses, import/export, provider replacement, unavailable diagnostics, trace/audit interpretation, and operational limits
- **AND** SDK discovery metadata and the industrial catalog index SHALL link to that guide

### Requirement: Citation Pack SHALL be traceable, auditable, replayable, and sanitized

`pack.knowledge.citations.v1` SHALL emit sanitized trace/audit events and
bounded snapshots for declaration, admission, citation creation, identifier
resolution, source span linking, verification, formatting, import/export, anchor
inspection, policy/resource decisions, provider calls, unavailable states, and
replay.

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a citation pack snapshot
- **THEN** the snapshot SHALL include descriptor version, provider capability hashes, identifier scheme support, style capability hashes, import/export support, verification status aggregates, source-anchor counts, provider health, command availability, policy template hash, resource counters, and sanitized replay pointers
- **AND** it SHALL exclude raw source documents, raw private quotes, raw provider payloads, credentials, raw style files, and unbounded formatted output

#### Scenario: Citation verification is audited
- **WHEN** identifier resolution, source anchor inspection, quote match, metadata freshness, or license checks run
- **THEN** Macaca SHALL emit a sanitized audit event with stable citation handle, source anchor handle, identifier scheme, verification status codes, policy decision, provider capability hash, result code, and replay pointer
- **AND** the event SHALL exclude raw provider payloads and raw private source text

### Requirement: Citation implementation SHALL preserve Macaca boundaries

The `pack.knowledge.citations.v1` implementation SHALL remain owned by citation
service providers behind the service runtime. The microkernel, SDK, shells, and
generic application framework SHALL remain provider-neutral and free of
application-specific or provider-specific routing branches.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete citation provider, style engine, or identifier resolver imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.knowledge.citations.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, capability hash, and result codes rather than provider-specific business branches
