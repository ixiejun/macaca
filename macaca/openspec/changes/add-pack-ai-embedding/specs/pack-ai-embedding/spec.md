## ADDED Requirements

### Requirement: Macaca SHALL provide the AI Embedding Pack as a serviceized capability

Macaca SHALL provide `pack.ai.embedding.v1` as a provider-neutral industrial pack for text/image embedding, batch embedding, vector metadata, and model diagnostics. The pack SHALL be declared by applications, resolved by admission/catalog services, and invoked only through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.ai.embedding.v1` as required and embedding service provider is registered, healthy, entitled, and policy-admissible
- **THEN** admission SHALL expose `pack.ai.embedding.v1` in the effective capability set with command schemas, permission scopes, policy template, health, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing provider secrets or raw provider payloads

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.ai.embedding.v1` as required but provider, permission, entitlement, resource, or host support is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.ai.embedding.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report
- **AND** SDK command helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: AI Embedding Pack commands SHALL use typed canonical service calls

Every `pack.ai.embedding.v1` operation SHALL be represented as a typed command/result DTO and SHALL traverse the canonical service runtime path with trace, policy, resource, entitlement, approval, health, snapshot, and structured error behavior.

#### Scenario: Command succeeds through service runtime
- **WHEN** a declared and policy-allowed command such as `embedding.embed_text` is invoked
- **THEN** Macaca SHALL route the command through SDK/facade helpers into the service runtime and embedding service provider
- **AND** it SHALL emit sanitized admission, policy, service-call, result, and replay events with stable trace identifiers

#### Scenario: Command is denied before side effects
- **WHEN** policy, permission, entitlement, approval, or resource checks reject a `pack.ai.embedding.v1` command
- **THEN** Macaca SHALL return a typed denied or quota result before invoking the concrete provider
- **AND** the audit trail SHALL include the bounded reason code without raw user data or provider payloads

#### Scenario: Command is unsupported by the active provider
- **WHEN** a descriptor exists but the active provider does not support a requested command
- **THEN** Macaca SHALL return a typed unsupported result with descriptor and provider capability diagnostics
- **AND** SDK discovery SHALL report the command as non-callable for the current effective capability set

### Requirement: AI Embedding Pack SHALL expose concrete industrial metadata

`pack.ai.embedding.v1` SHALL expose descriptor metadata for command schemas, permission scopes, policy templates, resource budgets, SDK examples, lifecycle state, compatibility, health probes, snapshots, and unavailable diagnostics.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.ai.embedding.v1`
- **THEN** it SHALL return the command namespace `embedding.*`, supported commands, permissions, policy templates, examples, lifecycle, availability, health, diagnostics, and compatibility metadata
- **AND** examples SHALL use generic handles or synthetic data rather than application-specific workflows

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a `pack.ai.embedding.v1` snapshot
- **THEN** the snapshot SHALL include descriptor version, provider class, health, command availability, policy template hash, resource counters, and sanitized replay pointers
- **AND** it SHALL exclude raw secrets, credentials, prompts, manifests, package bytes, private keys, signatures, raw provider payloads, and unbounded output

### Requirement: AI Embedding Pack implementation SHALL preserve Macaca boundaries

The `pack.ai.embedding.v1` implementation SHALL remain owned by embedding service provider; the microkernel, SDK, shells, and generic application framework SHALL remain provider-neutral and free of application-specific or provider-specific routing branches.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.ai.embedding.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class and descriptor metadata rather than provider-specific business branches

### Requirement: AI Embedding Pack SHALL model vectors, schemas, and ordered batches

`pack.ai.embedding.v1` SHALL expose typed embedding inputs, batch requests, vector results, vector schemas, truncation policy, usage accounting, and per-item diagnostics.

#### Scenario: Batch result preserves input mapping
- **WHEN** `embedding.batch_embed` processes multiple input items
- **THEN** every result SHALL include the original item id, item index, status, vector schema id, and bounded diagnostics
- **AND** partial failure SHALL NOT reorder successful items or hide denied items

#### Scenario: Vector schema is inspected
- **WHEN** `embedding.inspect_vector_schema` is invoked
- **THEN** Macaca SHALL return dimension, numeric type, normalization state, metric compatibility, schema hash, and lifecycle metadata
- **AND** it SHALL NOT expose provider secrets or raw embedded content

#### Scenario: Dimension mismatch is rejected
- **WHEN** `embedding.validate_schema` compares a vector result against an incompatible target schema
- **THEN** Macaca SHALL return a typed schema mismatch result
- **AND** downstream vector-store ingestion examples SHALL treat the vector as non-ingestable

#### Scenario: Truncation policy is explicit
- **WHEN** an input exceeds token, byte, image, or batch limits
- **THEN** Macaca SHALL apply the declared truncation policy or return `quota_exceeded`/`denied`
- **AND** the result SHALL include bounded truncation metadata without raw input content

### Requirement: AI Embedding Pack SHALL remain separate from storage and retrieval

`pack.ai.embedding.v1` SHALL produce vectors and metadata but SHALL NOT own vector index persistence, retrieval, graph linkage, or application ranking behavior.

#### Scenario: Embedding output is handed to another service
- **WHEN** an application wants to index embedding results
- **THEN** SDK examples SHALL show a separate retrieval/vector-store capability call with schema validation evidence
- **AND** the embedding service SHALL NOT write to storage as a hidden side effect

#### Scenario: Raw content is redacted from observability
- **WHEN** embedding commands emit trace, audit, snapshot, or replay diagnostics
- **THEN** Macaca SHALL include content hashes, modality, size bands, vector dimensions, schema ids, and usage counters
- **AND** raw text, images, prompts, credentials, and provider payloads SHALL NOT be recorded

#### Scenario: Unsupported modality is explicit
- **WHEN** `embedding.embed_image` is invoked against a provider that only supports text embeddings
- **THEN** Macaca SHALL return a typed unsupported result
- **AND** SDK discovery SHALL mark image embedding unavailable for the current effective capability set
