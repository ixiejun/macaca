# AI Embedding Pack Research

## Purpose

This note records borrowed platform patterns, Macaca mapping, existing platform
inventory, and GitNexus memo evidence for `pack.ai.embedding.v1`. The pack must
provide text, image, and batch embedding with vector schema diagnostics through
provider-neutral commands. It must not become vector storage, retrieval, search,
or application-specific ranking logic.

## Source Baseline

- OpenAI embeddings API documentation:
  <https://platform.openai.com/docs/guides/embeddings>
- Google Vertex AI text and multimodal embedding documentation:
  <https://cloud.google.com/vertex-ai/generative-ai/docs/embeddings>
- AWS Bedrock Titan embeddings documentation:
  <https://docs.aws.amazon.com/bedrock/latest/userguide/titan-embedding-models.html>
- Azure OpenAI embeddings documentation:
  <https://learn.microsoft.com/en-us/azure/ai-services/openai/how-to/embeddings>
- Vector-index compatibility concepts from mature vector databases and search
  services are treated as downstream consumer constraints, not embedding pack
  ownership.

## Borrowed Platform Patterns

- Embedding APIs converge on input arrays, item ordering, model/vector
  dimensions, usage counters, truncation limits, modality support, batch
  behavior, and partial failures. Macaca should preserve input item ids and
  ordered output mapping across retry and partial failure.
- Providers expose vector dimensions and sometimes model-specific metrics or
  normalization assumptions. Macaca should surface a `VectorSchemaDescriptor`
  with dimension, numeric type, metric family, normalization, and compatibility
  hash.
- Text, image, and multimodal inputs require different size, format, and
  redaction policies. Macaca should model content references and bounded hashes
  rather than raw text or image bytes in audit.
- Cost estimation is a preflight concern. Macaca should expose
  `embedding.estimate_cost` and batch resource budgeting before provider calls.
- Embedding outputs are not storage. Vector indexing, retrieval, and graph
  persistence remain knowledge/storage services.

## Macaca Mapping

- Descriptor: `pack.ai.embedding.v1`, command namespace `embedding.*`, scopes
  `ai.embedding.invoke` and `ai.embedding.batch`.
- Commands: `embedding.embed_text`, `embedding.embed_image`,
  `embedding.batch_embed`, `embedding.inspect_vector_schema`, and
  `embedding.estimate_cost`.
- DTOs: `EmbeddingInput`, `EmbeddingBatchRequest`, `EmbeddingVector`,
  `EmbeddingBatchResult`, `VectorSchemaDescriptor`, and `EmbeddingUsage`.
- Policy: validate declared source permission inheritance, modality, item count,
  size/truncation behavior, vector schema compatibility, resource budget,
  entitlement, and provider capability before dispatch.
- Trace/audit: record item ids, modality, content hashes, dimensions, schema
  hash, truncation status, usage counters, and bounded errors only.

## Existing Macaca Platform Inventory

- `macaca-memory` already has an `EmbeddingProvider` trait, `MockEmbedding`, and
  vector-degraded diagnostics in the memory query pipeline. These are useful
  provider and degradation patterns but do not satisfy `pack.ai.embedding.v1`
  command DTOs, descriptor metadata, SDK discovery, or admission rules.
- Memory snapshot/replay support provides Memento patterns for embedding batch
  diagnostics, but vector storage remains outside the embedding pack.
- `SystemFacade`, service-call tracing, service descriptors, and unavailable
  clients provide reusable Facade, Command, Observer, and Null Object patterns.
- Runtime-host composition roots already import `MockEmbedding`, proving provider
  creation must stay in approved composition roots, not SDK or shell code.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
