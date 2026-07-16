# Knowledge Graph Pack

`pack.knowledge.graph.v1` describes property graph and RDF-style graph
operations through provider-neutral DTOs and canonical service commands.

## Manifest Declaration

```toml
[service_contract]
optional_packs = ["pack.knowledge.graph.v1"]
```

Required declarations block readiness when graph support, query dialects,
permissions, entitlement, or host resources are unavailable.

## Permissions

Use graph-specific scopes for store, schema, node, edge, RDF, query, traversal,
path, merge, import, export, provenance, and provider inspection. Delete,
merge, sensitive provenance, and high-cardinality import/export operations may
require approval.

## Capability Model

DTOs cover graph stores, schemas, nodes, edges, RDF terms/statements,
properties, queries, query results, traversals, paths, import/export plans,
provenance, and provider capability. Provider-native dialects, raw execution
plans, private graph values, source documents, credentials, and unbounded result
sets are not OS semantics.

## Commands

Commands include store registration/inspection, schema upsert/validation,
node/edge upsert, graph deletion, triple upsert/delete, query validation and
execution, traversal, path finding, entity merge, import/export, provenance
inspection, and provider inspection.

Command DTOs use `KnowledgeCommandEnvelope`:

- `subject_ref`: graph store, node, edge, statement, import, export, query, or
  provenance handle.
- `parameters`: bounded provider-neutral options such as dialect, max depth,
  max rows, or dry-run flag.
- `cursor` and `page_size`: pagination controls for bounded query/export
  results.
- `idempotency_key`: required for write, delete, merge, import, and export
  commands that may be retried.

Result DTOs carry a `GraphResultStatus`, optional data, optional page, and
optional `KnowledgeError`. Status values distinguish success, page, partial
result, dry run, validation issue, denied, unavailable, unsupported, conflict,
quota, timeout, cancellation, and failure.

## App-Facing Examples

- Register a graph store with property-graph or RDF model support.
- Upsert a node with labels, properties by reference, and provenance.
- Upsert an edge with source/target node ids and audit-safe properties.
- Upsert an RDF triple using RDF term DTOs.
- Validate a portable query before execution.
- Traverse from a start node with bounded depth and fanout.
- Export a bounded subgraph with redaction profile metadata.
- Inspect provenance before showing sensitive source evidence.
- Handle unavailable or denied diagnostics without using provider-native graph
  database clients, provider names, credentials, application-specific
  workflows, domain-specific ontologies, sensitive queries, private graph
  values, or unbounded result sets.

## Supplier Mapping

Neo4j/Cypher concepts map to graph store, schema, node, edge, property, query,
path, and traversal DTOs. RDF/SPARQL concepts map to RDF term, statement,
named graph, query, and export plan DTOs. Neptune-style multi-model support maps
to provider capability metadata. TigerGraph/GSQL-style schema, loading, and
query concepts map to schema, import plan, and provider capability descriptors.
None of these supplier dialects become OS routing branches.

## Trace And Audit

Trace metadata should include store id, command name, graph model, dialect,
provider class, capability hash, row/depth/fanout limits, redaction profile,
and result status. Raw credentials, provider payloads, private graph values,
source documents, execution plans, sensitive queries, and unbounded results are
forbidden in observability.

## Provider Authors

Provider descriptors must report graph models, query dialects, import/export
formats, traversal/path limits, merge behavior, provenance visibility, resource
bounds, redaction, health, snapshots, unavailable behavior, and conformance
tests. Query and import/export adapters are Strategy implementations behind the
service runtime, never SDK or shell code.

## Conformance Checklist

- Descriptor metadata includes graph models, dialects, formats, permissions,
  policy templates, diagnostics, compatibility, and redaction profile.
- Command DTOs stay provider-neutral and all side-effecting commands carry
  idempotency metadata.
- Query validation enforces row, depth, fanout, timeout, and redaction bounds
  before provider execution.
- Import/export and merge paths return dry-run, conflict, denied, unavailable,
  timeout, and quota diagnostics without provider payloads.
- Trace, audit, snapshot, and SDK diagnostics exclude private values, sensitive
  queries, execution plans, and unbounded result sets.
