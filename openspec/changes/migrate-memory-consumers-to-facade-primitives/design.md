## Context

The completed `refactor-macaca-memory-patterns` change introduced additive memory primitives:

- `RememberText`
- `RecallQuery`
- `RecallResult`
- `ForgetMemory`
- `MemoryBackendFactory`
- snapshot and query strategy helpers

That change intentionally did not migrate upper crates. The current upper-crate memory boundary is `macaca-agent::MemoryService`, while `macaca-kernel::MemoryServiceAdapter` bridges concrete memory stores into that service trait.

## Goals

- Make facade-first memory access the canonical upper-crate path.
- Preserve existing behavior for no-op memory and kernel memory adapter tests.
- Keep deprecated old methods callable for compatibility and future grep-based migration.
- Avoid application-specific logic and workflow-specific assumptions.

## Non-Goals

- Do not remove `store` / `retrieve`.
- Do not deprecate low-level `MemoryStore`, `EmbeddingProvider`, or `VectorStore`; they remain backend contracts.
- Do not merge `macaca-framework::WorkingMemory` / `LongTermMemory` into `macaca-memory`.
- Do not change task, trace, session, planner, worker, driver, skill, MCP, or web behavior.
- Do not introduce new dependencies beyond the internal `macaca-agent -> macaca-memory` crate dependency.

## Decisions

- `macaca-agent::MemoryService` becomes facade-first by adding `remember_text(RememberText)` and `recall(RecallQuery)`.
- Deprecated compatibility methods remain on `MemoryService` with default wrappers so old callers can still compile.
- `NoopMemoryService` implements facade methods directly and still produces no persistent side effect.
- `macaca-kernel::MemoryServiceAdapter` keeps its type name for compatibility but adapts `MemoryManager` and `IsolatedMemoryManager` through facade methods.
- Kernel tests use `MemoryBackendFactory` so they exercise the new backend construction primitive instead of direct raw store construction.

## Design Patterns

- **Facade:** `MemoryService` exposes user-level remember/recall operations instead of raw store operations.
- **Adapter:** `MemoryServiceAdapter` adapts `macaca-memory` managers to `macaca-agent` services.
- **Null Object:** `NoopMemoryService` preserves no-service behavior without optional branching at call sites.
- **Builder:** `AgentServices::builder().memory(...)` remains the service injection mechanism.

## Risks

- Adding `macaca-memory` to `macaca-agent` expands the dependency surface. This is acceptable because `macaca-memory` is a lower-level crate that only depends on `macaca-proto`, so it does not introduce a dependency cycle.
- Compatibility wrappers from raw `MemoryEntry` to `RememberText` cannot preserve every field that facade text memory does not represent. Existing migrated production code must use facade methods; old methods remain only for compatibility and discovery.
- Grep for `.store` / `.retrieve` can produce false positives from unrelated traits. Verification should focus on `memory_service().store/retrieve`, `MemoryService` impls, and `MemoryServiceAdapter`.
