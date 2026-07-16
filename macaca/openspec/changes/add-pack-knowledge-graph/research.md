# Knowledge Graph Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
explicit non-goals, existing platform inventory, and GitNexus memo evidence for
`pack.knowledge.graph.v1`. Graph support must expose property graph, RDF graph,
query, traversal, path, import/export, merge, schema, and provenance operations
through serviceized commands. It must not hardcode application ontologies or
pass provider query engines directly into SDK or shell code.

## Source Baseline

- Neo4j Cypher manual for nodes, relationships, labels, properties, constraints,
  indexes, `MATCH`, `MERGE`, patterns, and planning:
  <https://neo4j.com/docs/cypher-manual/current/>
- W3C RDF 1.1 Concepts and SPARQL 1.1 Query:
  <https://www.w3.org/TR/rdf11-concepts/>
  and <https://www.w3.org/TR/sparql11-query/>
- Amazon Neptune user guide for RDF/SPARQL and property graph support:
  <https://docs.aws.amazon.com/neptune/latest/userguide/>
- TigerGraph GSQL language reference:
  <https://docs.tigergraph.com/gsql-ref/current/intro/>

## Supplier API Notes

- Neo4j/Cypher contributes property graph nodes, relationships, labels,
  properties, indexes, constraints, path patterns, `MATCH`, `MERGE`, planning,
  and query compatibility. Macaca should normalize graph schema, nodes, edges,
  path/traversal requests, constraints, and query-dialect metadata.
- RDF 1.1 and SPARQL 1.1 contribute RDF terms, IRIs, blank nodes, literals,
  triples, named graphs, datasets, graph patterns, result sets, and graph
  outputs. Macaca should model RDF terms/statements and query results without
  collapsing RDF into property-graph-only DTOs.
- Amazon Neptune contributes managed multi-model operations, SPARQL, Gremlin,
  openCypher-style property graph access, health, endpoints, query status,
  bulk loading, and provider limits. Macaca should surface provider capability,
  lifecycle, health, quota, cancellation, and unavailable diagnostics.
- TigerGraph/GSQL contributes vertex/edge types, attributes, schema definition,
  loading jobs, accumulators/query language, and graph-specific capability.
  Macaca should map these into schema, import plan, and provider capability DTOs.

## Macaca-Owned Abstractions

`pack.knowledge.graph.v1` should define `GraphStore`, `GraphSchema`,
`GraphNode`, `GraphEdge`, `RdfTerm`, `RdfStatement`, `GraphProperty`,
`GraphQuery`, `GraphQueryResult`, `GraphTraversal`, `GraphPath`,
`GraphImportPlan`, `GraphExportPlan`, `GraphProvenance`, and
`GraphProviderCapability`.

The DTOs must support property graph and RDF graph models, query dialect
declaration, schema validation, path/traversal limits, import/export planning,
merge/conflict behavior, provenance, redaction, provider health, and replay.
Raw provider queries, raw execution plans, private graph values, source
documents, credentials, and unbounded result sets are rejected.

## Explicit Non-Goals

- Do not implement concrete Neo4j, Neptune, TigerGraph, RDF store, SPARQL,
  Cypher, Gremlin, GSQL, or database-client providers in the research phase.
- Do not define application ontologies, CRM graphs, finance graphs, code graphs,
  workflow graphs, or domain-specific merge heuristics in OS-layer code.
- Do not expose provider-native query plans, credentials, raw graph payloads, or
  provider-specific ids as stable SDK contracts.
- Do not let shells or WASM adapters bypass graph service policy with raw
  provider queries.

## Existing Macaca Platform Inventory

- Generic service descriptors, domain-pack registration, `SystemFacade`,
  trace-required service calls, unavailable providers, policy/resource command
  objects, persistence snapshots, and dependency gates provide the substrate for
  a future graph service.
- Knowledge retrieval, citations, document parsing, and summarization proposals
  define adjacent source/evidence handles that graph must use through declared
  capabilities rather than direct coupling.
- No current evidence proves graph-specific DTOs, providers, query validators,
  import/export adapters, merge strategies, SDK helpers, or docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
