# Change: Add Knowledge Graph Pack

## Why

Developers need `pack.knowledge.graph.v1` as a real industrial graph capability,
not a catalog label. A production application should be able to declare the pack
and receive provider-neutral services for graph stores, typed schemas, entity and
relationship writes, RDF triples/quads, graph queries, traversals, path queries,
subgraph export, import, provenance, and capability inspection.

The pack must support both dominant graph families:

- Property graph systems such as Neo4j/Cypher and TigerGraph/GSQL model nodes or
  vertices, relationships or edges, labels/types, properties, constraints,
  indexes, traversals, and path queries.
- RDF graph systems and standards such as RDF 1.1 and SPARQL 1.1 model IRIs,
  blank nodes, literals, triples, named graphs, graph patterns, query result
  sets, and graph outputs.

Macaca must expose those concepts through a stable serviceized abstraction. The
kernel, SDK, shells, and generic application framework must remain provider
neutral, policy checked, traceable, auditable, replayable, and free of
application-specific graph logic.

## Research And Supplier/API Baseline

Official supplier and standards references considered for this pack:

- Neo4j Cypher Manual: Cypher is a declarative query language for property graph
  databases, with documented clauses for `MATCH`, `CREATE`, `MERGE`, `DELETE`,
  `RETURN`, path patterns, indexes, constraints, query plans, and compatibility.
  Reference: https://neo4j.com/docs/cypher-manual/current/
- W3C SPARQL 1.1 Query Language: SPARQL defines syntax and semantics for
  querying RDF graphs, graph patterns, optional patterns, aggregation,
  subqueries, negation, source graph constraints, result sets, and RDF graph
  results. Reference: https://www.w3.org/TR/sparql11-query/
- W3C RDF 1.1 Concepts: RDF defines directed labeled graph data with IRIs, blank
  nodes, literals, triples, named graphs, datasets, datatypes, and language tags.
  Reference: https://www.w3.org/TR/rdf11-concepts/
- Amazon Neptune documentation: Neptune supports graph database workloads across
  RDF/SPARQL and property graph APIs such as Gremlin and openCypher. Reference:
  https://docs.aws.amazon.com/neptune/latest/userguide/intro.html
- TigerGraph GSQL documentation: GSQL covers graph schema definition, loading,
  querying, vertex and edge types, attributes, and graph workflow from schema to
  load to query. References:
  https://docs.tigergraph.com/gsql-ref/4.2/intro/ and
  https://docs.tigergraph.com/gsql-ref/4.2/ddl-and-loading/

The Macaca abstraction must not clone any provider API. It must map supplier
capabilities into provider-neutral DTOs and commands while retaining enough
metadata for industrial use: graph model, schema, constraints, query dialect
support, path capabilities, provenance, import/export formats, consistency
semantics, quotas, and health.

## What Changes

- Add provider-neutral `pack.knowledge.graph.v1` under the `knowledge` family.
- Define graph store, property graph, RDF graph, schema, constraint, query,
  traversal, path, import/export, merge, provenance, capability, and diagnostic
  DTOs.
- Define command namespace `graph.*` with industrial commands for:
  - graph store registration and inspection
  - schema upsert and schema validation
  - entity/node/vertex upsert and delete
  - edge/relationship upsert and delete
  - RDF triple/quad upsert and delete
  - provider-neutral graph query and provider-dialect query validation
  - traversal and bounded path queries
  - entity merge/dedupe
  - subgraph import/export
  - provenance inspection
  - provider capability inspection
- Define permission scopes, policy defaults, resource budgets, approval rules,
  entitlement checks, structured unavailable behavior, SDK discovery, developer
  documentation, trace/audit events, snapshots, replay, and boundary gates.
- Require detailed developer documentation at
  `docs/developer-packs/knowledge/graph.md` before implementation completion.

## Impact

- Affected specs: `pack-knowledge-graph`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral protocol DTOs, pack descriptors,
  admission validators, SDK discovery and command builders, knowledge graph
  service provider or unavailable provider, runtime-host provider adapters,
  trace/audit schemas, replay tests, dependency-boundary gates, and developer
  documentation.
- Non-goals: no concrete Neo4j, Neptune, TigerGraph, RDF store, or graph driver
  implementation in this proposal; no application-specific ontology or workflow;
  no provider-name routing in OS layers; no raw query/provider payloads in
  observability; no SDK/shell/kernel provider construction; no fake success when
  provider, entitlement, permission, or host support is absent.
