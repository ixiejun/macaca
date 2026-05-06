# Complete Memory System Phases Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete all six phases from `docs/memory-system-openclaw-hermes-research.md` so Macaca has a production-grade, pluggable, observable, scope-safe memory fabric.

**Architecture:** Keep the current working memory behavior intact and finish missing pieces through additive contracts first. Build a canonical `MemoryRuntimeFacade`, wire provider/embedding/vector/knowledge/governance runtimes behind it, then migrate `macaca-web` production memory paths from direct `TestMemoryManager` access to facade-backed adapters.

**Tech Stack:** Rust, `async-trait`, `tokio`, `serde`, `serde_json`, `macaca-memory`, `macaca-context`, `macaca-web`, `macaca-runtime-host`, OpenSpec, GitNexus.

---

## Scope

This plan completes the unfinished work from the audit of `docs/memory-system-openclaw-hermes-research.md`.

Already mostly complete and protected:

- Phase 1: `MemoryScope`, `MemoryFacade`, `MemoryRouter`, builtin private/shared adapters.
- Phase 4: active recall capability, composer integration, budget, timeout/fail-open, context report diagnostics.

Incomplete and targeted:

- Phase 2: provider runtime, MCP live transport, production runtime wiring.
- Phase 3: embedding registry/decorators, backend conformance, hybrid query stack.
- Phase 5: contradiction detection, durable wiki/project decision artifacts, exact citation.
- Phase 6: durable governance runtime, automatic candidate capture, compaction/dreaming, provider migration.

## File Map

OpenSpec:

- Create: `openspec/changes/complete-memory-fabric-runtime/proposal.md`
- Create: `openspec/changes/complete-memory-fabric-runtime/design.md`
- Create: `openspec/changes/complete-memory-fabric-runtime/tasks.md`
- Create: `openspec/changes/complete-memory-fabric-runtime/specs/macaca-memory-fabric-runtime/spec.md`
- Modify: existing memory OpenSpec task files only to correct false-complete checkboxes when code remains stubbed.

`macaca-memory`:

- Create: `macaca/crates/macaca-memory/src/runtime/mod.rs`
- Create: `macaca/crates/macaca-memory/src/runtime/facade.rs`
- Create: `macaca/crates/macaca-memory/src/runtime/status.rs`
- Create: `macaca/crates/macaca-memory/src/runtime/builder.rs`
- Create: `macaca/crates/macaca-memory/src/embedding_registry.rs`
- Create: `macaca/crates/macaca-memory/src/embedding_decorators.rs`
- Create: `macaca/crates/macaca-memory/src/vector_conformance.rs`
- Create: `macaca/crates/macaca-memory/src/query_pipeline.rs`
- Create: `macaca/crates/macaca-memory/src/governance/runtime.rs`
- Create: `macaca/crates/macaca-memory/src/governance/migration.rs`
- Modify: `macaca/crates/macaca-memory/src/lib.rs`
- Modify: `macaca/crates/macaca-memory/src/providers/runtime.rs`
- Modify: `macaca/crates/macaca-memory/src/providers/factory.rs`
- Modify: `macaca/crates/macaca-memory/src/providers/mcp.rs`
- Modify: `macaca/crates/macaca-memory/src/governance/compiler.rs`
- Modify: `macaca/crates/macaca-memory/src/governance/artifacts.rs`
- Modify: `macaca/crates/macaca-memory/src/vector_backend.rs`
- Modify: `macaca/crates/macaca-memory/src/query.rs`

`macaca-context`:

- Modify: `macaca/crates/macaca-context/src/memory_active_recall_provider.rs`
- Modify: `macaca/crates/macaca-context/src/knowledge_digest/context_provider.rs`
- Modify: `macaca/crates/macaca-context/src/knowledge_digest/selection.rs`
- Add tests under existing module test files.

`macaca-web`:

- Create: `macaca/crates/macaca-web/src/memory_runtime.rs`
- Modify: `macaca/crates/macaca-web/src/state.rs`
- Modify: `macaca/crates/macaca-web/src/lib.rs`
- Modify: `macaca/crates/macaca-web/src/context_reporting_model.rs`
- Modify: `macaca/crates/macaca-web/src/context_memory_tools.rs`
- Modify: `macaca/crates/macaca-web/src/workspace_memory_recall_source.rs`
- Modify: `macaca/crates/macaca-web/src/workspace_knowledge_digest_capability.rs`

Integration:

- Add or extend tests in `macaca/crates/macaca-integration-tests` only after lower-crate unit tests pass.

## Task 1: OpenSpec Truth Alignment

**Files:**

- Create: `openspec/changes/complete-memory-fabric-runtime/proposal.md`
- Create: `openspec/changes/complete-memory-fabric-runtime/design.md`
- Create: `openspec/changes/complete-memory-fabric-runtime/tasks.md`
- Create: `openspec/changes/complete-memory-fabric-runtime/specs/macaca-memory-fabric-runtime/spec.md`
- Modify: `openspec/changes/add-memory-provider-runtime/tasks.md`
- Modify: `openspec/changes/add-memory-vector-backend-topology/tasks.md`
- Modify: `openspec/changes/add-knowledge-digest-context-provider/tasks.md`

- [ ] **Step 1: Verify existing change state**

Run:

```bash
cd /Users/quantum/Code/dev/agent
openspec list
test ! -e openspec/changes/complete-memory-fabric-runtime
```

Expected:

```text
The test command exits with status 0.
```

- [ ] **Step 2: Write proposal**

Create `openspec/changes/complete-memory-fabric-runtime/proposal.md`:

```markdown
# Change: Complete Memory Fabric Runtime

## Why

The memory research report defines six phases for a production memory fabric. Current code implements the core scope/facade model and active recall, but provider runtime, embedding/backend decoupling, knowledge/wiki governance, and long-running autonomy are incomplete or only partially wired.

## What Changes

- Introduce a production `MemoryRuntimeFacade` that composes provider routing, active recall, knowledge compilation, and governance.
- Finish memory provider runtime wiring, including real MCP transport integration or an explicit unsupported capability state.
- Add embedding provider registry and decorators for cache, timeout, retry, and metrics.
- Add vector backend conformance harness and complete builtin/Milvus topology coverage.
- Add query pipeline strategies for keyword, vector, hybrid, filtered, and rerank-compatible search.
- Add deterministic contradiction detection, project decision log, wiki digest artifact, and citation references.
- Add durable governance runtime for candidate capture, promotion policy, tombstones, snapshots, compaction, and provider migration.
- Migrate production `macaca-web` memory consumers to the runtime facade through adapters.

## Impact

- Affected specs: `macaca-memory-fabric-runtime`, `macaca-memory-provider-runtime`, `macaca-memory-vector-backend`, `macaca-memory-governance`, `active-vector-memory-context`, `knowledge-digest-context`.
- Affected crates: `macaca-memory`, `macaca-context`, `macaca-web`, optional integration tests.
- Compatibility: existing manager/store APIs remain available.
```

- [ ] **Step 3: Write design**

Create `openspec/changes/complete-memory-fabric-runtime/design.md` with these sections:

```markdown
## Runtime Boundary

`MemoryRuntimeFacade` is the canonical upper-crate boundary. It wraps the existing `MemoryFacade`, provider runtime, active recall, knowledge compiler, and governance runtime. Existing managers remain available as builtin adapters.

## Provider Runtime

`MemoryProviderRuntime` resolves provider slots from profile config and can build operational providers. MCP providers must either call a live MCP client or report an explicit unsupported capability state; they must not advertise full store/search capability while returning fixed "not wired" errors.

## Embedding and Query Stack

Embedding provider selection is registry-driven. Decorators wrap providers for cache, timeout, retry, and metrics. Query execution is strategy-based and supports keyword fallback when embedding or vector search is unavailable.

## Knowledge and Governance

Knowledge compilation produces claims, evidence, conflict groups, project decisions, and wiki digest artifacts. Governance stores candidate/audit/tombstone state durably through snapshot-capable stores and exposes migration checkpoints.

## Production Migration

`macaca-web` initializes a runtime facade and adapts active recall, explicit memory tools, and knowledge digest through it. Legacy `TestMemoryManager` remains only as the builtin provider implementation behind the facade.
```

- [ ] **Step 4: Write tasks**

Create `openspec/changes/complete-memory-fabric-runtime/tasks.md` with top-level checkboxes matching this plan's tasks 1 through 9.

- [ ] **Step 5: Write delta spec**

Create `openspec/changes/complete-memory-fabric-runtime/specs/macaca-memory-fabric-runtime/spec.md`:

```markdown
## ADDED Requirements

### Requirement: Runtime Facade

Macaca SHALL expose a production memory runtime facade for upper crates.

#### Scenario: Production callers use runtime facade

- **GIVEN** a configured Macaca web state
- **WHEN** active recall, explicit memory tools, or knowledge digest need memory access
- **THEN** they SHALL call the memory runtime facade or an adapter backed by it
- **AND** they SHALL NOT directly depend on a concrete legacy manager as their canonical boundary.

### Requirement: Provider Runtime

Memory providers SHALL be resolved by profile and slot and SHALL report truthful capability status.

#### Scenario: MCP provider is unavailable

- **GIVEN** an MCP memory provider without live client wiring
- **WHEN** status is requested
- **THEN** it SHALL report unavailable or unsupported capability state
- **AND** it SHALL NOT report healthy store/search capability.

### Requirement: Query Degradation

Memory search SHALL degrade without blocking agent execution.

#### Scenario: Embedding fails

- **GIVEN** a hybrid query strategy
- **WHEN** embedding generation fails
- **THEN** keyword search SHALL still run
- **AND** diagnostics SHALL record the vector degradation.

### Requirement: Knowledge Governance

Compiled knowledge SHALL preserve evidence and conflicts.

#### Scenario: Contradictory claims are compiled

- **GIVEN** two memory candidates with contradictory statements about the same subject
- **WHEN** knowledge compilation runs
- **THEN** the result SHALL include a conflict group
- **AND** both evidence ids SHALL remain visible to context reports.

### Requirement: Provider Migration

Memory provider migration SHALL be checkpointed and auditable.

#### Scenario: Migration validation fails

- **GIVEN** a provider migration from source to target
- **WHEN** verification fails
- **THEN** the runtime SHALL keep the source provider authoritative
- **AND** write an audit event describing the failed checkpoint.
```

- [ ] **Step 6: Correct existing false-complete task entries**

In existing task files, change only entries whose code is provably incomplete:

```text
openspec/changes/add-memory-provider-runtime/tasks.md
  5.2 and 5.3 should not remain checked until MCP calls a live client or spec says unsupported.

openspec/changes/add-memory-vector-backend-topology/tasks.md
  4.4 remains unchecked until conformance harness exists.

openspec/changes/add-knowledge-digest-context-provider/tasks.md
  6.3, 6.5, 7.3, 7.4 remain unchecked until implemented and verified.
```

- [ ] **Step 7: Validate OpenSpec**

Run:

```bash
cd /Users/quantum/Code/dev/agent
openspec validate complete-memory-fabric-runtime --strict
openspec validate add-memory-provider-runtime --strict
openspec validate add-memory-vector-backend-topology --strict
openspec validate add-knowledge-digest-context-provider --strict
```

Expected:

```text
All listed changes are valid.
```

## Task 2: Runtime Facade Boundary

**Files:**

- Create: `macaca/crates/macaca-memory/src/runtime/mod.rs`
- Create: `macaca/crates/macaca-memory/src/runtime/facade.rs`
- Create: `macaca/crates/macaca-memory/src/runtime/status.rs`
- Create: `macaca/crates/macaca-memory/src/runtime/builder.rs`
- Modify: `macaca/crates/macaca-memory/src/lib.rs`

- [ ] **Step 1: Run GitNexus impact**

Run impact before editing symbols:

```text
gitnexus_impact target=MemoryFacade direction=upstream
gitnexus_impact target=MemoryProviderRuntime direction=upstream
gitnexus_impact target=DefaultActiveRecallStrategy direction=upstream
gitnexus_impact target=KnowledgeCompiler direction=upstream
```

Expected:

```text
Record risk level and direct callers before editing.
```

- [ ] **Step 2: Add failing facade composition tests**

Add tests in `macaca/crates/macaca-memory/src/runtime/mod.rs` proving:

```rust
#[tokio::test]
async fn runtime_facade_routes_remember_and_search_through_inner_facade() {
    // Build runtime over MemoryFabricFacade with builtin private/shared adapters.
    // Remember into SessionShared.
    // Search same SessionShared scope.
    // Assert one remembered row is returned.
}

#[tokio::test]
async fn runtime_status_reports_composed_capabilities() {
    // Build runtime with builtin providers only.
    // Assert status provider id is memory-runtime.
    // Assert store/search/active_recall/knowledge fields are represented truthfully.
}
```

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-memory runtime:: --lib
```

Expected:

```text
Tests fail because runtime module does not exist.
```

- [ ] **Step 3: Implement `MemoryRuntimeFacade`**

Create `runtime/facade.rs` with:

```rust
#[async_trait::async_trait]
pub trait MemoryRuntimeFacade: Send + Sync {
    async fn remember(&self, request: crate::MemoryWriteRequest) -> macaca_proto::MacacaResult<macaca_proto::MemoryId>;
    async fn search(&self, request: crate::MemorySearchRequest) -> macaca_proto::MacacaResult<Vec<macaca_proto::MemoryEntry>>;
    async fn active_recall(&self, request: crate::ActiveRecallRequest) -> macaca_proto::MacacaResult<crate::ActiveRecallResult>;
    async fn compile_knowledge(&self, request: crate::KnowledgeCompileRequest) -> macaca_proto::MacacaResult<crate::KnowledgeCompileResult>;
    async fn status(&self) -> crate::runtime::MemoryRuntimeStatus;
}
```

Create a concrete composed runtime:

```rust
pub struct ComposedMemoryRuntime<F, A, K> {
    facade: std::sync::Arc<F>,
    active_recall: std::sync::Arc<A>,
    knowledge: std::sync::Arc<K>,
}
```

Use generic bounds instead of concrete app names.

- [ ] **Step 4: Implement status DTO**

Create `runtime/status.rs`:

```rust
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MemoryRuntimeStatus {
    pub runtime_id: String,
    pub provider_profile: Option<String>,
    pub store_available: bool,
    pub search_available: bool,
    pub active_recall_available: bool,
    pub knowledge_available: bool,
    pub diagnostics: Vec<String>,
}
```

- [ ] **Step 5: Implement builder**

Create `runtime/builder.rs` with a small builder that takes already-built components:

```rust
pub struct MemoryRuntimeBuilder<F, A, K> {
    facade: std::sync::Arc<F>,
    active_recall: std::sync::Arc<A>,
    knowledge: std::sync::Arc<K>,
}
```

Do not make this builder read application-specific config yet.

- [ ] **Step 6: Export runtime module**

Modify `macaca/crates/macaca-memory/src/lib.rs`:

```rust
pub mod runtime;
pub use runtime::{ComposedMemoryRuntime, MemoryRuntimeBuilder, MemoryRuntimeFacade, MemoryRuntimeStatus};
```

- [ ] **Step 7: Verify**

Run:

```bash
cargo fmt
cargo test -p macaca-memory runtime:: --lib
cargo test -p macaca-memory --lib
```

Expected:

```text
All macaca-memory library tests pass.
```

## Task 3: Provider Runtime and MCP Truthful Capability

**Files:**

- Modify: `macaca/crates/macaca-memory/src/providers/runtime.rs`
- Modify: `macaca/crates/macaca-memory/src/providers/factory.rs`
- Modify: `macaca/crates/macaca-memory/src/providers/mcp.rs`
- Add tests in `macaca/crates/macaca-memory/src/providers/tests.rs`

- [ ] **Step 1: Run GitNexus impact**

Run:

```text
gitnexus_impact target=MemoryProviderRuntime direction=upstream
gitnexus_impact target=McpMemoryProvider direction=upstream
gitnexus_impact target=BuiltinMemoryProviderFactory direction=upstream
```

- [ ] **Step 2: Add failing tests for provider status truthfulness**

Add tests:

```rust
#[test]
fn mcp_provider_without_live_client_reports_unavailable_capabilities() {
    // Build McpMemoryProvider with command config and no live client.
    // Assert status.available is false or capabilities store/search are false.
}

#[test]
fn provider_runtime_status_lists_resolved_profile_slots() {
    // Configure default profile with distinct agent_private_provider and session_shared_provider.
    // Assert runtime status includes both selected provider ids.
}
```

Run:

```bash
cargo test -p macaca-memory providers:: --lib
```

Expected:

```text
At least one test fails until status truthfulness is implemented.
```

- [ ] **Step 3: Change MCP status semantics**

In `providers/mcp.rs`, change status and descriptor so an unwired MCP provider does not claim healthy store/search.

Required behavior:

```text
status.provider_id == configured id
status.healthy == false or status.diagnostics contains "mcp_transport_unwired"
store/search capabilities are false until live client exists
```

- [ ] **Step 4: Add live MCP client seam**

Add a trait in `providers/mcp.rs`:

```rust
#[async_trait::async_trait]
pub trait MemoryMcpClient: Send + Sync {
    async fn call_tool(&self, tool_name: &str, input: serde_json::Value) -> macaca_proto::MacacaResult<serde_json::Value>;
}
```

Add `McpMemoryProvider::with_client(...)` that enables live capability.

- [ ] **Step 5: Implement schema mapping**

When a client exists:

```text
remember -> configured memory_store or memory_write tool
search -> configured memory_search tool
get -> configured memory_get tool
delete -> configured memory_delete or memory_forget tool
```

Use configured tool names from `MemoryProviderToolConfig`. If a required tool is absent, return a capability error and status diagnostic.

- [ ] **Step 6: Verify provider runtime**

Run:

```bash
cargo fmt
cargo test -p macaca-memory providers:: --lib
openspec validate add-memory-provider-runtime --strict
```

Expected:

```text
Provider tests pass and OpenSpec remains valid.
```

## Task 4: Embedding Registry and Decorators

**Files:**

- Create: `macaca/crates/macaca-memory/src/embedding_registry.rs`
- Create: `macaca/crates/macaca-memory/src/embedding_decorators.rs`
- Modify: `macaca/crates/macaca-memory/src/lib.rs`
- Add tests in the new modules.

- [ ] **Step 1: Run GitNexus impact**

Run:

```text
gitnexus_impact target=EmbeddingProvider direction=upstream
gitnexus_impact target=CachedEmbeddingProvider direction=upstream
gitnexus_impact target=DashScopeEmbedding direction=upstream
```

- [ ] **Step 2: Add failing registry tests**

Add tests:

```rust
#[tokio::test]
async fn registry_resolves_default_embedding_provider() {
    // Register "mock" provider factory.
    // Resolve default profile.
    // Embed one text and assert vector dimensions.
}

#[tokio::test]
async fn timeout_decorator_returns_error_without_panicking() {
    // Wrap a provider that sleeps longer than timeout.
    // Assert error contains embedding timeout diagnostic.
}

#[tokio::test]
async fn retry_decorator_retries_then_succeeds() {
    // Wrap a provider that fails once then succeeds.
    // Assert two calls and successful vector output.
}
```

- [ ] **Step 3: Implement registry**

Define:

```rust
pub trait EmbeddingProviderFactory: Send + Sync {
    fn provider_id(&self) -> &str;
    fn dimensions(&self) -> usize;
    fn build(&self) -> macaca_proto::MacacaResult<std::sync::Arc<dyn crate::EmbeddingProvider>>;
}

pub struct EmbeddingProviderRegistry {
    factories: std::collections::HashMap<String, std::sync::Arc<dyn EmbeddingProviderFactory>>,
}
```

- [ ] **Step 4: Implement decorators**

Add decorators:

```text
CachedEmbeddingProvider already exists and remains.
TimeoutEmbeddingProvider wraps embed() in tokio::time::timeout.
RetryEmbeddingProvider retries MacacaError::Memory failures up to configured count.
MetricsEmbeddingProvider records call count, failure count, and last latency in an in-process struct.
```

Do not add external metrics dependencies.

- [ ] **Step 5: Export**

Modify `lib.rs`:

```rust
pub mod embedding_registry;
pub mod embedding_decorators;
pub use embedding_registry::{EmbeddingProviderFactory, EmbeddingProviderRegistry};
pub use embedding_decorators::{EmbeddingMetrics, MetricsEmbeddingProvider, RetryEmbeddingProvider, TimeoutEmbeddingProvider};
```

- [ ] **Step 6: Verify**

Run:

```bash
cargo fmt
cargo test -p macaca-memory embedding --lib
cargo test -p macaca-memory embedding_registry --lib
cargo test -p macaca-memory embedding_decorators --lib
```

Expected:

```text
All embedding tests pass.
```

## Task 5: Vector Backend Conformance and Query Pipeline

**Files:**

- Create: `macaca/crates/macaca-memory/src/vector_conformance.rs`
- Create: `macaca/crates/macaca-memory/src/query_pipeline.rs`
- Modify: `macaca/crates/macaca-memory/src/vector_backend.rs`
- Modify: `macaca/crates/macaca-memory/src/query.rs`
- Modify: `macaca/crates/macaca-memory/src/lib.rs`

- [ ] **Step 1: Run GitNexus impact**

Run:

```text
gitnexus_impact target=VectorMemoryBackend direction=upstream
gitnexus_impact target=TopologyVectorMemoryBackend direction=upstream
gitnexus_impact target=VectorQueryStrategy direction=upstream
```

- [ ] **Step 2: Add conformance harness**

Create reusable functions:

```rust
pub async fn assert_vector_backend_conformance<B>(backend: &B)
where
    B: crate::VectorMemoryBackend,
{
    assert_private_collections_are_isolated(backend).await;
    assert_shared_collection_is_explicit(backend).await;
    assert_status_reports_topology(backend).await;
}
```

Move existing private conformance checks from `vector_backend.rs` tests into this module so alternative backends can reuse them.

- [ ] **Step 3: Add query pipeline tests**

Add tests:

```rust
#[tokio::test]
async fn hybrid_query_falls_back_to_keyword_when_embedding_fails() {
    // Build query pipeline with failing embedding provider and keyword index.
    // Assert keyword results are returned and diagnostics include vector_degraded.
}

#[tokio::test]
async fn filtered_query_keeps_only_matching_metadata() {
    // Build in-memory records with metadata.
    // Assert filter removes non-matching rows.
}
```

- [ ] **Step 4: Implement query pipeline**

Define strategies:

```rust
pub enum MemoryQueryMode {
    Keyword,
    Vector,
    Hybrid,
    Filtered,
}

#[async_trait::async_trait]
pub trait MemoryQueryPipeline: Send + Sync {
    async fn search(&self, request: MemorySearchRequest) -> macaca_proto::MacacaResult<MemoryQueryPipelineResult>;
}
```

`MemoryQueryPipelineResult` must include:

```text
entries
diagnostics
used_keyword
used_vector
```

- [ ] **Step 5: Keep default behavior unchanged**

Default runtime uses existing manager recall behavior unless explicitly configured for the new pipeline. No production behavior changes in this task.

- [ ] **Step 6: Verify**

Run:

```bash
cargo fmt
cargo test -p macaca-memory vector_backend --lib
cargo test -p macaca-memory vector_conformance --lib
cargo test -p macaca-memory query --lib
cargo test -p macaca-memory query_pipeline --lib
openspec validate add-memory-vector-backend-topology --strict
```

Expected:

```text
All tests pass and vector backend topology task 4.4 can be marked complete.
```

## Task 6: Knowledge Compiler, Wiki Digest, and Project Decision Artifacts

**Files:**

- Modify: `macaca/crates/macaca-memory/src/governance/compiler.rs`
- Modify: `macaca/crates/macaca-memory/src/governance/artifacts.rs`
- Modify: `macaca/crates/macaca-web/src/workspace_knowledge_digest_capability.rs`
- Modify: `macaca/crates/macaca-context/src/knowledge_digest/context_provider.rs`
- Modify: `macaca/crates/macaca-context/src/knowledge_digest/selection.rs`

- [ ] **Step 1: Run GitNexus impact**

Run:

```text
gitnexus_impact target=KnowledgeCompiler direction=upstream
gitnexus_impact target=WorkspaceKnowledgeDigestCapability direction=upstream
gitnexus_impact target=KnowledgeDigestContextProvider direction=upstream
```

- [ ] **Step 2: Add failing contradiction tests**

Add tests proving deterministic contradiction detection:

```rust
#[test]
fn compiler_groups_boolean_contradictions_by_subject() {
    // Candidate A: "deployment requires redis"
    // Candidate B: "deployment does not require redis"
    // Assert one conflict group with both claim ids.
}

#[test]
fn compiler_preserves_exact_evidence_ids_for_claims() {
    // Compile candidates with known memory ids.
    // Assert evidence source ids are unchanged.
}
```

- [ ] **Step 3: Implement deterministic contradiction strategy**

Do not add LLM calls. Implement a simple strategy boundary:

```rust
pub trait ContradictionStrategy: Send + Sync {
    fn detect(&self, claims: &[KnowledgeClaim]) -> Vec<ClaimGroup>;
}
```

Default strategy supports deterministic negation pairs:

```text
"X requires Y" conflicts with "X does not require Y"
"X is Y" conflicts with "X is not Y"
```

- [ ] **Step 4: Add artifacts**

Extend artifacts with:

```text
ProjectDecisionLogArtifact
WikiDigestArtifact
CitationArtifact
```

Each artifact must contain only bounded/redacted text by default and evidence ids for exact lookup.

- [ ] **Step 5: Add report redaction tests**

Add tests for OpenSpec unchecked items:

```rust
#[test]
fn structured_digest_report_does_not_serialize_full_sensitive_source_text() {
    // Build digest item with source text and evidence id.
    // Serialize report.
    // Assert evidence id exists and full sensitive text is absent.
}

#[test]
fn redacted_digest_rendering_keeps_evidence_refs() {
    // Render redacted digest.
    // Assert body contains evidence_refs and not raw memory body.
}
```

- [ ] **Step 6: Verify**

Run:

```bash
cargo fmt
cargo test -p macaca-memory governance --lib
cargo test -p macaca-context knowledge_digest --lib
cargo test -p macaca-web workspace_knowledge_digest --lib
openspec validate add-memory-governance-knowledge-layer --strict
openspec validate add-knowledge-digest-context-provider --strict
```

Expected:

```text
All tests pass and previously unchecked redaction/report tasks can be marked complete.
```

## Task 7: Governance Runtime and Provider Migration

**Files:**

- Create: `macaca/crates/macaca-memory/src/governance/runtime.rs`
- Create: `macaca/crates/macaca-memory/src/governance/migration.rs`
- Modify: `macaca/crates/macaca-memory/src/governance/mod.rs`
- Modify: `macaca/crates/macaca-memory/src/governance/facade.rs`
- Modify: `macaca/crates/macaca-memory/src/snapshot.rs`

- [ ] **Step 1: Run GitNexus impact**

Run:

```text
gitnexus_impact target=GovernedMemoryFacade direction=upstream
gitnexus_impact target=MemorySnapshotStore direction=upstream
gitnexus_impact target=MemoryPromotionPolicy direction=upstream
```

- [ ] **Step 2: Add governance runtime tests**

Add tests:

```rust
#[tokio::test]
async fn governance_runtime_captures_candidate_and_promotes_with_audit() {
    // Capture candidate.
    // Promote candidate.
    // Assert audit events include captured and promoted.
}

#[tokio::test]
async fn migration_keeps_source_authoritative_when_verification_fails() {
    // Source has two entries.
    // Target migration intentionally drops one entry.
    // Assert migration result is failed and source remains authoritative.
}
```

- [ ] **Step 3: Implement durable governance runtime boundary**

Define:

```rust
pub trait MemoryGovernanceJournal: Send + Sync {
    async fn append_audit(&self, event: MemoryAuditEvent) -> MacacaResult<()>;
    async fn list_audits(&self, scope: &MemoryScope, limit: usize) -> MacacaResult<Vec<MemoryAuditEvent>>;
}
```

Start with in-memory implementation plus snapshot/replay integration. Do not add a database dependency.

- [ ] **Step 4: Implement automatic candidate capture seam**

Add:

```rust
pub trait MemoryCandidateCapturePolicy: Send + Sync {
    fn should_capture(&self, source: &CandidateSource, content: &str) -> bool;
}
```

Default policy captures explicit user memory and agent summaries, rejects empty content, and records diagnostics.

- [ ] **Step 5: Implement provider migration plan**

Add migration types:

```rust
pub struct MemoryProviderMigrationPlan {
    pub source_provider_id: String,
    pub target_provider_id: String,
    pub scope: MemoryScope,
    pub batch_size: usize,
}

pub enum MemoryProviderMigrationStatus {
    Planned,
    Copying,
    Verifying,
    Completed,
    Failed,
    RolledBack,
}
```

Migration must copy, verify count/evidence ids, then mark completed. Failed verification must leave source authoritative.

- [ ] **Step 6: Add compaction/dreaming seam**

Add only the runtime seam in this plan:

```rust
pub trait MemoryCompactionStrategy: Send + Sync {
    async fn compact(&self, scope: MemoryScope, entries: Vec<MemoryEntry>) -> MacacaResult<Vec<MemoryCandidate>>;
}
```

Default implementation returns an empty candidate list and records `compaction_disabled`. This is truthful and avoids pretending dreaming is complete.

- [ ] **Step 7: Verify**

Run:

```bash
cargo fmt
cargo test -p macaca-memory governance --lib
cargo test -p macaca-memory snapshot --lib
```

Expected:

```text
Governance runtime tests pass.
```

## Task 8: Migrate macaca-web Production Consumers

**Files:**

- Create: `macaca/crates/macaca-web/src/memory_runtime.rs`
- Modify: `macaca/crates/macaca-web/src/state.rs`
- Modify: `macaca/crates/macaca-web/src/lib.rs`
- Modify: `macaca/crates/macaca-web/src/context_reporting_model.rs`
- Modify: `macaca/crates/macaca-web/src/context_memory_tools.rs`
- Modify: `macaca/crates/macaca-web/src/workspace_memory_recall_source.rs`
- Modify: `macaca/crates/macaca-web/src/workspace_knowledge_digest_capability.rs`

- [ ] **Step 1: Run GitNexus impact**

Run:

```text
gitnexus_impact target=ContextReportingChatModel direction=upstream
gitnexus_impact target=WorkspaceMemoryRecallSource direction=upstream
gitnexus_impact target=WorkspaceMemorySearchTool direction=upstream
gitnexus_impact target=WorkspaceKnowledgeDigestCapability direction=upstream
```

Warn before editing if any result is HIGH or CRITICAL.

- [ ] **Step 2: Add web adapter tests**

Add tests proving runtime-backed behavior:

```rust
#[tokio::test]
async fn web_memory_runtime_search_tool_uses_runtime_facade() {
    // Build fake runtime facade with one known entry.
    // Execute memory_search tool.
    // Assert returned id/content matches fake runtime.
}

#[tokio::test]
async fn active_recall_source_uses_runtime_and_preserves_scope_filtering() {
    // Build fake runtime facade with private and shared rows.
    // Query with current agent id.
    // Assert other agent private row is hidden.
}
```

- [ ] **Step 3: Add `WebMemoryRuntime` adapter**

Create `memory_runtime.rs`:

```rust
pub struct WebMemoryRuntime {
    inner: std::sync::Arc<dyn macaca_memory::MemoryRuntimeFacade>,
}
```

Add methods needed by tools and context:

```text
remember_text
search
get
delete
active_recall
compile_knowledge
status
```

- [ ] **Step 4: Update `AppState`**

Add:

```rust
pub memory_runtime: Option<Arc<WebMemoryRuntime>>,
```

Keep `workspace_memory` temporarily for compatibility but mark comments as legacy builtin backing store.

- [ ] **Step 5: Initialize runtime in web bootstrap**

In `lib.rs`, after existing `workspace_memory` initialization, build a runtime facade over the builtin adapter. Default behavior must stay identical when `context.recall.expose_memory_tools` is enabled.

- [ ] **Step 6: Migrate tools**

Change `WorkspaceMemorySearchTool`, `WorkspaceMemoryGetTool`, and `WorkspaceMemoryForgetTool` to depend on the runtime adapter. Keep a legacy constructor only for tests that explicitly exercise old manager behavior.

- [ ] **Step 7: Migrate active recall and knowledge digest adapters**

Change `WorkspaceMemoryRecallSource` and `WorkspaceKnowledgeDigestCapability` to call runtime facade methods. Preserve existing scope/tombstone filtering.

- [ ] **Step 8: Verify no duplicate recall path**

Run:

```bash
rg -n "TestMemoryManager|workspace_memory\\.recall|memory\\.recall\\(" macaca/crates/macaca-web/src
```

Expected:

```text
Remaining matches are limited to bootstrap legacy backing store, tests, or compatibility adapters.
```

- [ ] **Step 9: Verify**

Run:

```bash
cargo fmt
cargo test -p macaca-web context_memory --lib
cargo test -p macaca-web workspace_memory --lib
cargo check -p macaca-memory -p macaca-context -p macaca-web
```

Expected:

```text
All tests and checks pass.
```

## Task 9: End-to-End Verification and GitNexus

**Files:**

- No new source files unless tests reveal a bug.

- [ ] **Step 1: Run full focused verification**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-memory --lib
cargo test -p macaca-context --lib
cargo test -p macaca-web --lib
cargo check -p macaca-memory -p macaca-context -p macaca-web -p macaca-framework -p macaca-kernel
```

Expected:

```text
All commands pass.
```

- [ ] **Step 2: Validate all related OpenSpec changes**

Run:

```bash
cd /Users/quantum/Code/dev/agent
openspec validate complete-memory-fabric-runtime --strict
openspec validate add-memory-fabric-core --strict
openspec validate add-memory-provider-runtime --strict
openspec validate add-memory-vector-backend-topology --strict
openspec validate add-memory-active-recall-integration --strict
openspec validate add-active-vector-memory-context --strict
openspec validate add-memory-governance-knowledge-layer --strict
openspec validate add-knowledge-digest-context-provider --strict
```

Expected:

```text
All listed changes are valid.
```

- [ ] **Step 3: Run duplicate/direct access scans**

Run:

```bash
rg -n "not wired to a live client yet|workspace_memory\\.recall|TestMemoryManager" macaca/crates/macaca-memory/src macaca/crates/macaca-web/src
rg -n "MemoryProviderRuntimeStatus|MemoryRuntimeStatus|provider_runtime" macaca/crates/macaca-memory/src macaca/crates/macaca-web/src macaca/crates/macaca-context/src
```

Expected:

```text
No false healthy MCP provider remains.
Direct TestMemoryManager recall is limited to legacy backing store or tests.
Runtime status paths are present.
```

- [ ] **Step 4: Run GitNexus detect changes**

Run:

```text
gitnexus_detect_changes(scope="all")
```

Expected:

```text
Changed symbols and affected flows match memory/runtime/context/web migration scope.
No unrelated execution flows are affected.
```

- [ ] **Step 5: Optional live E2E**

Only after focused tests pass, start backend and frontend outside sandbox if requested:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo run -- web
cd /Users/quantum/Code/dev/agent/frontend
npm run dev
```

Manual E2E requirements:

```text
Create a session.
Trigger a task requiring memory recall.
Observe active recall diagnostics in context report.
Use memory_search/memory_get/memory_forget.
Refresh session and confirm reports/events remain visible.
Provider diagnostics are visible and no duplicate memory injection occurs.
```

## Self-Review

Spec coverage:

- Phase 1 protected by runtime facade compatibility and existing tests.
- Phase 2 covered by tasks 2, 3, and 8.
- Phase 3 covered by tasks 4 and 5.
- Phase 4 protected by task 8 migration and existing active recall tests.
- Phase 5 covered by task 6.
- Phase 6 covered by task 7.

Placeholder scan:

- No placeholder markers or intentionally incomplete sections are present.

Risk notes:

- `macaca-web` migration is the highest-risk slice because it affects live recall and user-visible context reports. It must be done after lower-crate contracts pass.
- MCP provider must not continue to advertise healthy capability until live client wiring exists.
- Governance compaction/dreaming should start as a truthful seam with disabled diagnostics if full autonomous dreaming is not implemented in this slice.
