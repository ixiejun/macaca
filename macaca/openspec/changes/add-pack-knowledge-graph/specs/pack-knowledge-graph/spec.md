## ADDED Requirements

### Requirement: Macaca SHALL provide Knowledge Graph Pack as a serviceized capability

Macaca SHALL provide `pack.knowledge.graph.v1` as a provider-neutral industrial
pack for property graph stores, RDF graph stores, schemas, nodes, edges, RDF
triples/quads, graph queries, traversals, path queries, entity merge, subgraph
import/export, provenance, provider capability inspection, and unavailable
diagnostics. Applications SHALL declare the pack in manifests, admission SHALL
resolve it into effective capabilities, and all operations SHALL run through
typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.knowledge.graph.v1` as required and a knowledge graph service provider is registered, healthy, entitled, model-compatible, query-compatible, schema-compatible, quota-compatible, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, graph model support, query dialect metadata, permission scopes, policy templates, resource limits, health, diagnostics, documentation links, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing credentials, raw provider payloads, raw private graph values, raw source documents, raw execution plans, raw manifests, package bytes, private keys, signatures, or unbounded results

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.knowledge.graph.v1` as required but provider, graph model, query dialect, schema support, permission, entitlement, approval, resource budget, or host support is absent
- **THEN** admission SHALL block readiness with structured unavailable, denied, unsupported, validation, approval-required, conflict, quota, timeout, or failure diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, execute another provider implicitly, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.knowledge.graph.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report
- **AND** SDK command helpers SHALL return Null Object unavailable diagnostics instead of creating callable service calls

### Requirement: Graph commands SHALL use typed canonical service calls

Every `pack.knowledge.graph.v1` operation SHALL be represented as a typed
command/result DTO and SHALL traverse the canonical service runtime path with
trace, policy, resource, entitlement, approval, health, snapshot, redaction,
replay, and structured error behavior.

#### Scenario: Property graph node is upserted
- **WHEN** `graph.upsert_node` is invoked with a graph store handle, stable key, labels or types, properties, idempotency key, provenance, confidence, validity interval, and redaction class
- **THEN** Macaca SHALL validate store model compatibility, schema constraints, property policy, idempotency, permission, entitlement, resource budget, and provider capability before invoking the provider
- **AND** it SHALL return a typed node handle, version hash, provenance handle, mutation status, and sanitized replay pointer

#### Scenario: Property graph edge is upserted
- **WHEN** `graph.upsert_edge` is invoked with source node, target node, direction, relationship type, properties, idempotency key, and provenance
- **THEN** Macaca SHALL validate endpoint existence policy, relationship type compatibility, direction, schema constraints, permission, entitlement, and resource budget before invoking the provider
- **AND** it SHALL emit sanitized mutation audit evidence with stable handles rather than raw private graph values

#### Scenario: RDF statement is upserted
- **WHEN** `graph.upsert_triple` is invoked with subject, predicate, object, optional named graph, datatype, language, provenance, and idempotency key
- **THEN** Macaca SHALL validate RDF term shape, named graph scope, datatype/language compatibility, permission, entitlement, policy, and provider RDF capability before invoking the provider
- **AND** it SHALL return a typed statement handle, version hash, provenance handle, and replay pointer

#### Scenario: Command is denied before provider call
- **WHEN** policy, permission, entitlement, approval, resource, schema validation, query validation, source access, or redaction checks reject a `graph.*` command
- **THEN** Macaca SHALL return a typed denied, approval-required, validation, conflict, quota, timeout, unavailable, or unsupported result before invoking the concrete provider
- **AND** audit evidence SHALL include bounded reason codes without raw credentials, raw provider payloads, raw source documents, raw private graph values, raw queries marked sensitive by policy, raw execution plans, or unbounded output

### Requirement: Graph DTOs SHALL model property graphs, RDF graphs, queries, traversals, paths, imports, exports, provenance, and provider capability

`pack.knowledge.graph.v1` SHALL define portable DTOs for graph stores, graph
schemas, property graph nodes, property graph edges, RDF terms, RDF statements,
graph properties, graph queries, query results, traversals, paths, import plans,
export plans, provenance, provider capabilities, result pages, partial results,
and diagnostics. Provider-specific fields SHALL remain bounded adapter metadata
and SHALL NOT become OS-layer routing branches.

#### Scenario: Developer inspects graph store schema
- **WHEN** SDK schemas expose `GraphStore` and `GraphSchema`
- **THEN** the schema SHALL identify graph model, tenant/application scope, retention class, consistency profile, transaction profile, schema version, labels, vertex types, edge types, relationship types, RDF vocabularies, properties, datatypes, cardinality, constraints, indexes, uniqueness rules, compatibility, provider capability hash, and health
- **AND** raw provider connection details, credentials, raw provider payloads, and private graph data SHALL NOT be exposed

#### Scenario: Developer inspects property graph DTOs
- **WHEN** SDK schemas expose `GraphNode` and `GraphEdge`
- **THEN** the schemas SHALL include stable handles, labels or relationship types, endpoint handles for edges, properties as typed/redacted values, source handles, confidence, validity intervals, provenance, version hashes, and redaction classes
- **AND** provider-specific node ids SHALL NOT be required for portable application logic

#### Scenario: Developer inspects RDF DTOs
- **WHEN** SDK schemas expose `RdfTerm` and `RdfStatement`
- **THEN** the schemas SHALL represent IRIs, blank node handles, literal value handles, datatypes, language tags, subject, predicate, object, named graph handle, provenance, confidence, validity interval, version hash, and redaction class
- **AND** raw literal values marked sensitive by policy SHALL be represented by handles or redacted metadata in observability

#### Scenario: Provider reports capability limits
- **WHEN** SDK discovery inspects the active graph provider
- **THEN** Macaca SHALL report graph models, query dialects, schema support, constraint/index support, transaction capability, import/export formats, path algorithms, max depth, max fanout, max result size, consistency profile, rate limits, lifecycle, health, and capability hash
- **AND** callers SHALL use this metadata instead of provider-name branches

### Requirement: Graph query, traversal, and path operations SHALL be bounded and validated

`pack.knowledge.graph.v1` SHALL support provider-neutral graph query requests
and provider-declared dialect query requests only after validation. Query,
traversal, and path operations SHALL enforce bounded max rows, max depth,
fanout, timeout, memory, output, redaction, and permission limits before
provider calls.

#### Scenario: Query is validated without execution
- **WHEN** `graph.validate_query` is invoked with a query mode, query payload or portable plan, parameters, dialect profile, and resource limits
- **THEN** Macaca SHALL validate syntax or plan shape, provider capability, permission, redaction, max rows, max depth, timeout, and cost bounds without executing data access beyond validation policy
- **AND** it SHALL return typed validation issues, warnings, capability diagnostics, and estimated resource class

#### Scenario: Query returns paged results
- **WHEN** `graph.query` is invoked with valid policy, permission, provider capability, parameters, and result limits
- **THEN** Macaca SHALL return typed result pages containing bounded bindings, rows, path handles, graph item handles, subgraph handles, warnings, cost counters, page tokens, partial status, and replay pointers
- **AND** raw provider result payloads and unbounded subgraphs SHALL NOT be exposed

#### Scenario: Traversal is bounded
- **WHEN** `graph.traverse` is invoked with start selectors, edge or predicate filters, direction, max depth, max fanout, uniqueness policy, stop conditions, and result budget
- **THEN** Macaca SHALL enforce depth, fanout, timeout, memory, and output limits before provider execution
- **AND** it SHALL return partial-result diagnostics when limits are reached instead of hanging or silently truncating

#### Scenario: Path query is capability-limited
- **WHEN** `graph.find_path` is invoked for shortest, weighted, or pattern-constrained paths
- **THEN** Macaca SHALL validate provider path capability, weight support, max depth, cost budget, and result limits
- **AND** it SHALL return typed path results, unsupported diagnostics, or partial diagnostics according to provider capability

### Requirement: Graph mutations, imports, exports, deletes, and merges SHALL preserve policy and provenance

`pack.knowledge.graph.v1` SHALL require provenance, conflict policy,
idempotency, resource budgets, redaction, and approval where appropriate for
mutating operations. Import, export, delete, and merge commands SHALL be bounded,
auditable, and reversible where the provider and policy allow.

#### Scenario: Schema is upserted
- **WHEN** `graph.upsert_schema` is invoked with labels, vertex types, edge types, relationship types, RDF vocabularies, properties, constraints, indexes, and compatibility version
- **THEN** Macaca SHALL validate provider schema capability, migration compatibility, policy, entitlement, and resource budget before applying the schema change
- **AND** it SHALL return schema version hash, migration diagnostics, and replay pointer

#### Scenario: Subgraph import performs dry run
- **WHEN** `graph.import_subgraph` is invoked with dry-run mode, source handle, input format, schema mapping, batch policy, conflict policy, and redaction profile
- **THEN** Macaca SHALL validate format support, source permission, schema mapping, quota, estimated mutations, and conflict behavior without committing data
- **AND** it SHALL return a typed import plan, validation report, and load diagnostics

#### Scenario: Subgraph export is redacted
- **WHEN** `graph.export_subgraph` is invoked with selector, query, provenance filters, output format, redaction profile, limit policy, and destination handle
- **THEN** Macaca SHALL validate export permission, destination permission, provider capability, output size, redaction, and resource budget
- **AND** it SHALL return bounded export handles or denied/quota diagnostics without raw private graph values in traces, audits, snapshots, or SDK diagnostics

#### Scenario: Entities are merged
- **WHEN** `graph.merge_entities` is invoked with candidate handles, merge strategy, conflict policy, provenance, and mutation reason
- **THEN** Macaca SHALL validate merge permission, conflict policy, source boundaries, reversibility support, approval requirements, and provider capability before invoking the provider
- **AND** it SHALL emit audit evidence for alias mapping, surviving handle, tombstoned handles, conflict diagnostics, and replay pointers

#### Scenario: Items are deleted
- **WHEN** `graph.delete_graph_items` or `graph.delete_triples` is invoked with selectors and deletion mode
- **THEN** Macaca SHALL require permission, policy, bounded selectors, approval for irreversible deletion, and provider capability before deletion
- **AND** it SHALL return typed tombstone or deletion diagnostics and sanitized audit evidence

### Requirement: Graph Pack SHALL enforce permissions, resource limits, entitlements, approvals, and redaction

`pack.knowledge.graph.v1` SHALL define permission scopes for graph store,
schema, node, edge, RDF, query, traversal, path, merge, import, export,
provenance, and provider inspection. Policy SHALL run before side effects and
SHALL account for source access, schema constraints, query sensitivity, graph
fanout, provider quota, output size, retention, approval, and redaction.

#### Scenario: Query permission is missing
- **WHEN** an application has graph write permission but lacks `graph.query`
- **THEN** Macaca SHALL return a typed denied result for `graph.query`, `graph.traverse`, and `graph.find_path` as applicable
- **AND** the concrete provider SHALL NOT be invoked

#### Scenario: Provenance permission is missing
- **WHEN** an application can read graph items but lacks `graph.provenance.read`
- **THEN** `graph.inspect_provenance` SHALL return a typed denied result or redacted provenance according to policy
- **AND** source identity, extractor identity, citation handles, mutation actor, and lineage details SHALL NOT leak through traces, audits, snapshots, or SDK diagnostics

#### Scenario: Resource limits reject graph expansion
- **WHEN** a query, traversal, path, import, or export exceeds max depth, fanout, rows, memory, timeout, storage, network, provider quota, or output limits
- **THEN** Macaca SHALL return typed quota, timeout, or partial-result diagnostics
- **AND** it SHALL emit bounded resource counters and stable reason codes

#### Scenario: Irreversible merge requires approval
- **WHEN** a merge or delete operation is irreversible, cross-source, high-cardinality, or policy-sensitive
- **THEN** Macaca SHALL return an approval-required result before side effects until a valid approval token is supplied
- **AND** trace/audit evidence SHALL record the approval decision without exposing raw graph values

### Requirement: Graph Pack SHALL expose industrial metadata and developer documentation

`pack.knowledge.graph.v1` SHALL expose descriptor metadata for graph models,
query dialects, schema support, command schemas, permission scopes, policy
templates, resource budgets, approval requirements, import/export formats,
path/traversal support, lifecycle state, compatibility, health probes,
snapshots, unavailable diagnostics, redaction profiles, SDK examples, provider
capability hashes, and developer documentation.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.knowledge.graph.v1`
- **THEN** it SHALL return command namespace `graph.*`, graph models, query dialects, schema support, supported commands, permissions, policy templates, import/export formats, path/traversal capability, examples, lifecycle, availability, health, diagnostics, compatibility, redaction profile, provider capability hash, and documentation links
- **AND** examples SHALL use generic handles and synthetic data rather than application-specific workflows, provider names, credentials, raw graph values, or domain-specific ontologies

#### Scenario: Developer documentation is published
- **WHEN** the pack implementation is marked complete
- **THEN** `docs/developer-packs/knowledge/graph.md` SHALL document manifest declaration, required versus optional behavior, permissions, property graph concepts, RDF graph concepts, store handles, schema lifecycle, node/edge DTOs, triple/quad DTOs, query modes, dialect validation, traversal/path limits, import/export, merge/dedupe, provenance, confidence, validity intervals, unavailable diagnostics, provider replacement, trace/audit interpretation, operational limits, and conformance tests
- **AND** SDK discovery metadata and the industrial catalog index SHALL link to that guide

### Requirement: Graph Pack SHALL be traceable, auditable, replayable, and sanitized

`pack.knowledge.graph.v1` SHALL emit sanitized trace/audit events and bounded
snapshots for declaration, admission, store registration, schema operations,
node/edge mutations, RDF statement mutations, query validation, query execution,
traversal, path finding, merge, delete, import/export, provenance inspection,
provider inspection, policy/resource decisions, provider calls, unavailable
states, and replay.

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a graph pack snapshot
- **THEN** the snapshot SHALL include descriptor version, graph model support, query dialect support, schema version hashes, provider capability hashes, command availability, provider health, policy template hash, quota/resource counters, bounded recent-result statistics, and sanitized replay pointers
- **AND** it SHALL exclude raw credentials, raw provider payloads, raw private graph values, raw source documents, raw execution plans, raw sensitive queries, unbounded output, manifests, package bytes, private keys, and signatures

#### Scenario: Query execution is audited
- **WHEN** `graph.query`, `graph.traverse`, or `graph.find_path` runs
- **THEN** Macaca SHALL emit sanitized audit events with graph store handle, command name, query mode, capability hash, policy decision, resource counters, result status, page/partial markers, latency, and replay pointer
- **AND** raw provider payloads, raw private values, raw execution plans, and unbounded results SHALL NOT enter audit records

#### Scenario: Mutation is audited
- **WHEN** schema, node, edge, triple, delete, merge, import, or export mutations run
- **THEN** Macaca SHALL emit sanitized audit events with stable graph handles, mutation kind, idempotency key hash, provenance handle, approval status where applicable, result code, version hash, and replay pointer
- **AND** raw private graph values and raw provider mutation payloads SHALL NOT enter audit records

### Requirement: Graph Pack implementation SHALL preserve Macaca boundaries

The `pack.knowledge.graph.v1` implementation SHALL remain owned by knowledge
graph service providers behind the service runtime. The microkernel, SDK,
shells, and generic application framework SHALL remain provider-neutral and free
of application-specific or provider-specific routing branches.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete Neo4j, SPARQL endpoint, Neptune, TigerGraph, graph database client, query engine, or provider adapter imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.knowledge.graph.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, capability hash, and result codes rather than provider-specific business branches

#### Scenario: SDK helper builds service call only
- **WHEN** an SDK helper such as `sdk.packs.knowledge.graph.query(command)` is used
- **THEN** the helper SHALL build a canonical traced service call with command DTO, permission metadata, resource limits, and replay context
- **AND** it SHALL NOT construct providers, open graph database connections, rewrite queries based on provider names, or bypass policy
