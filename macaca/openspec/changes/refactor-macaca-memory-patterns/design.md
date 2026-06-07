## Context

`macaca-memory` is a stage-1 infrastructure crate. It should provide stable memory primitives for long-running agents without depending on application-specific workflow logic.

Current managers duplicate three-layer orchestration: session memory, file memory, optional embedding, and optional vector search. This change introduces small additive primitives around that behavior rather than replacing it.

## Goals

- Preserve existing behavior 1:1.
- Keep existing public methods compiling.
- Add extension seams for facade calls, embedding cache, backend construction, snapshots, and query strategy.
- Keep the crate generic and application-agnostic.
- Make superseded manager-level direct APIs grepable through deprecation markers.

## Non-Goals

- Do not migrate upper-crate consumers.
- Do not remove existing public methods.
- Do not deprecate low-level `MemoryStore`, `EmbeddingProvider`, or `VectorStore` trait methods because they remain canonical backend contracts.
- Do not change file layout, vector payload shape, Milvus endpoints, DashScope request shape, session TTL semantics, or isolation rules.
- Do not add external dependencies.

## Decisions

- Facade methods call existing `store`, `retrieve`, `list`, `get`, and `delete` behavior internally through non-deprecated private helpers where needed.
- `CachedEmbeddingProvider<E>` wraps an existing provider and caches per input text.
- Backend factory starts with standard in-process/file/in-memory-vector construction only; Milvus factory wiring remains additive and explicit.
- Snapshot types are serializable mementos and do not become the persistence backend.
- Vector query strategy starts with current similarity search behavior and optional metadata filter support.
- Only inherent manager methods superseded by facade operations are marked deprecated; trait methods are not marked deprecated.
