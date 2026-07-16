## 1. Supplier API Research And Scope

- [x] 1.1 Re-read architecture governance, microkernel boundaries,
  serviceization allowlist, design-pattern guidance, and the industrial catalog
  umbrella proposal before implementation.
- [x] 1.2 Record API notes for Pinecone namespaces/records/query/filter/limits,
  Weaviate collections/named vectors/hybrid/rerank modules, Milvus collections/
  partitions/vector fields/hybrid/range/iterators, Qdrant points/named vectors/
  payload filters/scroll, OpenAI vector stores/file search, and LangChain
  retriever abstractions.
- [x] 1.3 Map supplier concepts to provider-neutral collection, namespace,
  record, chunk, vector-space, query, metadata-filter, hybrid-fusion, candidate,
  rerank, context-window, evidence, cursor, freshness, and provider capability
  DTOs.
- [x] 1.4 Inventory existing service descriptors, SDK clients, admission paths,
  trace/audit schemas, optional providers, mock providers, unavailable providers,
  embedding services, rerank services, citation/evidence services, and
  policy/resource gates that can back retrieval.
- [x] 1.5 Record GitNexus CRITICAL/HIGH findings as memo only before
  implementation commits.

## 2. Contract, Descriptor, And DTOs

- [x] 2.1 Define provider-neutral DTOs for `RetrievalCollection`,
  `RetrievalNamespace`, `RetrievalRecord`, `RetrievalChunk`,
  `RetrievalVectorSpace`, `RetrievalQuery`, `RetrievalMetadataFilter`,
  `RetrievalFusionStrategy`, `RetrievalCandidate`, `RetrievalEvidenceBundle`,
  `RetrievalCursor`, `RetrievalFreshness`, and `RetrievalProviderCapability`.
- [x] 2.2 Define typed command DTOs for `retrieval.register_collection`,
  `retrieval.upsert_records`, `retrieval.delete_records`, `retrieval.retrieve`,
  `retrieval.bulk_retrieve`, `retrieval.retrieve_by_id`,
  `retrieval.range_retrieve`, `retrieval.rerank_context`,
  `retrieval.expand_context`, `retrieval.package_evidence`,
  `retrieval.inspect_collection`, `retrieval.inspect_record`,
  `retrieval.refresh_collection`, and `retrieval.query_diagnostics`.
- [x] 2.3 Define typed success, page, async-handle, evidence-bundle, denied,
  unavailable, unsupported, conflict, quota, timeout, validation, and
  provider-failure result DTOs.
- [x] 2.4 Define descriptor metadata for pack id, collection types, vector-space
  schemas, command schemas, permissions, policy templates, filter support,
  fusion support, namespace/partition support, rerank support, top-k limits,
  score normalization, ACL model, redaction profile, SDK metadata,
  compatibility, diagnostics, and documentation links.
- [x] 2.5 Add descriptor hash, vector-space compatibility, metadata-filter
  validation, ACL filtering, score normalization, redaction-profile, and provider
  capability tests.

## 3. Admission, Permission, Policy, Resource, And Approval

- [x] 3.1 Implement declaration validation for scopes:
  `retrieval.collection.manage`, `retrieval.record.write`,
  `retrieval.query`, `retrieval.read`, `retrieval.evidence`,
  `retrieval.rerank`, `retrieval.metadata.inspect`, and `retrieval.refresh`.
- [ ] 3.2 Enforce collection ownership, secret references, namespace isolation,
  vector-space compatibility, embedding model compatibility, ACL filtering,
  metadata filter validation, top-k budgets, threshold/range limits, context
  window limits, query complexity, timeout, provider capability, rate limit,
  refresh quota, and resource budget checks before provider calls.
- [ ] 3.3 Reject raw credentials, raw provider payloads, raw vectors, raw
  documents, raw chunks beyond redaction policy, raw prompt text, private corpus
  content, and unbounded output at admission and observability boundaries.
- [x] 3.4 Model required declarations as readiness blockers and optional
  declarations as explicit degraded effective capabilities.
- [ ] 3.5 Add tests proving denied, validation, quota, unsupported, and
  unavailable paths do not call concrete retrieval providers.

## 4. Service Provider And Runtime Integration

- [x] 4.1 Implement or bind retrieval providers only through the service runtime
  and approved runtime-host composition roots.
- [x] 4.2 Add unavailable and mock providers with deterministic collection,
  record, vector, filter, hybrid, rerank, context expansion, evidence, refresh,
  diagnostics, and capability behavior.
- [ ] 4.3 Add lifecycle, health, snapshot, shutdown, timeout, cancellation,
  bounded pagination/cursors, idempotent record upsert, delete, bulk retrieval,
  range retrieval, refresh async handles, and collection health support.
- [x] 4.4 Add provider capability reporting for dense/sparse/multivector support,
  named vector spaces, metadata filters, namespaces/partitions, bulk query,
  hybrid fusion, range search, rerank, parent-window expansion, max top-k, max
  filters, rate limits, consistency, and health.
- [x] 4.5 Add canonical execution-path tests proving every retrieval command
  traverses SDK/facade, service runtime decorators, and provider dispatch exactly
  once.

## 5. SDK, WASM ABI, Application Framework, And Examples

- [x] 5.1 Extend SDK discovery for `pack.knowledge.retrieval.v1` with command
  schemas, collection capability reports, examples, availability, diagnostics,
  docs metadata, policy templates, vector-space compatibility, filter/fusion/
  rerank support, top-k limits, namespace limits, and compatibility.
- [x] 5.2 Add focused SDK helper builders that only produce canonical traced
  service calls and return Null Object unavailable diagnostics when the pack is
  absent.
- [ ] 5.3 Extend WASM/application ABI metadata so applications can declare
  retrieval collection access, write records, query, rerank, expand context, and
  package evidence only through declared permissions.
- [x] 5.4 Add generic examples for register collection, upsert records, retrieve,
  hybrid retrieve, metadata-filter retrieve, bulk retrieve, range retrieve,
  retrieve by id, rerank context, expand context, package evidence, inspect
  collection/record, refresh collection, diagnostics, and unavailable provider
  handling.

## 6. Trace, Audit, Replay, Security, And Gates

- [x] 6.1 Emit sanitized declaration, admission, collection registration, record
  upsert/delete, retrieval query, bulk retrieval, range retrieval, rerank, context
  expansion, evidence packaging, collection refresh, diagnostics, policy,
  resource, entitlement, approval, service-call, provider-call, health,
  snapshot, and unavailable events.
- [x] 6.2 Add replay tests proving collection registration, record writes,
  retrieval, hybrid fusion, rerank, context expansion, evidence packaging,
  refresh, and diagnostics are trace-addressable through the canonical service
  path.
- [x] 6.3 Add sanitization tests proving traces, audits, snapshots, SDK
  diagnostics, and examples do not leak raw credentials, raw provider payloads,
  raw vectors, raw chunks beyond policy, private corpus content, raw prompt text,
  or unbounded evidence content.
- [x] 6.4 Add dependency gates proving kernel, SDK, shells, and generic
  application framework do not import concrete retrieval providers or vector
  store adapters.
- [x] 6.5 Run `openspec validate add-pack-knowledge-retrieval --strict`,
  targeted cargo tests, boundary gates, file-size gates, canonical execution-path
  tests, and audit replay checks before marking complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/knowledge/retrieval.md` with pack
  purpose, platform comparison, manifest declaration, permission scopes,
  collection registration, namespace design, vector-space compatibility, record
  upsert/delete, metadata filters, vector/hybrid retrieval, bulk/range retrieval,
  score normalization, reranking, context expansion, evidence packaging,
  ACL filtering, provider replacement, unavailable diagnostics, trace/audit
  interpretation, and operational limits.
- [x] 7.2 Include generic app-facing examples for register collection, upsert,
  retrieve, metadata-filter retrieve, hybrid retrieve, bulk retrieve, rerank,
  expand context, package evidence, inspect collection, refresh, diagnostics,
  and handle unavailable provider results.
- [x] 7.3 Include provider-author guidance for descriptor metadata, vector-space
  schema mapping, embedding compatibility, ACL enforcement, namespace isolation,
  score normalization, redaction, evidence packaging, snapshots, quota
  reporting, and conformance tests.
- [x] 7.4 Cross-link the guide from SDK discovery metadata and the industrial
  pack catalog index before marking `add-pack-knowledge-retrieval` complete.
