# Knowledge Retrieval Pack Design

## Context

`pack.knowledge.retrieval.v1` exposes vector, hybrid, and evidence-oriented
retrieval as a Macaca OS serviceized capability. It lets applications retrieve
relevant chunks or records from declared collections without hardcoding
Pinecone, Weaviate, Milvus, Qdrant, OpenAI vector stores, LangChain retrievers,
or future retrieval providers into OS layers.

Retrieval is a policy-sensitive bridge between private corpora and AI context.
It can leak source text, vector neighborhoods, metadata, access-control state,
and sensitive prompt context. The pack therefore makes collection registration,
record writes, vector/hybrid queries, reranking, parent-context expansion, and
evidence packaging typed commands behind service runtime decorators.

## Supplier Capability Matrix

| Supplier/platform | Industrial capability | Macaca abstraction |
| --- | --- | --- |
| Pinecone | Indexes, namespaces, dense/sparse records, metadata filters, upsert/update/fetch/query/delete, top-k, multitenancy, rate limits | Retrieval collection, namespace, vector record, metadata filter AST, top-k budget, tenant isolation, quota diagnostics |
| Weaviate | Collections, objects, named vectors, vector/BM25/hybrid search, filters, modules, reranking | Collection descriptor, named vector spaces, lexical/vector/hybrid strategy, module capability, rerank delegation |
| Milvus | Collections, partitions, vector fields, scalar filters, single/bulk/multivector search, range search, iterators, limits | Partition scope, vector field descriptor, bulk query, hybrid fusion, range threshold, cursor iterator, limit diagnostics |
| Qdrant | Collections, points, named dense/sparse/multivectors, payload filters/indexes, scroll/search, hybrid retrieval | Point handle, named vector query, payload filter AST, payload index capability, scroll cursor, filter accuracy diagnostics |
| OpenAI File Search / Vector Stores | Managed vector stores, file ingestion, retrieval result content inclusion, expiration metadata | Managed collection handle, file/chunk provenance, content inclusion policy, expiration/freshness metadata |
| LangChain retrievers | Provider-agnostic retriever interface, parent-document, multi-query, contextual compression | Retrieval strategy descriptor, parent context window, query expansion hook, rerank/compression delegation |

## Goals

- Provide stable pack id `pack.knowledge.retrieval.v1` and command namespace
  `retrieval.*`.
- Support collection registration, record upsert/delete, vector retrieval,
  sparse retrieval, hybrid retrieval, range retrieval, bulk retrieval,
  retrieve-by-id, rerank, parent-context expansion, evidence packaging,
  collection inspection, record inspection, refresh, and diagnostics.
- Model embedding/vector-space compatibility, metadata filters, tenant
  namespaces, score normalization, fusion strategy, top-k budgets, context
  windows, freshness, and provenance explicitly.
- Keep provider-specific query parameters in bounded adapter metadata, never in
  OS-layer routing branches.
- Require developer documentation under
  `docs/developer-packs/knowledge/retrieval.md`.

## Non-Goals

- Do not implement a concrete Pinecone, Weaviate, Milvus, Qdrant, OpenAI, or
  LangChain provider in this proposal.
- Do not implement document parsing, embeddings, citations, graph knowledge, or
  summarization; those are separate packs that retrieval can depend on through
  declared capabilities.
- Do not construct prompts or application-specific RAG chains.
- Do not expose raw vectors, raw provider payloads, raw credentials, raw corpus
  content, raw prompt text, or unbounded chunks in logs, traces, snapshots, SDK
  diagnostics, or examples.

## Ownership And Boundaries

- Pack id: `pack.knowledge.retrieval.v1`.
- Family: `knowledge`.
- Backing service owner: retrieval service provider.
- SDK surface: `sdk.packs.knowledge.retrieval`.
- Command namespace: `retrieval.*`.
- Microkernel owns identity, policy facade, resource primitives, service-call
  evidence, trace/audit primitives, and registry metadata only.
- Application framework owns manifest declarations, app-scoped permissions, and
  effective capability projection.
- Runtime host owns provider adapter registration, service decorators, vector
  store bridge composition, and sanitized diagnostics through approved
  composition roots.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `retrieval.register_collection` | Register collection/index metadata, vector spaces, namespace policy, and provider binding | Requires entitlement, schema/vector compatibility, ACL model, and secret references |
| `retrieval.upsert_records` | Upsert chunks/records and metadata handles | Requires write permission, embedding compatibility, idempotency, and redaction |
| `retrieval.delete_records` | Delete records by handle, namespace, or bounded filter | Requires write permission and audit reason |
| `retrieval.retrieve` | Run vector, sparse, lexical, or hybrid retrieval | Requires query validation, ACL filtering, top-k budget, and score normalization |
| `retrieval.bulk_retrieve` | Run multiple retrieval queries in one bounded command | Requires per-query budgets and aggregate quota |
| `retrieval.retrieve_by_id` | Fetch records/chunks by stable handles | Requires read permission and redacted content policy |
| `retrieval.range_retrieve` | Retrieve candidates within score/distance range | Requires provider capability and bounded threshold semantics |
| `retrieval.rerank_context` | Rerank candidate chunks through declared rerank provider/capability | Requires rerank permission and redaction |
| `retrieval.expand_context` | Add parent document, sibling chunks, or window context around candidates | Requires parent-window limits and source permissions |
| `retrieval.package_evidence` | Package retrieved chunks with provenance, offsets, and redacted content | Requires evidence permission and citation/corpus policy |
| `retrieval.inspect_collection` | Inspect collection schema, vector spaces, freshness, and health | Returns bounded metadata, never raw corpus content |
| `retrieval.inspect_record` | Inspect one record's metadata/provenance by handle | Requires metadata/read permission and redaction profile |
| `retrieval.refresh_collection` | Request provider refresh/reindex/compaction where supported | Requires quota, async handle, and approval for expensive operations |
| `retrieval.query_diagnostics` | Validate retrieval query and capability compatibility without provider side effects | Returns validation/unsupported diagnostics |

## DTO Model

Core DTOs:

- `RetrievalCollection`: collection handle, namespace policy, provider class,
  vector spaces, embedding model references, ACL model, freshness policy,
  retention policy, health, and capability hash.
- `RetrievalNamespace`: tenant/app/session/source scope, isolation policy,
  quota policy, and provider namespace/partition reference hash.
- `RetrievalRecord`: stable record handle, source document handle, chunk handle,
  vector-space memberships, metadata, content hash, redacted preview, ACL
  evidence hash, freshness timestamp, and provenance.
- `RetrievalChunk`: chunk handle, parent document handle, offsets, token count,
  modality, redaction profile, content handle, embedding references, and sibling
  window metadata.
- `RetrievalVectorSpace`: vector name, dimension, dense/sparse/multivector kind,
  metric, embedding model reference, index capability, and compatibility hash.
- `RetrievalQuery`: query text handle or embedding handle, query vectors,
  vector-space target, metadata filter AST, top-k, threshold/range, hybrid
  strategy, fusion strategy, rerank policy, window policy, and evidence policy.
- `RetrievalCandidate`: record/chunk handle, score, normalized score, distance,
  rank, vector-space source, retrieval strategy, metadata hits, ACL evidence
  hash, and provenance.
- `RetrievalEvidenceBundle`: ordered candidates, redacted content handles,
  source attribution, offsets, confidence, freshness, dedupe metadata, and replay
  pointer.
- `RetrievalProviderCapability`: vector kinds, hybrid support, filter support,
  namespace/partition support, bulk query support, range support, rerank support,
  max top-k, max filters, rate limits, consistency, and health.

## Permission, Policy, Resource, Entitlement, And Approval Model

Permission scopes:

- `retrieval.collection.manage`
- `retrieval.record.write`
- `retrieval.query`
- `retrieval.read`
- `retrieval.evidence`
- `retrieval.rerank`
- `retrieval.metadata.inspect`
- `retrieval.refresh`

Policy defaults:

- Every command is scoped to application id, tenant id, session id, task id, and
  trace id when available.
- Collection registration requires secret references, vector-space compatibility,
  ACL model, namespace isolation, and provider capability checks.
- Retrieval commands require ACL filtering, namespace isolation, metadata filter
  validation, top-k limits, threshold limits, query complexity limits, timeout
  budget, and redaction policy.
- Rerank and evidence packaging can delegate to AI/rerank/citation packs only
  through declared capabilities and sanitized candidate DTOs.
- Raw vectors, raw provider payloads, raw credentials, raw prompt text, and raw
  corpus content are forbidden in observability.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, family, version, lifecycle, command schemas,
collection capabilities, permission scopes, policy templates, vector-space
compatibility, filter/fusion/rerank support, namespace limits, top-k limits,
examples, unavailable diagnostics, health, compatibility, redaction profiles,
and documentation links.

The developer guide at `docs/developer-packs/knowledge/retrieval.md` must cover
manifest declarations, collection registration, namespaces, vector spaces,
embedding compatibility, record upsert/delete, metadata filters, vector/hybrid
retrieval, bulk/range retrieval, reranking, context expansion, evidence
packaging, ACL filtering, score normalization, provider replacement, unavailable
diagnostics, trace/audit interpretation, and conformance tests.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `retrieval_pack_declared`
- `retrieval_pack_admission_validated`
- `retrieval_collection_registered`
- `retrieval_records_upserted`
- `retrieval_records_deleted`
- `retrieval_query_requested`
- `retrieval_query_completed`
- `retrieval_context_reranked`
- `retrieval_context_expanded`
- `retrieval_evidence_packaged`
- `retrieval_collection_refreshed`
- `retrieval_pack_policy_decision`
- `retrieval_pack_service_call_requested`
- `retrieval_pack_service_call_succeeded`
- `retrieval_pack_service_call_failed`
- `retrieval_pack_unavailable`
- `retrieval_pack_snapshot_recorded`

Snapshots include descriptor version, collection capability hashes, vector-space
schema hashes, namespace summaries, provider health, index freshness, command
availability, policy template hash, resource counters, top-k/rerank limits, and
sanitized replay pointers. Snapshots must exclude raw vectors, raw documents,
raw chunks, raw provider payloads, credentials, raw query text when policy
requires hashing, private corpus content, and unbounded output.

## Design Patterns

- **Facade**: SDK clients expose discovery and command builders only.
- **Command**: every operation is a typed command/result DTO.
- **Strategy**: provider adapters, vector search, sparse search, hybrid fusion,
  score normalization, rerank delegation, and unavailable behavior are
  replaceable.
- **Decorator**: trace, policy, entitlement, resource, approval, metering,
  ACL filtering, namespace isolation, and redaction wrap service calls.
- **Specification**: admission validates collection declarations, vector-space
  compatibility, filters, top-k budgets, permissions, provider capability, and
  compatibility.
- **Observer**: collection health, record mutations, retrieval events, refresh
  events, trace, and audit events are subscribable.
- **Memento**: retrieval cursors, evidence bundles, collection snapshots,
  query replay pointers, and refresh handles preserve recovery state.
- **Abstract Factory**: provider adapters are created only by approved runtime
  host composition roots.

## Risks And Mitigations

- Risk: retrieval leaks private chunks into prompts or traces. Mitigation:
  enforce ACL filtering, evidence redaction, bounded chunk windows, and sanitized
  observability.
- Risk: provider-specific vector parameters leak into OS semantics. Mitigation:
  use stable DTOs and bounded adapter metadata.
- Risk: score semantics differ across providers. Mitigation: return raw score
  class plus normalized score, metric metadata, and fusion strategy.
- Risk: high top-k or bulk retrieval overloads providers. Mitigation: require
  top-k budgets, bulk budgets, timeouts, quotas, and provider limit diagnostics.
- Risk: rerank creates a hidden second path. Mitigation: rerank is a declared
  command/delegation with trace, policy, and capability checks.
