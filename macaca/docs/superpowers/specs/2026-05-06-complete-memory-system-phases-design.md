# Complete Memory System Phases Design

## Context

`docs/memory-system-openclaw-hermes-research.md` defines six implementation phases for Macaca's memory system:

- Phase 1: Memory Fabric core model.
- Phase 2: Pluggable providers and free composition.
- Phase 3: Embedding / index / vector backend decoupling.
- Phase 4: Active Memory.
- Phase 5: Knowledge / Wiki layer.
- Phase 6: Governance and long-running autonomy.

The audit found that Phase 1 and Phase 4 are mostly implemented, while Phase 2, Phase 3, Phase 5, and Phase 6 are only partially complete. Some OpenSpec task lists mark work as complete even though the code is still stub-level or not wired into production runtime paths.

This design defines how to finish all phases without breaking the existing working behavior.

## Current Implementation Summary

Implemented and usable:

- `MemoryScope`, `MemoryFacade`, `MemoryRouter`, `MemoryFabricFacade`, builtin private/shared adapters.
- `ActiveRecallCapability`, composer-side active recall provider, budget handling, timeout/fail-open behavior, context report diagnostics.
- Basic provider config, `MemoryProviderRegistry`, remote provider adapter, resilience wrapper.
- `VectorMemoryBackend`, Milvus topology resolver, vector topology tests.
- `KnowledgeCompileCapability`, claim/evidence DTOs, workspace knowledge digest adapter.
- `GovernedMemoryFacade`, candidate capture, promotion policy, tombstone index, snapshot/replay.

Not fully implemented:

- MCP memory provider is not wired to a live MCP client; methods return a fixed "not wired" error.
- Provider runtime is not the production entry for `macaca-web`; runtime still uses `TestMemoryManager` directly.
- There is no `EmbeddingProviderRegistry` with provider selection, timeout/retry/metrics decorators.
- Vector backend alternatives and conformance harness are incomplete.
- Keyword / FTS / vector / hybrid / filtered / rerank search are not represented as a complete strategy stack.
- Knowledge compiler does not perform real contradiction detection or persistent wiki/project decision log management.
- Governance is mostly an in-memory decorator, not a durable long-running autonomy runtime.
- Automatic candidate capture, memory compaction/dreaming, provider migration, and PII propagation are incomplete.

## Brainstorm Options

### Option A: Mark current implementation as complete and only patch obvious stubs

This would wire MCP enough to stop the fixed error and add a few tests.

Benefits:

- Smallest short-term code change.
- Low immediate regression risk.

Risks:

- Does not satisfy the report's target architecture.
- Leaves provider runtime unused by production paths.
- Leaves Phase 3, Phase 5, and Phase 6 as mostly aspirational.
- Makes future audits misleading because OpenSpec tasks would still overstate completion.

Decision: reject. This does not complete all phases.

### Option B: Complete phases through additive runtime slices, then migrate production consumers

This approach keeps existing `TestMemoryManager` and legacy managers available, but introduces canonical additive contracts first, then moves `macaca-web` and framework/context integrations onto them.

Benefits:

- Matches AGENTS.md incremental and reversible-change rule.
- Keeps working active recall and trace behavior intact.
- Allows each incomplete phase to become independently testable.
- Avoids application-specific logic.
- Preserves existing public API while making production paths use the new primitives.

Risks:

- More total work than a patch.
- Requires careful compatibility adapters to avoid duplicate recall/digest injection.
- Requires contract tests for provider and vector backend behavior before production migration.

Decision: recommended.

### Option C: Replace the memory subsystem with a new Memory Runtime in one large change

This would rewrite `macaca-memory`, `macaca-context`, and `macaca-web` around a new runtime and remove legacy manager paths.

Benefits:

- Clean end-state fastest on paper.
- Less adapter code after completion.

Risks:

- Violates the project's small-slice refactor constraint.
- High chance of breaking existing working active recall, workspace tools, digest, tombstones, or session behavior.
- Large blast radius across multiple crates.

Decision: reject. Too risky.

## Recommended Design

Use Option B: finish the report phases with seven additive implementation slices.

The slices are ordered to make every step runnable:

1. Correct OpenSpec/task truth and add missing conformance tests.
2. Finish provider runtime by wiring MCP and provider diagnostics as real runtime capabilities.
3. Introduce `MemoryRuntimeFacade` as the production canonical boundary while preserving legacy managers.
4. Finish embedding/index/backend decoupling.
5. Finish knowledge/wiki runtime semantics.
6. Finish governance autonomy runtime.
7. Migrate `macaca-web` production memory paths to the runtime facade.

## Design Pattern Mapping

- Facade: `MemoryRuntimeFacade` hides provider registry, routing, active recall, knowledge digest, and governance composition.
- Adapter: Legacy `TestMemoryManager`, remote provider, MCP provider, Milvus vector store, and workspace memory tools adapt into the new facade.
- Strategy: Provider selection, query strategy, active recall policy, knowledge conflict detection, promotion policy, and migration policy.
- Decorator: Timeout/retry/metrics/cache/redaction/circuit-breaker wrappers around embedding, provider, and vector calls.
- Abstract Factory: Provider/backend/embedding factories resolve runtime instances from profile config.
- Chain of Responsibility: Memory write governance pipeline and query pipeline.
- Memento: Snapshot/restore and provider migration checkpoints.
- Observer: Provider diagnostics and runtime memory events routed into context report / trace.
- Proxy: Remote memory provider, MCP provider, Milvus/LanceDB/Qdrant/remote vector backends.

## Target Architecture

### Runtime Boundary

All upper crates should depend on a single memory runtime boundary:

```rust
#[async_trait::async_trait]
pub trait MemoryRuntimeFacade: Send + Sync {
    async fn remember(&self, request: MemoryWriteRequest) -> MacacaResult<MemoryId>;
    async fn search(&self, request: MemorySearchRequest) -> MacacaResult<Vec<MemoryEntry>>;
    async fn active_recall(&self, request: ActiveRecallRequest) -> MacacaResult<ActiveRecallResult>;
    async fn compile_knowledge(&self, request: KnowledgeCompileRequest) -> MacacaResult<KnowledgeCompileResult>;
    async fn status(&self) -> MemoryRuntimeStatus;
}
```

This trait should be implemented by a composed runtime that internally uses existing facades, providers, embedding registry, vector backend registry, active recall, knowledge compiler, and governance decorators.

### Provider Runtime

`MemoryProviderRuntime` must become an actual factory-backed runtime, not just a status wrapper. It should resolve slot bindings for:

- agent private memory provider
- session shared memory provider
- embedding provider
- vector backend
- active recall provider
- knowledge compiler provider

Provider failure remains fail-open where the memory report requires it, but must record diagnostics.

### Production Migration

`macaca-web` should stop treating `TestMemoryManager` as the canonical memory subsystem. It can keep a builtin adapter, but `AppState` should expose a runtime facade that active recall, explicit memory tools, and knowledge digest all consume.

### Non-Goals

- Do not remove existing legacy manager APIs in the completion change.
- Do not introduce app-specific or agent-name-specific behavior.
- Do not require external Milvus, LanceDB, Qdrant, MCP, or remote providers for default tests.
- Do not change user-facing trace event semantics except adding better diagnostics where missing.
- Do not move the entire memory system into multiple new crates.

## Acceptance Criteria

The implementation is complete only when all of these are true:

- OpenSpec tasks and code reality match; no task claims a stub is complete.
- MCP memory provider can execute through the Agent OS MCP runtime or is explicitly marked unsupported in spec/tasks, not falsely completed.
- `MemoryProviderRuntime` can build and route real provider instances by profile and slot.
- Production active recall, knowledge digest, and memory tools consume `MemoryRuntimeFacade` or an adapter over it.
- `EmbeddingProviderRegistry` exists with cache, timeout, retry, and metrics decorators.
- Vector backend contract has reusable conformance tests and at least builtin + Milvus implementations.
- Hybrid query strategy supports keyword-only fallback when embedding fails or is absent.
- Knowledge compiler detects at least deterministic contradiction cases and emits conflict groups.
- Project decision log and wiki digest artifacts are generated from structured knowledge, not only in-memory skeletons.
- Governance has durable audit/candidate/tombstone snapshot support and a configurable promotion policy path.
- Provider migration has a checkpointed copy/verify/rollback plan.
- All related OpenSpec changes validate strictly.
- `cargo test -p macaca-memory --lib` and `cargo test -p macaca-context --lib` pass.
- Relevant `macaca-web` tests or checks pass.
