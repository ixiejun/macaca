# Change: Add Industrial Knowledge Retrieval Pack

## Why

Applications need retrieval as a reusable RAG-grade capability for finding,
ranking, and packaging relevant chunks or source records from declared knowledge
corpora. Retrieval is narrower and deeper than keyword search: it must model
vector collections, namespaces, embeddings, dense/sparse/multivector queries,
hybrid fusion, metadata filters, top-k limits, score normalization, parent
documents, chunk windows, reranking hooks, evidence bundles, freshness, and
provider replacement.

If each application builds its own retrieval path, Macaca will accumulate
provider-specific vector-store calls, inconsistent ACL enforcement, prompt-time
data leakage, duplicate reranking paths, and non-replayable evidence. This pack
defines one canonical service path for retrieval-augmented context fetch.

## Supplier And Platform API Research

This proposal maps established retrieval/vector-store APIs into Macaca
provider-neutral abstractions:

- Pinecone exposes indexes, namespaces, records with dense/sparse vectors,
  metadata filters, upsert/update/fetch/query/delete operations, top-k query,
  multitenancy guidance, and request/rate limits. Macaca maps these to
  retrieval collections, namespaces, vector records, metadata filters,
  record handles, top-k budgets, tenant isolation, and provider quota
  diagnostics.
- Weaviate exposes collections/classes, objects, named vectors, vector search,
  BM25, hybrid search, filters, generative modules, reranking modules, and
  schema/capability metadata. Macaca maps these to collection descriptors,
  named vector spaces, lexical/vector/hybrid retrieval strategies, module
  capability metadata, rerank delegation, and schema compatibility checks.
- Milvus exposes collections, partitions, vector fields, scalar filters,
  single-vector search, bulk-vector search, hybrid/multivector search, range
  search, search iterators, and index/collection limits. Macaca maps these to
  partition scopes, vector-field descriptors, bulk query, hybrid fusion,
  range-threshold retrieval, cursor/iterator pagination, and limit diagnostics.
- Qdrant exposes collections, points, named dense/sparse/multivectors, payload
  filters, payload indexes, scroll/search APIs, hybrid retrieval, and filtered
  vector search optimization. Macaca maps these to point handles, payload filter
  AST, named vector queries, payload-index capability, scroll cursors, and
  filter accuracy diagnostics.
- OpenAI File Search / Vector Stores expose vector-store-backed retrieval for
  uploaded files and result content inclusion. Macaca maps these to managed
  vector-store handles, file/chunk provenance, retrieval result content policy,
  expiration metadata, and provider-managed ingestion state.
- LangChain retriever interfaces model retrieval as a provider-agnostic
  component that returns documents for a query and can wrap vector stores,
  parent-document retrievers, multi-query retrievers, and contextual compressors.
  Macaca maps this to a retrieval strategy descriptor, parent context windows,
  query expansion hooks, and rerank/compression delegation.

The Macaca contract is not a vector database API pass-through. Provider-specific
parameters remain bounded adapter metadata; OS-layer semantics use stable
retrieval DTOs and declared provider capability metadata.

## What Changes

- Add provider-neutral `pack.knowledge.retrieval.v1` under the `knowledge`
  family.
- Define DTOs for retrieval collections, namespaces, records, chunks, embeddings,
  vector spaces, metadata filters, retrieval queries, hybrid fusion, candidates,
  rerank inputs/results, parent context windows, evidence bundles, cursors,
  freshness, and provider capabilities.
- Define commands for registering collections, upserting/removing records,
  retrieve, retrieve by id, bulk retrieve, hybrid retrieve, range retrieve,
  rerank context, package evidence, inspect collection, inspect record, refresh
  collection, and query diagnostics.
- Define permission scopes for collection management, record write, query,
  record read, evidence packaging, rerank delegation, metadata inspection, and
  refresh.
- Require ACL filtering, tenant namespace isolation, score normalization,
  bounded top-k/window sizes, redacted evidence packaging, replayable retrieval
  provenance, unavailable diagnostics, and a detailed developer guide.

## Impact

- Affected specs: `pack-knowledge-retrieval`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected future code: provider-neutral proto DTOs, retrieval descriptors,
  admission validators, SDK discovery metadata, focused SDK clients, retrieval
  service providers, unavailable/mock providers, trace/audit schemas, retrieval
  conformance tests, replay tests, and dependency-boundary gates.
- Non-goals: no application-specific RAG workflow, no prompt construction, no
  provider-name routing in OS layers, no raw vector/provider payload exposure,
  no concrete provider construction in kernel/SDK/shells, and no fake success
  when retrieval providers or collections are unavailable.

## References

- Pinecone indexing and query docs:
  https://docs.pinecone.io/guides/index-data/indexing-overview
- Pinecone metadata filtering:
  https://docs.pinecone.io/guides/search/filter-by-metadata
- Weaviate hybrid search:
  https://docs.weaviate.io/weaviate/search/hybrid
- Milvus vector search:
  https://milvus.io/docs/single-vector-search.md
- Milvus multi-vector hybrid search:
  https://milvus.io/docs/multi-vector-search.md
- Qdrant search:
  https://qdrant.tech/documentation/search/search/
- OpenAI File Search / Vector Stores:
  https://developers.openai.com/api/docs/guides/tools-file-search
- LangChain retrievers:
  https://docs.langchain.com/oss/python/integrations/retrievers
