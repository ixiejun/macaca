## ADDED Requirements

### Requirement: Macaca SHALL provide the Developer Design Tools Pack as a serviceized capability

Macaca SHALL provide `pack.developer.design.tools.v1` as a provider-neutral industrial pack for design workspace discovery, design file opening, page/canvas inspection, node tree inspection, component/library inspection, style/token inspection, token sync, asset export, design-to-code component mapping, design change planning, approved write requests, review/comment inspection, artifact handles, snapshots, and replay diagnostics. The pack SHALL be declared by applications, resolved by admission/catalog services, and invoked only through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.developer.design.tools.v1` as required and design-tool service provider is registered, healthy, entitled, workspace-scoped, host-capable, and policy-admissible
- **THEN** admission SHALL expose `pack.developer.design.tools.v1` in the effective capability set with command schemas, permission scopes, workspace/file scope metadata, policy template hash, provider capability hash, health, and replay metadata
- **AND** SDK discovery SHALL mark callable `design_tools.*` commands as available without exposing provider secrets, raw access tokens, raw design files, raw image assets, private comments, customer data, raw provider payloads, or application-specific workflow names

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.developer.design.tools.v1` as required but provider, credential reference, workspace permission, entitlement, resource, approval, network, host support, or policy admission is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, mutate files, sync tokens, export assets, overwrite components, notify collaborators, contact a provider, or fake success

#### Scenario: Optional declaration degrades explicitly
- **WHEN** an application declares `pack.developer.design.tools.v1` as optional and the pack or a sub-capability is unavailable
- **THEN** admission SHALL produce a degraded effective capability memento naming unavailable commands and bounded reason codes
- **AND** SDK command helpers SHALL refuse to build callable service calls for unavailable commands while preserving discoverability and diagnostics

### Requirement: Design tools commands SHALL use typed canonical service calls

Every `pack.developer.design.tools.v1` operation SHALL be represented as a typed command/result DTO and SHALL traverse the canonical service runtime path with trace, policy, resource, entitlement, approval, health, snapshot, and structured error behavior. SDK, WASM ABI, shell, and application-framework helpers SHALL only build canonical service commands and SHALL NOT construct concrete design-tool providers or call design-tool APIs directly.

#### Scenario: Read command succeeds through service runtime
- **WHEN** a declared and policy-allowed command such as `design_tools.open_file`, `design_tools.inspect_node`, `design_tools.inspect_tokens`, or `design_tools.inspect_reviews` is invoked
- **THEN** Macaca SHALL route the command through SDK/facade helpers into the service runtime and design-tool service provider
- **AND** it SHALL emit sanitized admission, policy, service-call, result, and replay events with stable trace identifiers and bounded paging metadata

#### Scenario: Mutating or artifact command is planned before request
- **WHEN** an application wants to sync tokens, export assets, or write design changes
- **THEN** Macaca SHALL require the applicable planning command with version preconditions, token schema compatibility, export policy, collaborator/notification policy, resource reservation, idempotency key, approval state where required, and provider capability validation
- **AND** planning commands SHALL be replay-addressable and SHALL NOT mutate design files, sync tokens, export assets, or notify collaborators

#### Scenario: Command is denied before provider invocation
- **WHEN** policy, permission, entitlement, approval, resource, quota, workspace, file, node, token schema, version, export, write, artifact, provider capability, or timeout checks reject a `design_tools.*` command
- **THEN** Macaca SHALL return a typed denied, validation, conflict, stale-version, schema-mismatch, export-denied, write-denied, artifact-denied, approval-required, quota, timeout, unavailable, or unsupported result before invoking the concrete provider
- **AND** the audit trail SHALL include only bounded reason codes and sanitized handles

### Requirement: Design tools DTOs SHALL model provider-neutral design concepts

`pack.developer.design.tools.v1` SHALL define provider-neutral DTOs for design scope, provider capability, workspace, file, page/canvas, node, component, style, token, token sync plan, export plan, artifact handle, change set, component mapping, review event, version/freshness metadata, and diagnostics. Provider-specific fields SHALL be exposed only as bounded `adapter_metadata` guarded by capability hashes and SHALL NOT drive OS-layer routing branches.

#### Scenario: Provider capability is inspected
- **WHEN** `design_tools.inspect_provider` is invoked for a provider or workspace scope
- **THEN** Macaca SHALL return provider-neutral `DesignToolProviderCapability` metadata for file support, node support, component support, style support, token support, export support, write support, comment/review support, auth modes, rate limits, lifecycle, health, and compatibility
- **AND** it SHALL include stable descriptor, provider capability, policy template, and compatibility hashes for validation and replay

#### Scenario: Node tree is inspected
- **WHEN** `design_tools.inspect_node` returns a node or bounded node tree
- **THEN** the result SHALL use `DesignNode`, page/file handles, parent handles, node kind, name handles, bounds class, style references, component references, child count class, version hash, depth metadata, and redaction class
- **AND** it SHALL NOT expose raw design files, raw image assets, private comments, customer data, raw provider payloads, or unbounded node trees

#### Scenario: Provider-specific capability exists
- **WHEN** an active provider supports a design concept not present in the canonical DTO model
- **THEN** the provider MAY expose bounded `adapter_metadata` and compatibility diagnostics through `DesignToolProviderCapability`
- **AND** the OS, SDK, shell, and generic application framework SHALL NOT branch on provider names, tool names, file names, component names, token names, node types, or design-system conventions

### Requirement: Token sync, asset export, and design writes SHALL be planned, requested, version-safe, approval-aware, and auditable

All design-tool side effects SHALL use plan/request separation, workspace/file/node scope validation, version preconditions, token schema compatibility, provider capability validation, resource reservations, idempotency, collaborator/notification policy, approval gates where required, and sanitized audit.

#### Scenario: Token sync is requested
- **WHEN** `design_tools.plan_token_sync` validates source/target handles, token schema hash, DTCG compatibility, alias/theme validation, conflicts, version preconditions, quota, and approvals
- **THEN** `design_tools.token_sync_request` MAY use the validated plan handle and idempotency key to request token sync
- **AND** Macaca SHALL record sanitized plan, request, token schema hash, provider capability hash, policy decision, audit reason, result handles, and replay pointer

#### Scenario: Asset export is requested
- **WHEN** `design_tools.plan_asset_export` validates source node/page/component handles, format, scale, bounds, color/profile policy, retention, sensitivity, resource budget, and approvals
- **THEN** `design_tools.export_asset_request` MAY request export through the service provider
- **AND** it SHALL return bounded `DesignArtifactHandle` metadata rather than raw image assets in traces, audits, snapshots, examples, or diagnostics

#### Scenario: Write detects stale version
- **WHEN** `design_tools.write_change_request` receives a change set whose file, node, component, style, or token version precondition no longer matches provider state
- **THEN** Macaca SHALL return a typed stale-version or schema-mismatch result
- **AND** it SHALL NOT apply partial writes unless the command explicitly declares provider-supported partial semantics and policy allows them

#### Scenario: Collaborator-visible write requires approval
- **WHEN** `design_tools.plan_write_change` detects token writes, component overwrites, destructive mutations, private/unpublished file updates, brand-library changes, or collaborator notifications
- **THEN** Macaca SHALL return approval-required diagnostics until valid approval is supplied
- **AND** no write, token sync, asset export, overwrite, or collaborator notification SHALL happen before approval

### Requirement: Design artifacts, reviews, tokens, and private design data SHALL be bounded and policy-controlled

`pack.developer.design.tools.v1` SHALL treat raw design files, image assets, token values, private comments, review metadata, customer data, and provider payloads as policy-controlled resources with explicit permissions, quotas, redaction, retention, and provider capability checks.

#### Scenario: Review metadata is inspected
- **WHEN** `design_tools.inspect_reviews` is invoked
- **THEN** Macaca SHALL return bounded `DesignReviewEvent` records with event kind, actor handle, timestamp, comment/review handle, changed fields, cursor, and redaction class
- **AND** it SHALL enforce review permission, page size, redaction, retention, timeout, and replay bounds

#### Scenario: Artifact handle is resolved
- **WHEN** `design_tools.get_artifact_handle` is invoked
- **THEN** Macaca SHALL validate artifact permission, source handle, content type, size class, retention, redaction class, provider capability, resource budget, and approval requirements
- **AND** it SHALL return bounded artifact metadata rather than raw design files, raw images, or raw provider payloads

#### Scenario: Token values are inspected
- **WHEN** `design_tools.inspect_tokens` returns style, variable, token, theme, or alias metadata
- **THEN** Macaca SHALL enforce token-read permission, sensitivity classification, schema compatibility metadata, output bounds, and redaction
- **AND** sensitive token values SHALL be represented as handles or bounded summaries according to policy

### Requirement: Design Tools Pack SHALL enforce permissions, scopes, resources, entitlements, approvals, and redaction

`pack.developer.design.tools.v1` SHALL enforce explicit permission scopes for provider inspection, workspace reading, file reading, page reading, node reading, component reading, token reading, token writing, asset export, component mapping, design writing, review reading, and artifact reading. Every command SHALL carry application id, tenant id, session id, task id, trace id, provider scope, workspace/file/page/node handle where applicable, and actor handle when available.

#### Scenario: Permission is missing
- **WHEN** an application invokes a `design_tools.*` command without the required permission scope
- **THEN** Macaca SHALL return a typed denied result before provider invocation
- **AND** the denied result SHALL identify the missing permission scope using sanitized identifiers

#### Scenario: Resource budget is exceeded
- **WHEN** file listing, node inspection, component inspection, token inspection, token sync, asset export, write planning, review inspection, or artifact retrieval exceeds file count, node depth, node count, component count, token count, export size, artifact size, comment/review count, payload size, provider quota, network transfer, timeout, memory, storage, or snapshot retention budgets
- **THEN** Macaca SHALL return typed quota, timeout, cancellation, export-denied, artifact-denied, or resource-denied diagnostics
- **AND** it SHALL preserve replayable audit evidence without raw design files, raw image assets, or provider payloads

#### Scenario: Sensitive operation requires approval
- **WHEN** policy marks private/unpublished files, brand libraries, customer data, private comments, token writes, component overwrites, collaborator-visible changes, destructive mutations, asset export, or external notifications as approval-required
- **THEN** Macaca SHALL return an approval-required result until a valid approval token is supplied
- **AND** no design file mutation, token sync, asset export, component overwrite, collaborator notification, or raw artifact retrieval SHALL happen before approval

### Requirement: Design Tools Pack SHALL expose industrial metadata and developer documentation

`pack.developer.design.tools.v1` SHALL expose descriptor metadata for command schemas, permission scopes, policy templates, resource budgets, approval rules, redaction profiles, provider capability hashes, SDK examples, lifecycle state, compatibility, health probes, snapshots, unavailable diagnostics, and documentation links. The implementation SHALL include detailed developer documentation at `docs/developer-packs/developer/design-tools.md`.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.developer.design.tools.v1`
- **THEN** it SHALL return command namespace `design_tools.*`, command schemas, permissions, workspace/file support, node support, component support, token support, export support, write support, review support, examples, lifecycle, availability, health, diagnostics, compatibility metadata, redaction profiles, and documentation link
- **AND** examples SHALL use synthetic workspaces, files, nodes, components, tokens, artifacts, and reviews rather than provider names, credentials, private comments, customer data, raw assets, proprietary designs, or workflow-specific conventions

#### Scenario: Developer documentation is complete
- **WHEN** the pack implementation is marked complete
- **THEN** `docs/developer-packs/developer/design-tools.md` SHALL document manifest declarations, required versus optional behavior, permissions, provider scopes, workspaces, files, pages/canvases, nodes, components, component sets, instances, styles, variables, tokens, libraries, exports, mappings, change sets, comments/reviews, artifacts, command DTOs, result DTOs, idempotency, pagination, timeout/cancellation, redaction, approvals, artifact retention, version preconditions, token schema compatibility, unavailable diagnostics, provider replacement, trace/audit interpretation, conformance tests, and supplier/API mapping
- **AND** the guide SHALL be linked from SDK discovery metadata and the industrial pack catalog index

### Requirement: Design Tools Pack SHALL be traceable, auditable, replayable, and sanitized

`pack.developer.design.tools.v1` SHALL emit sanitized trace and audit events for declaration, admission, provider inspection, workspace listing, file listing, file opening, page inspection, node inspection, component inspection, token inspection, token sync planning, token sync requests, asset export planning, asset export requests, component mapping, write planning, write requests, review inspection, artifact handle resolution, policy decisions, service-call lifecycle, failures, unavailable states, and snapshots.

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a `pack.developer.design.tools.v1` snapshot
- **THEN** the snapshot SHALL include descriptor version, provider capability hashes, workspace/file schema hashes, token schema hashes, command availability, provider health, policy template hash, resource counters, bounded file/node/component/token summaries, artifact summaries, review cursors, and sanitized replay pointers
- **AND** it SHALL exclude raw credentials, tokens, private comments, customer data, raw design files, raw image assets, raw provider payloads, prompts, manifests, package bytes, private keys, signatures, and unbounded node trees

#### Scenario: Replay reconstructs command evidence
- **WHEN** replay inspects a past `design_tools.*` command
- **THEN** Macaca SHALL reconstruct descriptor version, command DTO hash, policy decision, resource decision, approval state, provider capability hash, version preconditions, plan handle where applicable, artifact/review cursor where applicable, result classification, and sanitized provider class metadata
- **AND** replay SHALL NOT require raw provider payloads, raw design files, raw image assets, private comments, credentials, tokens, or application-specific workflow code

### Requirement: Design tools implementation SHALL preserve Macaca boundaries

The `pack.developer.design.tools.v1` implementation SHALL remain owned by design-tool service providers and service-runtime contracts. The microkernel, SDK, shells, and generic application framework SHALL remain provider-neutral and free of application-specific, supplier-specific, tool-specific, file-specific, component-specific, token-specific, or workflow-specific routing branches.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, canonical execution-path, and serviceization gates scan the implementation
- **THEN** they SHALL find no concrete Figma, Adobe, Penpot, Sketch, OAuth, plugin-runtime, desktop automation, credential-manager, asset-provider, or provider-adapter imports in the microkernel, SDK helpers, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.developer.design.tools.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, capability hashes, and bounded diagnostics rather than provider-specific business branches
