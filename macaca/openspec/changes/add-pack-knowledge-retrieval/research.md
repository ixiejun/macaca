# Knowledge Retrieval Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
existing platform inventory, and GitNexus memo evidence for
`pack.knowledge.retrieval.v1`. Retrieval must expose vector/hybrid candidate
selection, metadata filtering, context packaging, evidence bundles, freshness,
and cursors through serviceized commands. It must not become embedding, rerank,
citation, document storage, or provider-native vector database pass-through.

## Source Baseline

- Pinecone records, namespaces, metadata filtering, and query:
  <https://docs.pinecone.io/>
- Weaviate collections, named vectors, hybrid search, and modules:
  <https://docs.weaviate.io/weaviate>
- Milvus collections, partitions, vector fields, scalar filters, iterators, and
  hybrid search: <https://milvus.io/docs>
- Qdrant points, named vectors, payload filters, and scroll/search:
  <https://qdrant.tech/documentation/>
- OpenAI vector stores and file search:
  <https://platform.openai.com/docs/guides/tools-file-search>
- LangChain retriever and vector store abstractions:
  <https://docs.langchain.com/oss/python/integrations/vectorstores>

## Supplier API Notes

- Pinecone contributes namespace isolation, records, metadata filters, top-k,
  limits, and query diagnostics. Macaca should model collection, namespace,
  record, metadata-filter, and provider-limit DTOs.
- Weaviate contributes collections, schema, named vectors, hybrid search,
  rerank/generative modules, and filters. Macaca should keep embedding/rerank
  as separate declared pack dependencies and expose capability handles.
- Milvus contributes collections, partitions, vector fields, range/iterator
  retrieval, hybrid search, and scalar filters. Macaca should model vector-space
  schema and cursor/range retrieval without provider query syntax.
- Qdrant contributes points, payload filters, named vectors, scroll, and hybrid
  retrieval. Macaca should model records/chunks, payload filter AST, and cursor
  semantics.
- OpenAI vector stores/file search and LangChain retrievers show higher-level
  retrieval abstractions that combine files, chunks, search, and evidence.
  Macaca should expose explicit evidence packaging and context-window expansion
  rather than hidden provider retrieval.

## Macaca-Owned Abstractions

`pack.knowledge.retrieval.v1` should define `RetrievalCollection`,
`RetrievalNamespace`, `RetrievalRecord`, `RetrievalChunk`,
`RetrievalVectorSpace`, `RetrievalQuery`, `RetrievalMetadataFilter`,
`RetrievalFusionStrategy`, `RetrievalCandidate`,
`RetrievalEvidenceBundle`, `RetrievalCursor`, `RetrievalFreshness`, and
`RetrievalProviderCapability`.

The DTOs must capture collection ownership, namespace isolation, vector-space
schema, embedding compatibility, ACL trimming, metadata filters, hybrid fusion,
rerank hooks, context-window budgets, evidence links, freshness, and replay.
Raw vectors, raw documents, raw provider payloads, credentials, and unbounded
chunks are rejected from traces, audits, snapshots, and SDK diagnostics.

## Existing Macaca Platform Inventory

- `macaca-memory` already has embedding and hybrid query pipeline patterns that
  can inform retrieval, but memory retrieval is not this industrial pack.
- AI embedding and rerank pack proposals are explicit dependencies for
  embedding and rerank behavior; retrieval must only call them through declared
  capabilities.
- Generic descriptors, SDK facade clients, service-call tracing, unavailable
  clients, persistence snapshots, and policy command objects provide reusable
  infrastructure for future retrieval service providers.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
