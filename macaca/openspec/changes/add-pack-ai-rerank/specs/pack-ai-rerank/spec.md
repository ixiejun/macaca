## ADDED Requirements

### Requirement: Macaca SHALL provide the AI Rerank Pack as a serviceized capability

Macaca SHALL provide `pack.ai.rerank.v1` as a provider-neutral industrial pack for candidate reranking, score explanation, batch ranking, and evaluation metadata. The pack SHALL be declared by applications, resolved by admission/catalog services, and invoked only through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.ai.rerank.v1` as required and rerank service provider is registered, healthy, entitled, and policy-admissible
- **THEN** admission SHALL expose `pack.ai.rerank.v1` in the effective capability set with command schemas, permission scopes, policy template, health, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing provider secrets or raw provider payloads

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.ai.rerank.v1` as required but provider, permission, entitlement, resource, or host support is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.ai.rerank.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report
- **AND** SDK command helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: AI Rerank Pack commands SHALL use typed canonical service calls

Every `pack.ai.rerank.v1` operation SHALL be represented as a typed command/result DTO and SHALL traverse the canonical service runtime path with trace, policy, resource, entitlement, approval, health, snapshot, and structured error behavior.

#### Scenario: Command succeeds through service runtime
- **WHEN** a declared and policy-allowed command such as `rerank.rerank` is invoked
- **THEN** Macaca SHALL route the command through SDK/facade helpers into the service runtime and rerank service provider
- **AND** it SHALL emit sanitized admission, policy, service-call, result, and replay events with stable trace identifiers

#### Scenario: Command is denied before side effects
- **WHEN** policy, permission, entitlement, approval, or resource checks reject a `pack.ai.rerank.v1` command
- **THEN** Macaca SHALL return a typed denied or quota result before invoking the concrete provider
- **AND** the audit trail SHALL include the bounded reason code without raw user data or provider payloads

#### Scenario: Command is unsupported by the active provider
- **WHEN** a descriptor exists but the active provider does not support a requested command
- **THEN** Macaca SHALL return a typed unsupported result with descriptor and provider capability diagnostics
- **AND** SDK discovery SHALL report the command as non-callable for the current effective capability set

### Requirement: AI Rerank Pack SHALL expose concrete industrial metadata

`pack.ai.rerank.v1` SHALL expose descriptor metadata for command schemas, permission scopes, policy templates, resource budgets, SDK examples, lifecycle state, compatibility, health probes, snapshots, and unavailable diagnostics.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.ai.rerank.v1`
- **THEN** it SHALL return the command namespace `rerank.*`, supported commands, permissions, policy templates, examples, lifecycle, availability, health, diagnostics, and compatibility metadata
- **AND** examples SHALL use generic handles or synthetic data rather than application-specific workflows

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a `pack.ai.rerank.v1` snapshot
- **THEN** the snapshot SHALL include descriptor version, provider class, health, command availability, policy template hash, resource counters, and sanitized replay pointers
- **AND** it SHALL exclude raw secrets, credentials, prompts, manifests, package bytes, private keys, signatures, raw provider payloads, and unbounded output

### Requirement: AI Rerank Pack implementation SHALL preserve Macaca boundaries

The `pack.ai.rerank.v1` implementation SHALL remain owned by rerank service provider; the microkernel, SDK, shells, and generic application framework SHALL remain provider-neutral and free of application-specific or provider-specific routing branches.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.ai.rerank.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class and descriptor metadata rather than provider-specific business branches

### Requirement: AI Rerank Pack SHALL rank caller-provided candidates deterministically

`pack.ai.rerank.v1` SHALL accept explicit query and candidate DTOs, validate candidate visibility and limits, and return stable ranked results with score metadata.

#### Scenario: Candidate ids are stable
- **WHEN** `rerank.rerank` is invoked with candidate ids
- **THEN** every result SHALL reference a supplied candidate id and rank
- **AND** no provider-generated opaque candidate id SHALL replace the caller's stable ids

#### Scenario: Top-n bounds are enforced
- **WHEN** a rerank request asks for more results than policy, provider descriptor, or resource budget allows
- **THEN** Macaca SHALL reject the request or apply an explicit bounded top-n policy before provider invocation
- **AND** the chosen behavior SHALL be visible in sanitized diagnostics

#### Scenario: Tie breaker is replayable
- **WHEN** two candidates receive equivalent scores or score bands
- **THEN** Macaca SHALL apply the declared tie-breaker policy
- **AND** replay SHALL reproduce the same order without raw candidate content

#### Scenario: Hidden candidate is denied
- **WHEN** a candidate is not visible to the caller under policy
- **THEN** Macaca SHALL deny or omit that candidate according to declared request policy before provider invocation
- **AND** diagnostics SHALL NOT leak hidden candidate content or count-sensitive metadata

### Requirement: AI Rerank Pack SHALL expose score explanations and evaluation metadata safely

`pack.ai.rerank.v1` SHALL provide optional explanation and evaluation metadata without leaking raw query or candidate content.

#### Scenario: Score explanation is redacted
- **WHEN** `rerank.explain_scores` is invoked
- **THEN** Macaca SHALL return bounded feature references, score bands, normalized scores, and explanation metadata when supported
- **AND** raw candidate text, images, prompts, credentials, and provider payloads SHALL NOT be returned

#### Scenario: Batch rerank preserves mapping
- **WHEN** `rerank.batch_rerank` processes multiple queries
- **THEN** each result SHALL carry batch item id, query id, candidate id, status, rank, and bounded diagnostics
- **AND** partial failures SHALL NOT reorder unrelated query results

#### Scenario: Rerank does not retrieve candidates
- **WHEN** an application needs candidate discovery before reranking
- **THEN** SDK examples SHALL show a separate search or retrieval capability call feeding candidate refs into rerank
- **AND** the rerank provider SHALL NOT perform hidden retrieval as an OS-layer side effect
