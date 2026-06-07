## ADDED Requirements

### Requirement: Macaca SHALL expose a production memory runtime facade

Macaca SHALL expose `MemoryRuntimeFacade` as the canonical upper-crate memory boundary for remember, search, active recall, knowledge compilation, and runtime status.

#### Scenario: Production callers use runtime facade

- **GIVEN** a configured Macaca web state
- **WHEN** active recall, explicit memory tools, or knowledge digest need memory access
- **THEN** they SHALL call the memory runtime facade or an adapter backed by it
- **AND** they SHALL NOT directly depend on a concrete legacy manager as their canonical boundary

#### Scenario: Legacy manager remains available as adapter

- **GIVEN** existing code or tests still require the legacy memory manager
- **WHEN** the memory runtime is initialized
- **THEN** the legacy manager MAY be used as a builtin backing store or compatibility adapter
- **AND** the legacy manager SHALL NOT be the production canonical boundary for new memory consumers

### Requirement: Memory provider runtime SHALL resolve real providers by profile and slot

Memory provider runtime SHALL resolve provider slots from profile configuration and build operational provider instances or truthful unsupported states.

#### Scenario: Provider runtime reports resolved profile slots

- **GIVEN** a memory profile config selects distinct agent private and session shared providers
- **WHEN** runtime status is requested
- **THEN** the status SHALL include the resolved provider ids for those slots
- **AND** diagnostics SHALL describe any missing or degraded slot

#### Scenario: MCP provider without live client is unavailable

- **GIVEN** an MCP memory provider is configured without a live MCP client
- **WHEN** status is requested
- **THEN** it SHALL report unavailable or unsupported capability state
- **AND** it SHALL NOT report healthy store/search capability

#### Scenario: MCP provider with live client maps configured tools

- **GIVEN** an MCP memory provider has a live client and configured memory store/search/get/delete tools
- **WHEN** remember, search, get, or delete is invoked
- **THEN** the provider SHALL call the configured MCP tool
- **AND** it SHALL validate returned schema and trust metadata before returning memory results

### Requirement: Embedding providers SHALL be registry-driven and decorator-safe

Macaca SHALL provide an embedding provider registry and decorator stack for cache, timeout, retry, and metrics without requiring external metrics dependencies.

#### Scenario: Registry resolves default embedding provider

- **GIVEN** a default embedding provider id is configured
- **WHEN** the embedding registry resolves a provider
- **THEN** it SHALL build the configured provider through a factory
- **AND** the provider dimensions SHALL be available for validation

#### Scenario: Slow embedding provider times out

- **GIVEN** an embedding provider exceeds its configured timeout
- **WHEN** embedding is requested
- **THEN** the timeout decorator SHALL return a memory diagnostic error without panicking
- **AND** query execution SHALL be able to degrade according to query policy

#### Scenario: Retry decorator retries transient failures

- **GIVEN** an embedding provider fails with a retryable memory error
- **WHEN** retry policy allows another attempt
- **THEN** the retry decorator SHALL retry up to the configured count
- **AND** metrics SHALL record calls, failures, and last latency

### Requirement: Vector backend contract SHALL have reusable conformance tests

Macaca SHALL provide reusable vector backend conformance tests for provider-neutral topology and isolation semantics.

#### Scenario: Agent private collections are isolated

- **GIVEN** two agents store vector memory in the same application
- **WHEN** conformance tests query each agent private scope
- **THEN** each agent SHALL only see its own private collection entries

#### Scenario: Session shared collection is explicit

- **GIVEN** session shared memory and agent private memory exist in the same application
- **WHEN** conformance tests query session shared scope
- **THEN** shared results SHALL NOT be mixed with agent private collections unless policy explicitly requests both

#### Scenario: Backend status reports topology

- **GIVEN** a vector backend implementation supports Macaca topology
- **WHEN** status is requested
- **THEN** it SHALL report application database and agent/session collection semantics without leaking vendor-specific assumptions into upper crates

### Requirement: Memory query pipeline SHALL support graceful degradation

Memory search SHALL support keyword, vector, hybrid, filtered, and rerank-compatible strategy composition, and SHALL degrade without blocking agent execution.

#### Scenario: Embedding fails during hybrid query

- **GIVEN** a hybrid query strategy
- **WHEN** embedding generation fails
- **THEN** keyword search SHALL still run when keyword index is available
- **AND** diagnostics SHALL record vector degradation

#### Scenario: Filtered query enforces metadata constraints

- **GIVEN** memory entries contain metadata
- **WHEN** a filtered query specifies metadata constraints
- **THEN** the query pipeline SHALL return only entries matching the constraints
- **AND** diagnostics SHALL indicate that filtering was applied

### Requirement: Knowledge compiler SHALL preserve evidence and deterministic conflicts

Knowledge compilation SHALL produce claims, evidence references, deterministic conflict groups, and artifacts suitable for wiki digest and project decision context.

#### Scenario: Contradictory claims are compiled into conflict group

- **GIVEN** two memory candidates contain deterministic negation pairs about the same subject
- **WHEN** knowledge compilation runs
- **THEN** the result SHALL include a conflict group
- **AND** both evidence ids SHALL remain visible to context reports

#### Scenario: Evidence ids remain exact

- **GIVEN** candidates compile into claims
- **WHEN** claim evidence is inspected
- **THEN** evidence source ids SHALL match the original memory or candidate ids
- **AND** full sensitive source text SHALL NOT be serialized into reports by default

#### Scenario: Wiki and decision artifacts are generated from structured knowledge

- **GIVEN** compiled knowledge includes claims, decisions, conflicts, and evidence
- **WHEN** artifacts are generated
- **THEN** Macaca SHALL produce bounded wiki digest, project decision log, or citation artifacts
- **AND** artifacts SHALL contain evidence references for exact lookup

### Requirement: Governance runtime SHALL be durable, auditable, and truthful about autonomy seams

Memory governance SHALL provide durable audit/candidate/tombstone snapshot support, configurable promotion policy, provider migration checkpoints, and truthful compaction/dreaming status.

#### Scenario: Candidate capture and promotion records audit

- **GIVEN** governance runtime captures and promotes a memory candidate
- **WHEN** audit events are listed for the scope
- **THEN** the audit log SHALL include candidate captured and promoted events
- **AND** the events SHALL include policy or decision reasons

#### Scenario: Tombstone remains authoritative across snapshot replay

- **GIVEN** a memory is deleted and tombstoned
- **WHEN** governance state is snapshotted and replayed
- **THEN** the tombstone SHALL remain authoritative
- **AND** deleted content SHALL NOT reappear through replay or provider sync

#### Scenario: Compaction disabled seam is truthful

- **GIVEN** autonomous compaction or dreaming is not enabled
- **WHEN** compaction is requested
- **THEN** the default strategy SHALL return no candidates
- **AND** diagnostics SHALL record `compaction_disabled` rather than pretending compaction completed

### Requirement: Provider migration SHALL be checkpointed and auditable

Memory provider migration SHALL copy, verify, complete, fail, or roll back through explicit checkpoints.

#### Scenario: Migration validation fails

- **GIVEN** a provider migration from source to target
- **WHEN** verification fails
- **THEN** the runtime SHALL keep the source provider authoritative
- **AND** it SHALL write an audit event describing the failed checkpoint

#### Scenario: Migration completes after verification

- **GIVEN** a provider migration copies all expected entries and evidence ids
- **WHEN** verification succeeds
- **THEN** the migration status SHALL become completed
- **AND** the audit log SHALL record source provider, target provider, scope, and checkpoint summary

### Requirement: macaca-web SHALL consume memory through runtime-backed adapters

Macaca web production memory consumers SHALL use `WebMemoryRuntime` or another adapter backed by `MemoryRuntimeFacade`.

#### Scenario: Memory search tool uses runtime facade

- **GIVEN** web memory runtime is configured
- **WHEN** `memory_search` executes
- **THEN** it SHALL call the runtime facade or runtime-backed adapter
- **AND** it SHALL preserve existing scope filtering behavior

#### Scenario: Active recall source uses runtime facade

- **GIVEN** active recall is enabled for a session
- **WHEN** workspace memory recall source gathers candidates
- **THEN** it SHALL call runtime active recall
- **AND** it SHALL preserve agent private and session shared visibility rules

#### Scenario: Knowledge digest capability uses runtime facade

- **GIVEN** knowledge digest context is enabled
- **WHEN** workspace knowledge digest capability compiles digest
- **THEN** it SHALL call runtime knowledge compilation
- **AND** it SHALL preserve tombstone and redaction behavior

### Requirement: Verification SHALL prove runtime migration scope and no duplicate direct memory injection

Completion SHALL require focused tests, OpenSpec validation, direct access scans, and GitNexus change detection.

#### Scenario: Direct legacy access scan is clean

- **GIVEN** implementation is complete
- **WHEN** source is scanned for direct `TestMemoryManager` recall and `workspace_memory.recall`
- **THEN** remaining matches SHALL be limited to legacy backing store, tests, or compatibility adapters
- **AND** no duplicate production memory injection path SHALL remain

#### Scenario: Related OpenSpec changes validate

- **GIVEN** implementation tasks are complete
- **WHEN** related OpenSpec changes are validated with `--strict`
- **THEN** all listed memory/context changes SHALL be valid
- **AND** task checkboxes SHALL match implemented code reality
