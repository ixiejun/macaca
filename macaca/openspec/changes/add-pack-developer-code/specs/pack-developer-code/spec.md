## ADDED Requirements

### Requirement: Macaca SHALL provide Developer Code Pack as a serviceized capability

Macaca SHALL provide `pack.developer.code.v1` as a provider-neutral industrial
pack for workspace inspection, source indexing, syntax parsing, symbol lookup,
references, diagnostics, code actions, edit planning, patch generation, patch
validation, patch application requests, diff inspection, impact analysis, test
suggestion, scan result import, scan finding inspection, provider capability
inspection, and unavailable diagnostics. Applications SHALL declare the pack in
manifests, admission SHALL resolve it into effective capabilities, and all
operations SHALL run through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.developer.code.v1` as required and a code intelligence service provider is registered, healthy, entitled, workspace-compatible, language-compatible, feature-compatible, quota-compatible, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, language support, parser support, LSP-style feature support, scan support, patch support, permission scopes, policy templates, resource limits, health, diagnostics, documentation links, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing credentials, raw source files, raw patches, raw diffs, raw scan payloads, raw provider payloads, raw manifests, package bytes, private keys, signatures, or unbounded diagnostics

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.developer.code.v1` as required but provider, workspace trust, path access, language support, parser support, scanner support, patch support, permission, entitlement, approval, resource budget, or host support is absent
- **THEN** admission SHALL block readiness with structured unavailable, denied, unsupported, validation, stale-index, approval-required, conflict, quota, timeout, or failure diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, instantiate another provider implicitly, mutate files, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.developer.code.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report
- **AND** SDK command helpers SHALL return Null Object unavailable diagnostics instead of creating callable service calls

### Requirement: Code commands SHALL use typed canonical service calls

Every `pack.developer.code.v1` operation SHALL be represented as a typed
command/result DTO and SHALL traverse the canonical service runtime path with
trace, policy, workspace/path scope checks, resource, entitlement, approval,
health, snapshot, redaction, replay, and structured error behavior.

#### Scenario: Workspace is inspected
- **WHEN** `code.inspect_workspace` is invoked with a workspace handle and requested inventory depth
- **THEN** Macaca SHALL validate workspace trust, declared root scope, path exclusions, permission, entitlement, and resource budget before provider access
- **AND** it SHALL return bounded workspace inventory, language inventory, index state, file counts, excluded path diagnostics, health, and replay pointer

#### Scenario: Symbols are found
- **WHEN** `code.find_symbols` is invoked with query filters, language, path scope, symbol kind, and result limits
- **THEN** Macaca SHALL validate symbol permission, language support, index state, path scope, result limit, and provider capability
- **AND** it SHALL return bounded symbol pages with stable symbol handles, ranges, confidence, stale-index warnings, and replay pointers

#### Scenario: Diagnostics are returned
- **WHEN** `code.get_diagnostics` is invoked for workspace, document, range, or scan scope
- **THEN** Macaca SHALL validate diagnostic permission, path scope, scanner/language capability, redaction, and resource limits
- **AND** it SHALL return typed diagnostics with severity, source, rule/code, message handles, ranges, related ranges, fix availability, taxonomy, and baseline status

#### Scenario: Command is denied before provider call
- **WHEN** policy, workspace trust, path scope, permission, entitlement, approval, resource, language support, redaction, current content hash, or patch validation checks reject a `code.*` command
- **THEN** Macaca SHALL return a typed denied, approval-required, validation, stale-index, conflict, quota, timeout, unavailable, or unsupported result before invoking the concrete provider or mutating files
- **AND** audit evidence SHALL include bounded reason codes without raw credentials, raw source files, raw patches, raw diffs, raw scan payloads, raw provider payloads, or unbounded diagnostics

### Requirement: Code DTOs SHALL model workspaces, documents, syntax, symbols, diagnostics, actions, edits, patches, diffs, impact, tests, scans, and provider capability

`pack.developer.code.v1` SHALL define portable DTOs for code workspaces,
documents, ranges, syntax tree summaries, symbols, diagnostics, code actions,
workspace edit plans, patches, diffs, impact reports, test suggestions, scan
findings, provider capabilities, result pages, partial results, and diagnostics.
Provider-specific fields SHALL remain bounded adapter metadata and SHALL NOT
become OS-layer routing branches.

#### Scenario: Developer inspects workspace schema
- **WHEN** SDK schemas expose `CodeWorkspace`
- **THEN** the schema SHALL identify workspace handle, declared roots, trust state, language inventory, index state, file count, excluded paths, policy scope, and health
- **AND** raw absolute paths may be redacted according to policy and credentials SHALL NOT be exposed

#### Scenario: Developer inspects document and range schemas
- **WHEN** SDK schemas expose `CodeDocument` and `CodeRange`
- **THEN** the schemas SHALL include document handle, workspace handle, relative path handle, language id, version hash, content hash, size class, generated/vendor flags, sensitivity class, line/column range, byte range hash, selector kind, and redaction class
- **AND** raw source text SHALL NOT be required for portable application logic

#### Scenario: Developer inspects patch schema
- **WHEN** SDK schemas expose `CodePatch`
- **THEN** the schema SHALL include patch handle, patch format, affected documents, hunks, content hashes, generated-file markers, dry-run status, conflict diagnostics, approval status where applicable, and rollback handle
- **AND** raw patch bodies SHALL be represented by handles or bounded/redacted snippets in observability

#### Scenario: Provider reports capability limits
- **WHEN** SDK discovery inspects the active code provider
- **THEN** Macaca SHALL report languages, parser support, LSP-style feature support, scan support, patch formats, diff support, formatting support, test discovery, max workspace size, rate limits, lifecycle, health, and capability hash
- **AND** callers SHALL use this metadata instead of provider-name branches

### Requirement: Patch and edit operations SHALL be safe, staged, validated, and approval-aware

`pack.developer.code.v1` SHALL separate edit planning, patch generation, patch
validation, and patch application request into distinct typed commands. Patch
application SHALL require write permission, current content hashes, conflict
checks, approval when policy requires it, rollback metadata, and auditable
side-effect records.

#### Scenario: Edit is planned without mutation
- **WHEN** `code.plan_edit` is invoked with intent, diagnostics, selected ranges, or code action handles
- **THEN** Macaca SHALL validate read permission, action safety, path scope, provider capability, and resource limits
- **AND** it SHALL return an edit plan, affected documents, risk flags, required approvals, idempotency key, and rollback strategy without mutating files

#### Scenario: Patch is generated from edit plan
- **WHEN** `code.generate_patch` is invoked with an edit plan and output patch format
- **THEN** Macaca SHALL validate the plan, source content hashes, patch format, redaction profile, provider capability, and resource budget
- **AND** it SHALL return a patch handle, affected documents, hunk summaries, dry-run status, conflict diagnostics, and rollback handle

#### Scenario: Patch is validated without mutation
- **WHEN** `code.validate_patch` is invoked with a patch handle and current workspace state
- **THEN** Macaca SHALL verify content hashes, applicability, conflicts, protected files, generated/binary file checks, formatting impact, policy, and rollback metadata
- **AND** it SHALL return typed validation results without mutating files

#### Scenario: Patch application requires approval
- **WHEN** `code.apply_patch_request` is invoked for a validated patch that touches protected files, generated files, broad workspace scope, destructive deletes, or policy-sensitive paths
- **THEN** Macaca SHALL return approval-required before mutation unless a valid approval token is supplied
- **AND** trace/audit evidence SHALL record approval state, patch handle, content hashes, rollback handle, and result code without exposing raw patch bodies

#### Scenario: Patch conflicts with current files
- **WHEN** `code.apply_patch_request` is invoked and current content hashes no longer match the validated patch
- **THEN** Macaca SHALL return a typed conflict result and SHALL NOT mutate files
- **AND** it SHALL emit sanitized conflict diagnostics and replay pointers

### Requirement: Code analysis, impact, tests, and scan findings SHALL be bounded and replayable

`pack.developer.code.v1` SHALL support indexing, parsing, diagnostics, impact
analysis, test suggestion, and scan result inspection through bounded,
policy-checked commands. Stale indexes, unsupported languages, and unavailable
scanners SHALL be explicit diagnostics.

#### Scenario: Workspace index is stale
- **WHEN** `code.estimate_impact` or `code.find_references` depends on an index whose document hashes do not match current workspace state
- **THEN** Macaca SHALL return stale-index diagnostics or reduced-confidence results according to policy
- **AND** it SHALL NOT present stale output as fully authoritative

#### Scenario: Impact is estimated
- **WHEN** `code.estimate_impact` is invoked with changed symbols, documents, or patch handles
- **THEN** Macaca SHALL validate read permission, index state, provider capability, and resource limits
- **AND** it SHALL return affected symbols, packages, dependency edges, execution flows, suggested tests, confidence, stale-index warnings, and bounded rationale handles

#### Scenario: Tests are suggested
- **WHEN** `code.suggest_tests` is invoked with diagnostics, changed symbols, patch handle, or impact report
- **THEN** Macaca SHALL return provider-neutral test suggestions with command handles, test kind, scope, rationale handles, expected duration, required resources, and safety class
- **AND** it SHALL NOT execute terminal commands; execution belongs to the terminal/CI/workflow packs

#### Scenario: Scan findings are imported
- **WHEN** `code.import_scan_results` is invoked with SARIF-like scan output handles
- **THEN** Macaca SHALL validate source mapping, size limits, taxonomy, severity, related locations, baseline metadata, redaction, and scan permission
- **AND** it SHALL return typed import diagnostics and finding handles without raw scan payloads in observability

### Requirement: Code Pack SHALL enforce permissions, workspace scope, resource limits, entitlements, approvals, and redaction

`pack.developer.code.v1` SHALL define permission scopes for workspace reading,
indexing, document reading, parsing, symbols, diagnostics, code actions, edit
planning, patch generation, patch validation, patch application, diffs, impact,
test suggestion, scan import, scan reading, and provider inspection. Policy
SHALL run before side effects and SHALL account for workspace trust, path scope,
language support, file sensitivity, provider quota, output size, approval, and
redaction.

#### Scenario: Path is outside declared workspace
- **WHEN** a command targets a file path outside declared workspace roots or inside denied path scopes
- **THEN** Macaca SHALL return a typed denied result before provider access
- **AND** the concrete provider SHALL NOT receive the out-of-scope path

#### Scenario: Write permission is missing
- **WHEN** an application can analyze code but lacks `code.patch.apply`
- **THEN** `code.apply_patch_request` SHALL return a typed denied result and SHALL NOT mutate files
- **AND** audit evidence SHALL identify the missing scope by stable code

#### Scenario: Resource limits reject large analysis
- **WHEN** indexing, parsing, scan import, diff inspection, impact analysis, or patch generation exceeds file count, source bytes, syntax tree size, scan finding count, diff size, patch size, timeout, memory, storage, provider quota, output, or snapshot limits
- **THEN** Macaca SHALL return typed quota, timeout, cancellation, or partial-result diagnostics
- **AND** it SHALL emit bounded resource counters and stable reason codes

### Requirement: Code Pack SHALL expose industrial metadata and developer documentation

`pack.developer.code.v1` SHALL expose descriptor metadata for languages, parser
support, LSP-style features, scan support, patch formats, diff support, command
schemas, permission scopes, policy templates, resource budgets, approval
requirements, lifecycle state, compatibility, health probes, snapshots,
unavailable diagnostics, redaction profiles, SDK examples, provider capability
hashes, and developer documentation.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.developer.code.v1`
- **THEN** it SHALL return command namespace `code.*`, languages, parser support, LSP-style feature support, scan support, patch formats, diff support, supported commands, permissions, policy templates, examples, lifecycle, availability, health, diagnostics, compatibility, redaction profile, provider capability hash, and documentation links
- **AND** examples SHALL use generic handles and synthetic code rather than application-specific workflows, provider names, credentials, private source code, repository-specific conventions, or business routing

#### Scenario: Developer documentation is published
- **WHEN** the pack implementation is marked complete
- **THEN** `docs/developer-packs/developer/code.md` SHALL document manifest declaration, required versus optional behavior, permissions, workspace handles, path scopes, documents, ranges, syntax trees, symbols, diagnostics, code actions, edit plans, patches, diffs, impact reports, test suggestions, scan findings, unavailable diagnostics, provider replacement, trace/audit interpretation, operational limits, and conformance tests
- **AND** SDK discovery metadata and the industrial catalog index SHALL link to that guide

### Requirement: Code Pack SHALL be traceable, auditable, replayable, and sanitized

`pack.developer.code.v1` SHALL emit sanitized trace/audit events and bounded
snapshots for declaration, admission, workspace inspection, indexing, parsing,
symbol lookup, references, diagnostics, code actions, edit planning, patch
generation, patch validation, patch apply requests, diff inspection, impact
analysis, test suggestion, scan import, scan finding inspection, provider
inspection, policy/resource decisions, provider calls, unavailable states, and
replay.

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a code pack snapshot
- **THEN** the snapshot SHALL include descriptor version, provider capability hashes, language support, index state hashes, parser capability hashes, scan capability hashes, patch format support, command availability, provider health, policy template hash, resource counters, bounded workspace statistics, and sanitized replay pointers
- **AND** it SHALL exclude raw source, raw patches, raw diffs, raw scan payloads, raw provider payloads, credentials, prompts, manifests, package bytes, private keys, signatures, and unbounded diagnostics

#### Scenario: Patch request is audited
- **WHEN** `code.generate_patch`, `code.validate_patch`, or `code.apply_patch_request` runs
- **THEN** Macaca SHALL emit sanitized audit events with patch handle, affected document handles, content hash summaries, dry-run status, validation status, approval status, rollback handle, result code, and replay pointer
- **AND** raw patch bodies and raw source text SHALL NOT enter audit records

#### Scenario: Scan finding inspection is audited
- **WHEN** `code.import_scan_results` or `code.inspect_scan_findings` runs
- **THEN** Macaca SHALL emit sanitized audit events with scan handle, finding count, severity summary, taxonomy hash, baseline state, policy decision, result code, and replay pointer
- **AND** raw scan payloads and raw source snippets SHALL NOT enter audit records

### Requirement: Code Pack implementation SHALL preserve Macaca boundaries

The `pack.developer.code.v1` implementation SHALL remain owned by code
intelligence service providers behind the service runtime. The microkernel, SDK,
shells, and generic application framework SHALL remain provider-neutral and free
of application-specific or provider-specific routing branches.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete language server, VS Code API, Tree-sitter parser, CodeQL scanner, SARIF parser, model client, repository client, terminal client, or provider adapter imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.developer.code.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, capability hash, and result codes rather than provider-specific business branches

#### Scenario: SDK helper builds service call only
- **WHEN** an SDK helper such as `sdk.packs.developer.code.generate_patch(command)` is used
- **THEN** the helper SHALL build a canonical traced service call with command DTO, permission metadata, workspace handles, path scope, resource limits, redaction profile, and replay context
- **AND** it SHALL NOT construct providers, instantiate parsers, call language servers, parse raw scan payloads, run terminal commands, mutate files, route by provider name, or bypass policy
