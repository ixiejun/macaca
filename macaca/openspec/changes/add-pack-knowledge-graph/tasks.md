## 1. Supplier API Research And Scope

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design patterns, OpenSpec rules, and the industrial catalog umbrella proposal before implementation.
- [x] 1.2 Study official Neo4j/Cypher docs for property graph nodes, relationships, labels, properties, indexes, constraints, `MATCH`, `MERGE`, path patterns, query planning, and compatibility.
- [x] 1.3 Study W3C RDF 1.1 and SPARQL 1.1 docs for RDF terms, triples, named graphs, datasets, graph patterns, result sets, graph outputs, and query semantics.
- [x] 1.4 Study Amazon Neptune docs for multi-model RDF/SPARQL and property graph support, managed operational limits, health, endpoint behavior, and provider capability metadata.
- [x] 1.5 Study TigerGraph/GSQL docs for vertex/edge types, attributes, graph schema, loading jobs, and query capability.
- [x] 1.6 Produce a supplier capability comparison memo mapping property graph, RDF graph, query dialect, schema, import/export, path/traversal, transaction, and provenance capabilities into Macaca provider-neutral abstractions.
- [x] 1.7 Define explicit non-goals for provider implementations, application ontologies, raw provider pass-through, and application-specific workflows.
- [x] 1.8 Record GitNexus CRITICAL/HIGH findings as memo only before implementation commits.

## 2. Contract, Descriptor, And DTOs

- [x] 2.1 Define `pack.knowledge.graph.v1` descriptor metadata: pack id, family, lifecycle, stability, graph model support, command schemas, permission scopes, policy templates, resource budgets, approval requirements, data-governance class, SDK metadata, documentation link, compatibility, and diagnostics.
- [x] 2.2 Define provider-neutral DTOs for `GraphStore`, `GraphSchema`, `GraphNode`, `GraphEdge`, `RdfTerm`, `RdfStatement`, `GraphProperty`, `GraphQuery`, `GraphQueryResult`, `GraphTraversal`, `GraphPath`, `GraphImportPlan`, `GraphExportPlan`, `GraphProvenance`, and `GraphProviderCapability`.
- [x] 2.3 Define typed command/result DTOs for `graph.register_store`, `graph.inspect_store`, `graph.upsert_schema`, `graph.validate_schema`, `graph.upsert_node`, `graph.upsert_edge`, `graph.delete_graph_items`, `graph.upsert_triple`, `graph.delete_triples`, `graph.query`, `graph.validate_query`, `graph.traverse`, `graph.find_path`, `graph.merge_entities`, `graph.import_subgraph`, `graph.export_subgraph`, `graph.inspect_provenance`, and `graph.inspect_provider`.
- [x] 2.4 Define typed success, paged result, partial result, dry-run result, validation issue, denied, unavailable, unsupported, conflict, quota, timeout, cancellation, and failure DTOs.
- [x] 2.5 Define stable descriptor hashing, provider capability hashing, schema version hashing, query dialect compatibility, and result redaction metadata.
- [x] 2.6 Add descriptor and DTO compatibility tests for valid descriptors, rejected invalid descriptors, stable hashes, schema evolution, redaction profiles, and serde compatibility.

## 3. Admission, Permission, Policy, Resource, Entitlement, And Approval

- [x] 3.1 Implement manifest declaration validation for required and optional `pack.knowledge.graph.v1` declarations.
- [x] 3.2 Implement permission validation for `graph.store.read`, `graph.store.manage`, `graph.schema.read`, `graph.schema.write`, `graph.node.read`, `graph.node.write`, `graph.edge.read`, `graph.edge.write`, `graph.rdf.read`, `graph.rdf.write`, `graph.query`, `graph.traverse`, `graph.path`, `graph.merge`, `graph.import`, `graph.export`, `graph.provenance.read`, and `graph.provider.inspect`.
- [ ] 3.3 Implement policy checks before side effects for source access, schema compatibility, write idempotency, query sensitivity, provenance visibility, delete approval, merge approval, import/export redaction, and provider capability.
- [ ] 3.4 Implement resource reservation for max rows, max depth, max fanout, timeout, memory, storage, network, provider quota, import batch size, export size, and retained snapshots.
- [ ] 3.5 Implement entitlement checks and structured unavailable/denied diagnostics for missing provider, disabled pack, missing entitlement, missing permission, absent graph model, unsupported query dialect, unsupported import/export format, disabled network, and host resource denial.
- [ ] 3.6 Implement approval behavior for irreversible deletion, irreversible merge, cross-source merge, high-cardinality import/export, sensitive provenance disclosure, and long-running graph jobs.
- [ ] 3.7 Add tests proving denied, validation, quota, unavailable, and approval-required paths do not call concrete providers.

## 4. Service Provider And Runtime Integration

- [x] 4.1 Implement or bind the knowledge graph service provider behind the service runtime; do not construct graph providers from SDK, shell, kernel, or application code.
- [x] 4.2 Add a deterministic unavailable provider that returns typed unavailable/unsupported diagnostics and complete discovery metadata.
- [x] 4.3 Add mock provider support for property graph commands, RDF commands, query validation, traversal, path, import/export dry-run, merge conflict, provenance inspection, and provider capability inspection.
- [ ] 4.4 Add lifecycle, health, snapshot, shutdown, timeout, cancellation, bounded streaming, and paginated result behavior.
- [ ] 4.5 Add query validation adapters as Strategy implementations for portable graph query, Cypher-like, SPARQL, Gremlin-like, GSQL-like, and provider-declared modes without hardcoding provider names in OS routing.
- [ ] 4.6 Add import/export adapters as Strategy implementations for provider-neutral graph bundles, RDF-like datasets, JSON-LD-like documents, CSV-like loads, and paged export handles.
- [ ] 4.7 Add merge/conflict Strategy hooks for deterministic entity merge, reversible alias mapping where possible, conflict diagnostics, and audit reasons.
- [ ] 4.8 Add provider capability reporting for available, degraded, preview, unavailable, unsupported, retired, model-specific, dialect-specific, and quota-limited states.

## 5. SDK, WASM ABI, Application Framework, And Examples

- [x] 5.1 Extend SDK discovery for `pack.knowledge.graph.v1` with command schemas, graph models, query dialects, import/export formats, path/traversal support, examples, availability, diagnostics, documentation link, provider class, capability hash, compatibility, and redaction profile.
- [x] 5.2 Extend application admission so required declarations block readiness when unavailable and optional declarations degrade explicitly with effective capability mementos.
- [x] 5.3 Add SDK command helper builders for all `graph.*` commands; helpers must only build canonical traced service calls and must never construct providers or bypass policy.
- [ ] 5.4 Extend WASM/app ABI descriptors so applications can discover graph commands, declare permissions, receive unavailable diagnostics, and submit typed service calls through the canonical execution path.
- [x] 5.5 Add generic app-facing examples for registering a graph store, upserting a node, upserting an edge, upserting an RDF triple, validating a query, traversing from a start node, exporting a bounded subgraph, and inspecting provenance.
- [x] 5.6 Add unavailable-provider and denied-policy examples that demonstrate diagnostics without provider names, credentials, application-specific workflows, or domain-specific ontologies.

## 6. Trace, Audit, Replay, Security, And Gates

- [ ] 6.1 Emit sanitized declaration, admission, policy, entitlement, resource, approval, service-call, mutation, query, traversal, path, import/export, merge, provenance, health, snapshot, unavailable, and failure events.
- [x] 6.2 Ensure traces, audits, snapshots, SDK diagnostics, and examples exclude raw credentials, raw provider payloads, raw private graph values, raw source documents, raw execution plans, raw sensitive queries, unbounded outputs, package bytes, manifests, private keys, and signatures.
- [x] 6.3 Add replay tests proving every `graph.*` command is trace-addressable through the canonical service path and that snapshots contain enough bounded metadata for recovery diagnostics.
- [x] 6.4 Add dependency gates proving kernel, SDK, shells, and generic application framework do not import concrete graph providers, query engines, database clients, or provider-specific adapters.
- [x] 6.5 Add no-direct-provider-call gates proving SDK helpers, WASM ABI handlers, app admission, web, CLI, and frontend paths route through descriptor-owned service commands.
- [x] 6.6 Add boundary tests proving optional provider absence returns structured unavailable diagnostics and never crashes, hangs, silently falls back, or fakes success.
- [x] 6.7 Run `openspec validate add-pack-knowledge-graph --strict`, targeted cargo tests, boundary gates, file-size gates, and audit replay checks before marking implementation complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/knowledge/graph.md` with purpose, capability model, manifest declaration, required versus optional behavior, permissions, graph store handles, property graph concepts, RDF concepts, schema lifecycle, node/edge DTOs, triple/quad DTOs, query modes, traversal/path limits, import/export, merge/dedupe, provenance, confidence, validity intervals, trace/audit interpretation, unavailable diagnostics, provider replacement, and operational limits.
- [x] 7.2 Document all command DTOs and result DTOs with field-level explanations, idempotency semantics, redaction behavior, pagination/streaming behavior, timeout/cancellation behavior, and structured error codes.
- [x] 7.3 Document supplier/API mapping: Neo4j/Cypher, RDF/SPARQL, Neptune, and TigerGraph/GSQL concepts mapped to Macaca abstractions, including what is intentionally not exposed as OS semantics.
- [x] 7.4 Add generic examples for property graph upsert/query/traversal and RDF triple/query/export using synthetic data only.
- [x] 7.5 Add conformance checklist and test guidance for provider authors: descriptor completeness, command support, query validation, resource bounds, policy hooks, trace/audit events, unavailable behavior, snapshot/replay, and redaction.
- [x] 7.6 Cross-link the guide from SDK discovery metadata and the industrial pack catalog index before marking `add-pack-knowledge-graph` complete.
