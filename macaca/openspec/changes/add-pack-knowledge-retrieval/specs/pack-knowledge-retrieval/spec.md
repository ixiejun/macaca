## ADDED Requirements

### Requirement: Macaca SHALL provide Knowledge Retrieval Pack as a serviceized capability

Macaca SHALL provide `pack.knowledge.retrieval.v1` as a provider-neutral
industrial pack for collection registration, vector/sparse/hybrid retrieval,
record writes, retrieve-by-id, reranking, context expansion, evidence packaging,
collection inspection, refresh, ACL filtering, source provenance, and unavailable
diagnostics. Applications SHALL declare the pack in manifests, admission SHALL
resolve it into effective capabilities, and all operations SHALL run through
typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.knowledge.retrieval.v1` as required and a retrieval service provider is registered, healthy, entitled, collection-compatible, vector-space-compatible, ACL-compatible, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, collection capability metadata, permission scopes, policy templates, vector-space compatibility, top-k limits, health, diagnostics, documentation links, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing raw credentials, raw provider payloads, raw vectors, raw documents, raw chunks beyond policy, raw prompt text, or private corpus content

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.knowledge.retrieval.v1` as required but provider, collection support, permission, entitlement, ACL model, embedding compatibility, resource budget, or policy support is absent
- **THEN** admission SHALL block readiness with structured unavailable, denied, unsupported, validation, conflict, or quota diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.knowledge.retrieval.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report
- **AND** SDK command helpers SHALL return Null Object unavailable diagnostics instead of creating callable service calls

### Requirement: Retrieval commands SHALL use typed canonical service calls

Every `pack.knowledge.retrieval.v1` operation SHALL be represented as a typed
command/result DTO and SHALL traverse the canonical service runtime path with
trace, policy, resource, entitlement, approval, health, snapshot, query
validation, ACL filtering, namespace isolation, redaction, replay, and structured
error behavior.

#### Scenario: Collection is registered
- **WHEN** `retrieval.register_collection` is invoked with collection metadata, namespace policy, vector-space descriptors, embedding model references, ACL model, and secret references
- **THEN** Macaca SHALL validate declaration, entitlement, provider capability, vector-space compatibility, ACL model, policy, and descriptor compatibility before registering the collection
- **AND** the result SHALL contain an opaque collection handle and sanitized capability metadata rather than raw credentials or provider payloads

#### Scenario: Retrieval query is executed
- **WHEN** `retrieval.retrieve` is invoked with a collection handle, query embedding or query text handle, vector-space target, metadata filter, top-k, hybrid strategy, rerank policy, and evidence policy
- **THEN** Macaca SHALL validate collection access, namespace isolation, ACL filtering, embedding/vector compatibility, query complexity, provider capability, top-k budget, resource budget, and redaction policy before invoking the provider
- **AND** it SHALL return typed candidates, normalized scores, strategy metadata, provenance, evidence pointers, and sanitized replay evidence

#### Scenario: Hybrid retrieval is executed
- **WHEN** `retrieval.retrieve` or `retrieval.bulk_retrieve` requests dense, sparse, lexical, or multivector hybrid fusion
- **THEN** Macaca SHALL require provider capability metadata and explicit fusion strategy
- **AND** unsupported vector spaces, fusion modes, or filter combinations SHALL return typed unsupported or validation diagnostics rather than provider-specific errors

#### Scenario: Command is denied before provider call
- **WHEN** policy, permission, entitlement, ACL, approval, resource, query validation, namespace, vector-space, or redaction checks reject a retrieval command
- **THEN** Macaca SHALL return a typed denied, validation, conflict, or quota result before invoking the concrete provider
- **AND** audit evidence SHALL include a bounded reason code without raw vectors, raw chunks, raw documents, raw provider payloads, raw prompt text, credentials, or private corpus content

### Requirement: Retrieval DTOs SHALL model collections, namespaces, records, chunks, vector spaces, queries, candidates, and evidence

`pack.knowledge.retrieval.v1` SHALL define portable DTOs for collections,
namespaces, records, chunks, vector spaces, metadata filters, retrieval queries,
hybrid fusion, candidates, scores, rerank results, context windows, evidence
bundles, cursors, freshness, provider capability, and diagnostics.
Provider-specific fields SHALL remain bounded adapter metadata and SHALL NOT
become OS-layer routing branches.

#### Scenario: Developer inspects collection schema
- **WHEN** SDK discovery or `retrieval.inspect_collection` exposes collection metadata
- **THEN** the schema SHALL include collection handle, namespace policy, vector spaces, embedding model references, ACL model, freshness, retention, filter support, fusion support, top-k limits, provider health, and compatibility
- **AND** raw vectors, raw provider topology beyond policy, credentials, raw documents, raw chunks, and private corpus content SHALL NOT be exposed

#### Scenario: Developer packages evidence
- **WHEN** `retrieval.package_evidence` is invoked with candidate handles and evidence policy
- **THEN** Macaca SHALL return ordered evidence with redacted content handles, source attribution, offsets, chunk/window metadata, confidence, freshness, dedupe metadata, and replay pointer
- **AND** evidence content SHALL be bounded by token/byte limits and redaction policy

#### Scenario: Provider reports capability limits
- **WHEN** SDK discovery inspects the active retrieval provider
- **THEN** Macaca SHALL report dense/sparse/multivector support, named vector spaces, metadata filters, namespaces/partitions, bulk query, hybrid fusion, range search, rerank support, parent-window expansion, max top-k, max filters, rate limits, consistency, lifecycle, and health
- **AND** callers SHALL use this metadata rather than provider-name branches

### Requirement: Retrieval Pack SHALL enforce permissions, ACL filtering, namespace isolation, and score semantics

`pack.knowledge.retrieval.v1` SHALL define permission scopes for collection
management, record writing, querying, reading, evidence packaging, reranking,
metadata inspection, and refresh. Policy SHALL run before side effects and SHALL
account for collection ownership, namespace isolation, embedding compatibility,
metadata filter validation, top-k limits, score normalization, context windows,
provider capability, resource budgets, and approval.

#### Scenario: ACL filtering removes unauthorized candidates
- **WHEN** a retrieval provider returns candidates outside the caller's corpus, namespace, or record ACL scope
- **THEN** Macaca SHALL remove or redact unauthorized candidates before returning them
- **AND** trace/audit evidence SHALL record bounded ACL-filtering counters without revealing unauthorized record details

#### Scenario: Top-k or context window exceeds policy
- **WHEN** a retrieval query requests top-k, bulk query count, range threshold, or parent context windows beyond policy or provider limits
- **THEN** Macaca SHALL return a typed quota or validation result with bounded diagnostics
- **AND** it SHALL NOT invoke the provider with unbounded retrieval parameters

#### Scenario: Scores are normalized
- **WHEN** retrieval candidates come from providers or vector spaces with different distance/score semantics
- **THEN** Macaca SHALL return metric metadata, raw score class, normalized score, fusion strategy, and confidence metadata
- **AND** consumers SHALL NOT infer cross-provider comparability without normalized score metadata

### Requirement: Retrieval Pack SHALL expose industrial metadata and developer documentation

`pack.knowledge.retrieval.v1` SHALL expose descriptor metadata for collection
capabilities, vector-space schemas, command schemas, permission scopes, policy
templates, filter support, fusion support, namespace/partition support, rerank
support, top-k limits, score normalization, resource budgets, SDK examples,
lifecycle state, compatibility, health probes, snapshots, unavailable
diagnostics, redaction profiles, and developer documentation.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.knowledge.retrieval.v1`
- **THEN** it SHALL return command namespace `retrieval.*`, collection capabilities, supported commands, permissions, policy templates, vector-space compatibility, filter/fusion/rerank support, namespace limits, top-k limits, examples, lifecycle, availability, health, diagnostics, compatibility, redaction profile, and documentation links
- **AND** examples SHALL use generic handles and synthetic data rather than application-specific workflows, provider names, credentials, prompt text, or business routing

#### Scenario: Developer documentation is published
- **WHEN** the pack implementation is marked complete
- **THEN** `docs/developer-packs/knowledge/retrieval.md` SHALL document manifest declaration, permissions, collection registration, namespace design, vector-space compatibility, record upsert/delete, metadata filters, vector/hybrid retrieval, bulk/range retrieval, score normalization, reranking, context expansion, evidence packaging, ACL filtering, provider replacement, unavailable diagnostics, trace/audit interpretation, and operational limits
- **AND** SDK discovery metadata and the industrial catalog index SHALL link to that guide

### Requirement: Retrieval Pack SHALL be traceable, auditable, replayable, and sanitized

`pack.knowledge.retrieval.v1` SHALL emit sanitized trace/audit events and
bounded snapshots for declaration, admission, collection registration, record
writes/deletes, retrieval queries, bulk queries, range queries, reranking,
context expansion, evidence packaging, refresh, diagnostics, policy/resource
decisions, provider calls, unavailable states, and replay.

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a retrieval pack snapshot
- **THEN** the snapshot SHALL include descriptor version, collection capability hashes, vector-space schema hashes, namespace summaries, provider health, index freshness, command availability, policy template hash, resource counters, top-k/rerank limits, and sanitized replay pointers
- **AND** it SHALL exclude raw vectors, raw documents, raw chunks, raw provider payloads, credentials, raw prompt text, private corpus content, and unbounded output

#### Scenario: Retrieval query is audited
- **WHEN** retrieval, bulk retrieval, range retrieval, rerank, expand context, package evidence, refresh, or diagnostics command runs
- **THEN** Macaca SHALL emit a sanitized audit event with stable collection handles, command name, query hash, namespace hash, policy decision, ACL-filtering counters, top-k bounds, provider capability hash, result code, and replay pointer
- **AND** the event SHALL exclude raw sensitive query text when policy requires hashing or redaction

### Requirement: Retrieval implementation SHALL preserve Macaca boundaries

The `pack.knowledge.retrieval.v1` implementation SHALL remain owned by
retrieval service providers behind the service runtime. The microkernel, SDK,
shells, and generic application framework SHALL remain provider-neutral and free
of application-specific or provider-specific routing branches.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete retrieval provider or vector-store adapter imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.knowledge.retrieval.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, capability hash, and result codes rather than provider-specific business branches
