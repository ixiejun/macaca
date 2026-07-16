# Knowledge Retrieval Pack

`pack.knowledge.retrieval.v1` describes vector, hybrid, metadata-filtered, and
evidence-oriented retrieval over application-declared collections.

## Manifest Declaration

```toml
[service_contract]
optional_packs = ["pack.knowledge.retrieval.v1"]
```

Required declarations block readiness when no compatible provider is installed.
Optional declarations surface degraded effective capabilities.

## Permissions

Scopes are `retrieval.collection.manage`, `retrieval.record.write`,
`retrieval.query`, `retrieval.read`, `retrieval.evidence`,
`retrieval.rerank`, `retrieval.metadata.inspect`, and `retrieval.refresh`.

## Capability Model

The pack uses provider-neutral collection, namespace, record, chunk,
vector-space, metadata filter, fusion strategy, candidate, cursor, freshness,
and evidence-bundle DTOs. Raw vectors, embeddings, documents, prompts, chunks
beyond redaction policy, and provider payloads are never part of public
observability.

## Platform Comparison

Pinecone namespaces, records, queries, filters, and limits map to collection,
namespace, record, query, metadata filter, and provider capability DTOs.
Weaviate named vectors, hybrid search, and rerank modules map to vector-space,
fusion, and rerank metadata. Milvus collections, partitions, vector fields,
range search, and iterators map to namespace, vector-space, range retrieval,
and cursor DTOs. Qdrant points, payload filters, and scroll map to record,
metadata filter, and cursor DTOs. OpenAI vector stores and file search map to
collection, source handle, and evidence bundle metadata. LangChain retrievers
map to provider class capability descriptors, not OS routing logic.

## Commands

Commands include collection registration, record upsert/delete, retrieve,
bulk/range retrieve, retrieve by id, rerank, context expansion, evidence
packaging, collection and record inspection, refresh, and diagnostics. All
commands travel through SDK/facade, service runtime decorators, and provider
dispatch.

## App-Facing Examples

- Register a collection with namespace, vector-space, ACL, and freshness
  policy metadata.
- Upsert records by handle; store chunks and vectors by reference only.
- Retrieve with metadata filters and a bounded `top_k`.
- Use hybrid retrieval only when provider capability reports dense and sparse
  support.
- Run bulk or range retrieval with cursor pagination.
- Retrieve by id when an application already owns a record reference.
- Rerank context and expand parent windows through declared permissions.
- Package evidence as references for citations, summarization, or audit
  surfaces.
- Inspect collection and record metadata before refresh or diagnostics.
- Treat unavailable provider results as explicit degraded capability, not as
  permission to bypass the service runtime.

## Trace And Audit

Trace metadata should include collection id, namespace, vector-space id,
command name, provider class, capability hash, page/cursor metadata, and result
status. Raw vectors, embeddings, documents, chunks beyond redaction policy,
private corpus content, and provider payloads must stay out of traces and
snapshots.

## Provider Authors

Descriptors must report vector-space compatibility, dense/sparse/multivector
support, named vector spaces, metadata filters, namespaces, hybrid fusion,
rerank, context expansion, top-k limits, ACL behavior, score normalization,
health, quota, and snapshots. Missing embedding, rerank, evidence, or refresh
support must return structured unavailable or unsupported diagnostics.

Conformance tests should cover vector-space compatibility, filter validation,
ACL enforcement, score normalization, evidence packaging, redaction, cursor
pagination, unavailable behavior, and provider capability reporting.
