# Knowledge Graph Pack Design

## Context

`pack.knowledge.graph.v1` exposes graph storage and graph computation as a
Macaca OS serviceized capability. It lets applications manage connected
knowledge without embedding Neo4j, SPARQL, Neptune, TigerGraph, Gremlin, GSQL,
or custom ontology behavior into generic OS layers.

Graph capability is broader than search or retrieval. Search locates documents
or chunks; retrieval returns ranked evidence; document parsing extracts
structure; citations preserve evidence anchors. Graph stores typed entities,
relationships, RDF statements, schemas, provenance, and traversable structure.
The pack therefore needs first-class contracts for both property graphs and RDF
graphs, plus strong policy and bounded execution because graph queries can touch
large connected datasets.

## Supplier Capability Matrix

| Supplier/standard | Industrial capability | Macaca abstraction |
| --- | --- | --- |
| Neo4j/Cypher | Nodes, relationships, labels, relationship types, properties, constraints, indexes, `MATCH`, `MERGE`, path patterns, query plans | Property graph model, node/edge DTOs, schema constraints, provider query validation, bounded path query, capability inspection |
| W3C RDF 1.1 | IRIs, blank nodes, literals, triples, named graphs, datasets, datatypes, language tags | RDF term DTO, triple/quad DTO, named graph handle, datatype/language metadata, RDF dataset metadata |
| W3C SPARQL 1.1 | Graph patterns, optional patterns, source graph constraints, aggregation, subqueries, result sets, RDF graph results | Provider-neutral query request, RDF query dialect profile, query validation, result binding/page DTO, graph output DTO |
| Amazon Neptune | RDF/SPARQL and property graph APIs such as Gremlin/openCypher, managed graph operational constraints | Multi-model provider capability, dialect matrix, endpoint health, quota/rate diagnostics, transaction/support metadata |
| TigerGraph/GSQL | Vertex/edge types, attributes, graph schema, loading jobs, graph queries | Schema definition, bulk import plan, graph load diagnostics, traversal/query capability, typed vertex/edge metadata |

The pack uses these sources to define a provider-neutral OS contract. Provider
adapters translate between Macaca DTOs and concrete supplier APIs. OS-layer code
must not branch on provider names or business ontologies.

## Goals

- Provide stable pack id `pack.knowledge.graph.v1` and command namespace
  `graph.*`.
- Support property graph and RDF graph models through one provider-neutral
  descriptor.
- Support graph store registration, schema inspection/upsert, node/entity
  upsert, edge/relation upsert, RDF triple/quad upsert, deletion, graph query,
  query validation, traversal, bounded path query, entity merge, subgraph
  import/export, provenance inspection, and provider capability inspection.
- Preserve graph provenance across source, extractor, citation, confidence,
  validity interval, and mutation reason without exposing raw private data.
- Keep concrete graph engines behind replaceable service providers.
- Require developer documentation at
  `docs/developer-packs/knowledge/graph.md`.

## Non-Goals

- Do not implement concrete graph engine adapters in this proposal.
- Do not define a universal business ontology, finance graph, code graph, social
  graph, product graph, or application-specific schema.
- Do not expose raw provider query payloads, credentials, source documents, raw
  private values, provider execution plans, or unbounded subgraphs in traces,
  audits, snapshots, SDK diagnostics, or examples.
- Do not allow shell UI, SDK helpers, or application framework code to own query
  rewriting, graph repair, dedupe heuristics, or provider fallback semantics.
- Do not guarantee global ACID semantics across providers; expose provider
  consistency and transaction capabilities explicitly.

## Strategy Hooks

The runtime-host graph bridge uses explicit Strategy interfaces for the three
extension points that vary across graph providers:

- `GraphQueryValidationStrategy` validates bounded opaque query envelopes for
  portable, Cypher-like, SPARQL-like, Gremlin-like, GSQL-like, and
  provider-declared modes. It does not parse or log provider-native query text.
- `GraphImportExportStrategy` validates opaque import handles and bounded
  provider-neutral formats (`graph_bundle`, `rdf_dataset`, `json_ld_like`, and
  `csv_like`) before any provider adapter can read or write data.
- `GraphMergeStrategy` evaluates reference-only merge/conflict requests and
  emits a deterministic reversible alias reference when requested. It never
  inspects graph values or embeds ontology-specific dedupe behavior.

The service provider selects these Strategies by declared mode or format, not by
provider name. Every rejection is emitted as a policy decision and prevents
reference allocation, preserving traceability and fail-closed behavior.

## Ownership And Boundaries

- Pack id: `pack.knowledge.graph.v1`.
- Family: `knowledge`.
- Backing service owner: knowledge graph service provider.
- SDK surface: `sdk.packs.knowledge.graph`.
- Command namespace: `graph.*`.
- Microkernel owns identity, policy facade, resource primitives, service-call
  evidence, trace/audit primitives, and registry metadata only.
- Application framework owns manifest declarations, app-scoped permissions, and
  effective capability projection.
- Runtime host owns provider adapter registration, optional provider
  composition, decorators, and sanitized diagnostics through approved
  composition roots.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `graph.register_store` | Register or bind an app-visible graph store handle | Validates model, tenant scope, retention, provider capability, and entitlement |
| `graph.inspect_store` | Inspect graph store metadata and health | Returns bounded schema, model, provider capability, and diagnostics |
| `graph.upsert_schema` | Create/update labels, types, properties, constraints, indexes, or RDF vocabulary metadata | Requires compatibility validation and migration diagnostics |
| `graph.validate_schema` | Validate proposed data/query against schema/capability | Returns validation issues without side effects |
| `graph.upsert_node` | Upsert property graph node/entity | Requires idempotency, label/type validation, property policy, and provenance |
| `graph.upsert_edge` | Upsert property graph edge/relationship | Requires endpoint validation, relationship type validation, direction, and provenance |
| `graph.delete_graph_items` | Delete or tombstone nodes, edges, triples, quads, or subgraphs | Requires approval for irreversible deletion and emits tombstone audit |
| `graph.upsert_triple` | Upsert RDF triple or quad | Validates subject, predicate, object, named graph, datatype, language, and provenance |
| `graph.delete_triples` | Delete RDF triples/quads by selector | Requires bounded selector and policy check |
| `graph.query` | Execute provider-neutral graph query plan or declared provider dialect query | Requires query validation, bounded result limits, and sanitized result pages |
| `graph.validate_query` | Validate query syntax, dialect, parameters, cost bounds, and permissions | Must not execute data access beyond validation policy |
| `graph.traverse` | Traverse from start nodes/terms using bounded edge/predicate/path filters | Enforces max depth, fanout, timeout, and result budget |
| `graph.find_path` | Find bounded shortest/weighted/path-pattern paths | Reports capability limits and partial results |
| `graph.merge_entities` | Merge/dedupe entities or terms with provenance | Requires conflict policy, reversible mapping where possible, and audit reason |
| `graph.import_subgraph` | Import property graph, RDF, JSON-LD-like, CSV-like, or provider-neutral graph bundle | Requires validation, load plan, quota, and partial failure reporting |
| `graph.export_subgraph` | Export bounded subgraph by selector/query/provenance/source | Requires export permission, redaction, and output-size limits |
| `graph.inspect_provenance` | Inspect source, mutation, confidence, validity, and lineage | Returns bounded provenance handles and replay pointers |
| `graph.inspect_provider` | Inspect model, dialect, transaction, path, schema, import/export, quota, and health capability | Returns sanitized capability metadata only |

Every command must define typed command DTOs, typed success results, typed
partial results for paged/streamed operations, typed validation/denied/conflict/
quota/unavailable/unsupported/failure results, idempotency semantics for side
effects, redaction profile, and replay metadata.

## DTO Model

Core DTOs:

- `GraphStore`: store handle, model (`property_graph`, `rdf`, or `multi_model`),
  tenant/application scope, retention class, consistency profile, transaction
  profile, schema version, provider capability hash, health, and quota metadata.
- `GraphSchema`: labels, vertex types, edge types, relationship types, RDF
  vocabularies, property definitions, datatypes, cardinality, constraints,
  indexes, uniqueness rules, compatibility version, and migration diagnostics.
- `GraphNode`: node handle, stable external key, labels/types, properties,
  source handles, confidence, validity interval, provenance, version hash, and
  redaction class.
- `GraphEdge`: edge handle, source node, target node, direction, relationship
  type, properties, confidence, validity interval, provenance, version hash, and
  redaction class.
- `RdfTerm`: IRI, blank node handle, literal value handle, datatype, language,
  namespace profile, redaction class, and validation status.
- `RdfStatement`: subject, predicate, object, named graph handle, provenance,
  confidence, validity interval, version hash, and redaction class.
- `GraphProperty`: name, value handle, type, cardinality, sensitivity class,
  validation status, and source attribution.
- `GraphQuery`: query mode (`portable`, `cypher_like`, `sparql`, `gremlin_like`,
  `gsql_like`, or provider-declared), parameters, max rows, max depth, timeout,
  cost budget, redaction profile, and dialect capability hash.
- `GraphQueryResult`: result kind, columns/bindings/path/subgraph handles,
  page token, warnings, partial status, cost counters, and replay pointer.
- `GraphTraversal`: start selectors, edge/predicate filters, node/term filters,
  direction, max depth, max fanout, uniqueness policy, stop conditions, and
  result budget.
- `GraphPath`: ordered nodes/terms, edges/statements, weights, path cost,
  constraint satisfaction, confidence, and redaction profile.
- `GraphImportPlan`: input format, source handle, schema mapping, dry-run flag,
  validation report, batch policy, conflict policy, and load diagnostics.
- `GraphExportPlan`: selector/query/provenance filters, output format, redaction
  profile, limit policy, destination handle, and export diagnostics.
- `GraphProvenance`: source handle, extractor handle, citation handle, mutation
  actor, mutation reason, event trace id, confidence, evidence quality, validity
  interval, and lineage pointers.
- `GraphProviderCapability`: graph models, query dialects, schema support,
  constraint/index support, transaction capability, bulk load capability, path
  algorithms, import/export formats, max depth/fanout/result size, consistency,
  rate limits, lifecycle, and health.

Provider-specific extensions may appear only as bounded `adapter_metadata`
behind capability hashes and must not drive OS-layer routing.

## Permission, Policy, Resource, Entitlement, And Approval Model

Permission scopes:

- `graph.store.read`
- `graph.store.manage`
- `graph.schema.read`
- `graph.schema.write`
- `graph.node.read`
- `graph.node.write`
- `graph.edge.read`
- `graph.edge.write`
- `graph.rdf.read`
- `graph.rdf.write`
- `graph.query`
- `graph.traverse`
- `graph.path`
- `graph.merge`
- `graph.import`
- `graph.export`
- `graph.provenance.read`
- `graph.provider.inspect`

Policy defaults:

- Every command is scoped to application id, tenant id, session id, task id, and
  trace id when available.
- Query, traversal, and path commands require bounded max rows, max depth,
  fanout, timeout, and memory budgets before provider calls.
- Write commands require schema/capability validation, idempotency keys,
  provenance, and conflict policy.
- Delete and merge commands require explicit approval when irreversible,
  cross-source, high-cardinality, or policy-sensitive.
- Import/export commands require source/destination permission, redaction
  profile, quota, and output-size limits.
- Provenance access requires separate permission because it can reveal source
  identity, extraction lineage, or mutation history.
- Raw credentials, raw provider payloads, raw private properties, raw source
  documents, raw execution plans, unbounded queries, and unbounded subgraphs are
  forbidden in observability.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, family, version, lifecycle, command schemas,
graph models, query dialects, schema support, path/traversal support, import and
export formats, permission scopes, policy templates, resource limits, approval
rules, provider capability hashes, health, compatibility, diagnostics, example
commands, redaction profiles, and documentation links.

The developer guide at `docs/developer-packs/knowledge/graph.md` must cover:

- manifest declaration and optional/required behavior
- property graph versus RDF graph concepts
- graph store handles and schema lifecycle
- node/edge and RDF triple/quad DTOs
- query modes, dialect validation, traversal, path limits, and pagination
- import/export formats and dry-run validation
- entity merge/dedupe policy and conflict handling
- provenance, citations, confidence, validity intervals, and replay pointers
- permissions, approvals, quota, unavailable diagnostics, and structured errors
- provider replacement notes and capability inspection
- trace/audit interpretation and conformance tests

Examples must use generic synthetic entities and statements. They must not bake
in provider names, application names, credentials, business workflows, or
domain-specific ontologies.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `graph_pack_declared`
- `graph_pack_admission_validated`
- `graph_store_registered`
- `graph_store_inspected`
- `graph_schema_upserted`
- `graph_schema_validated`
- `graph_node_upserted`
- `graph_edge_upserted`
- `graph_items_deleted`
- `graph_triple_upserted`
- `graph_triples_deleted`
- `graph_query_validated`
- `graph_query_executed`
- `graph_traversal_executed`
- `graph_path_found`
- `graph_entities_merged`
- `graph_subgraph_imported`
- `graph_subgraph_exported`
- `graph_provenance_inspected`
- `graph_provider_inspected`
- `graph_pack_policy_decision`
- `graph_pack_service_call_requested`
- `graph_pack_service_call_succeeded`
- `graph_pack_service_call_failed`
- `graph_pack_unavailable`
- `graph_pack_snapshot_recorded`

Snapshots include descriptor version, graph model support, query dialect support,
schema version hashes, provider capability hashes, command availability,
provider health, policy template hash, quota/resource counters, recent bounded
result statistics, and sanitized replay pointers. Snapshots must exclude raw
queries when policy marks them sensitive, raw provider payloads, credentials,
raw private graph values, raw source documents, raw execution plans, and
unbounded results.

## Design Patterns

- **Facade**: SDK clients expose discovery and command builders only; callers do
  not construct providers.
- **Command**: every operation is a typed command/result DTO with structured
  errors and replay metadata.
- **Strategy**: graph model adapters, query dialect validators, import/export
  adapters, merge policies, conflict policies, and unavailable behavior are
  replaceable.
- **Decorator**: trace, policy, entitlement, resource, approval, metering,
  redaction, query limits, and provenance checks wrap service calls.
- **Specification**: admission validates pack declaration, graph model, schema,
  command availability, permission, provider capability, version compatibility,
  and policy constraints.
- **Observer**: graph mutations, query events, provider health, trace events,
  and audit events are subscribable.
- **Memento**: effective capability reports, schema version hashes, mutation
  version hashes, snapshots, and replay pointers preserve recovery state.
- **Abstract Factory**: concrete graph providers and validators are created only
  by approved runtime-host composition roots.

## Risks And Mitigations

- Risk: graph query support becomes raw provider pass-through. Mitigation:
  require query mode metadata, validation, capability hashes, redaction, and
  bounded result DTOs; provider dialects are adapter inputs, not OS semantics.
- Risk: traversals explode resource usage. Mitigation: mandatory max depth,
  fanout, timeout, row, memory, and output limits before provider calls.
- Risk: RDF and property graph concepts are forced into one weak model.
  Mitigation: model both explicitly under a `GraphStore` model profile and expose
  multi-model provider capability rather than pretending all stores are equal.
- Risk: merge/dedupe corrupts graph lineage. Mitigation: require conflict
  policy, provenance, audit reason, reversible mapping where possible, and
  explicit approval for irreversible merges.
- Risk: provider capabilities leak provider-specific branches into OS layers.
  Mitigation: route via descriptors, capability hashes, Strategy adapters, and
  boundary gates that reject provider imports in kernel, SDK, shells, and generic
  application framework.
