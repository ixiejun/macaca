# Change: Add AI Embedding Pack

## Why

Developers need `pack.ai.embedding.v1` as a real industrial capability for text/image embedding, batch embedding, vector metadata, and model diagnostics. The pack must not be a catalog label only: if an application declares it and the provider is installed, the SDK must expose callable typed commands; if policy, entitlement, provider, or host support is absent, Macaca must return explicit unavailable or denied diagnostics.

This child proposal intentionally narrows the broad industrial catalog into one implementable service-backed pack. It records the research basis, capability boundary, command surface, permission model, observability model, and acceptance gates needed before this pack can be called production-ready.

## Research And Borrowed Platform Patterns

The proposal borrows stable ideas from mature application platforms without copying their product-specific APIs:

- Platform privacy models: raw user data must not enter logs or diagnostics when processed by intelligent services.
- Android runtime permission pattern: AI operations that touch private data inherit source permissions.
- Windows capability declaration pattern: model-backed operations must be visible in app capability metadata.
- Apple entitlement/privacy pattern: sensitive processing requires policy and developer-declared purpose.

The Macaca interpretation is: app manifests declare pack intent; admission validates capability and permission metadata; runtime permission and approval gates happen before sensitive side effects; concrete providers stay behind service descriptors; traces and audits prove the execution path.

## What Changes

- Add the provider-neutral `pack.ai.embedding.v1` contract under the `ai` family.
- Define a concrete command namespace, initial industrial command set, result/error DTO requirements, permission scopes, policy defaults, resource/entitlement behavior, and unavailable diagnostics.
- Bind implementation ownership to embedding service provider; the kernel, SDK, shells, and generic application framework remain provider-neutral.
- Add SDK discovery metadata and examples for embed text, embed image, batch embed, inspect vector schema, estimate cost.
- Add trace, audit, health, snapshot, replay, and boundary gates proving the pack uses the canonical service path.

## Impact

- Affected specs: `pack-ai-embedding`, `developer-pack-industrial-capability-catalog`, `sdk-system-facade`, `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral DTOs in protocol crates, descriptor/admission validators, SDK discovery/command helpers, embedding service provider, optional provider registration, trace/audit tests, and dependency-boundary gates.
- Non-goals: no application-specific workflow, no provider-name routing in OS layers, no concrete provider construction in SDK/shell/kernel, and no fake success for absent providers.

## Supplier/API Baseline To Match

- OpenAI embeddings style APIs: input batching, vector dimensions, model usage,
  and deterministic response ordering.
- Cohere Embed style APIs: input type hints, embedding types, truncation policy,
  and batch handling.
- Vertex AI text/multimodal embeddings: task type, output dimensions, image/text
  embedding variants, and quota accounting.
- Vector database ingestion contracts: dimension/schema compatibility, vector
  metadata, idempotent batch writes, and index-version compatibility.

The pack returns vectors and metadata only through typed DTOs. It does not own
storage, retrieval, or application-specific ranking behavior; those remain in
knowledge/retrieval or vector-store services.

## Industrial Acceptance Bar

This proposal is not complete until implementation tasks produce:

- a developer guide at `docs/developer-packs/ai/embedding.md`;
- typed input, batch, vector, vector schema, truncation, usage, and compatibility
  DTOs;
- deterministic tests for batch order, dimension mismatch, truncation policy,
  idempotent batch retry, unsupported modality, and redacted replay;
- audit replay proving raw embedded content is not logged while hashes, vector
  dimensions, schema ids, usage, and policy decisions remain inspectable.
