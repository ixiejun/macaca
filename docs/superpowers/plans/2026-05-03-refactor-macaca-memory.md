# Refactor macaca-memory Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor `macaca-memory` with design-pattern primitives while preserving current memory behavior 1:1.

**Architecture:** Use additive-first primitives around the existing `MemoryStore`, `EmbeddingProvider`, `VectorStore`, `MemoryManager`, and `IsolatedMemoryManager`. Each slice introduces a small contract or helper first, keeps current APIs compiling, then verifies existing behavior before any later consumer migration.

**Tech Stack:** Rust, `async-trait`, `tokio`, `serde`, `serde_json`, `macaca-proto`, OpenSpec, GitNexus.

---

## Context

`macaca-memory` is a stage-1 bottom-layer crate in `macaca/docs/design-pattern-refactor-plans/refactor-order.md`. It depends only on `macaca-proto` and is consumed primarily by `macaca-kernel` through `MemoryServiceAdapter`, with future relevance to agent services, framework runtime memory, session resume, and long-running autonomous operation.

Current implementation:

- `store.rs` defines `MemoryStore`, `MemoryRetriever`, `EmbeddingProvider`, `VectorStore`, `MemoryQueryContext`, and `VectorSearchResult`.
- `session.rs` implements TTL-backed in-memory `SessionMemory`.
- `file.rs` implements file-backed `FileMemory`.
- `vector.rs` implements `InMemoryVectorStore` and `MilvusStore`.
- `embedding.rs` implements `DashScopeEmbedding` and `MockEmbedding`.
- `manager.rs` has `MemoryManager<V, E>` directly orchestrating session/file/vector/embedding.
- `isolated.rs` has `IsolatedMemoryManager<V, E>` duplicating much of `MemoryManager` orchestration with app/agent isolation.

Baseline verification already observed:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-memory -- --nocapture
```

Observed result:

```text
32 passed; 0 failed; 2 ignored
```

## Superpowers Brainstorm

### Option A: Facade-only first slice

Add convenience facade methods to `MemoryManager` and `IsolatedMemoryManager`, leave backend/cache/query abstractions for later.

Benefits:

- Lowest risk.
- Very small blast radius.
- Easy to prove behavior is unchanged.

Risks:

- Does not address duplicated embed/vector orchestration.
- Does not prepare enough structure for future resume/debugging or backend creation.
- Later slices may still need to touch the same manager code repeatedly.

### Option B: Five additive slices matching the crate plan

Implement all five planned primitives as independent additive slices:

- Facade methods on managers.
- `EmbeddingCache` / cached embedding decorator.
- `MemoryBackendFactory` for backend construction.
- Snapshot memento schema and replay helpers.
- Vector query strategy abstraction.

Benefits:

- Matches `macaca/docs/design-pattern-refactor-plans/macaca-memory.md`.
- Keeps each slice independently testable and reversible.
- Builds enough foundation before any upper-crate migration.
- Avoids application-specific logic and keeps `macaca-memory` generic infrastructure.

Risks:

- More code than Option A.
- Requires careful API naming to avoid locking in poor abstractions.
- Dynamic backend factory must avoid overreaching because current managers are generic over `VectorStore` and `EmbeddingProvider`.

### Option C: Replace manager internals with a new backend abstraction immediately

Introduce a new `MemoryBackend` trait and rewrite `MemoryManager` / `IsolatedMemoryManager` to depend on that trait directly.

Benefits:

- Cleans duplication fastest.
- Produces a clearer end-state in one step.

Risks:

- Too large for the project’s incremental-refactor rule.
- High chance of behavior drift in deduplication, vector fallback, isolation, and TTL behavior.
- Affects downstream kernel adapter and tests more than needed.

### Recommendation

Use Option B, but execute it as five strict additive slices. Do not remove or replace existing manager APIs in this change. Every new primitive must be covered by tests before touching manager internals. Any old API that becomes superseded should be marked deprecated only after all direct in-crate call sites have an additive alternative.

## Design Pattern Mapping

- **Facade:** `MemoryManager` and `IsolatedMemoryManager` expose higher-level `remember_*`, `recall_*`, and `forget_*` methods so callers do not need to know session/file/vector/embedding details.
- **Proxy + Decorator:** `CachedEmbeddingProvider<E>` wraps any `EmbeddingProvider` and adds cache behavior without modifying concrete providers.
- **Flyweight:** `EmbeddingCache` reuses vectors for identical text inputs inside the process.
- **Factory Method / Abstract Factory:** `MemoryBackendFactory` creates standard memory manager variants from config without upper crates hardcoding concrete backend construction.
- **Memento:** `MemorySnapshot` captures memory state for session resume and debugging.
- **Strategy:** `VectorQueryStrategy` encapsulates vector search request shape and result filtering, allowing hybrid search and metadata-filter variants later.

## Scope

In scope:

- Additive `macaca-memory` refactor primitives only.
- Tests proving new primitives preserve current behavior.
- OpenSpec proposal/design/tasks/spec before implementation.
- Deprecation markers only where a new additive path exists and current behavior remains available.

Out of scope:

- Migrating `macaca-kernel`, `macaca-agent`, `macaca-framework`, or `macaca-web` consumers.
- Changing Milvus schema or DashScope request semantics.
- Adding a new external dependency.
- Building production distributed embedding cache.
- Changing session TTL, file layout, vector payload keys, or agent isolation semantics.

## Files

- Create: `openspec/changes/refactor-macaca-memory-patterns/proposal.md`
- Create: `openspec/changes/refactor-macaca-memory-patterns/design.md`
- Create: `openspec/changes/refactor-macaca-memory-patterns/tasks.md`
- Create: `openspec/changes/refactor-macaca-memory-patterns/specs/macaca-memory-core/spec.md`
- Create: `macaca/crates/macaca-memory/src/facade.rs`
- Create: `macaca/crates/macaca-memory/src/cache.rs`
- Create: `macaca/crates/macaca-memory/src/backend.rs`
- Create: `macaca/crates/macaca-memory/src/snapshot.rs`
- Create: `macaca/crates/macaca-memory/src/query.rs`
- Modify: `macaca/crates/macaca-memory/src/lib.rs`
- Modify: `macaca/crates/macaca-memory/src/manager.rs`
- Modify: `macaca/crates/macaca-memory/src/isolated.rs`
- Modify: `macaca/crates/macaca-memory/src/session.rs`
- Modify: `macaca/crates/macaca-memory/src/file.rs`
- Modify: `macaca/crates/macaca-memory/src/vector.rs`
- Modify: `macaca/crates/macaca-memory/src/embedding.rs`

## Task 1: OpenSpec Change

**Files:**

- Create: `openspec/changes/refactor-macaca-memory-patterns/proposal.md`
- Create: `openspec/changes/refactor-macaca-memory-patterns/design.md`
- Create: `openspec/changes/refactor-macaca-memory-patterns/tasks.md`
- Create: `openspec/changes/refactor-macaca-memory-patterns/specs/macaca-memory-core/spec.md`

- [ ] **Step 1: Check active changes**

Run:

```bash
cd /Users/quantum/Code/dev/agent
openspec list
openspec list --specs
test ! -e openspec/changes/refactor-macaca-memory-patterns
```

Expected:

```text
The test command exits with status 0.
```

- [ ] **Step 2: Create proposal**

Create `openspec/changes/refactor-macaca-memory-patterns/proposal.md`:

```markdown
# Change: Refactor macaca-memory with design-pattern primitives

## Why

`macaca-memory` currently exposes useful storage traits, but manager orchestration directly combines session, file, vector, and embedding behavior. The duplication makes future session resume, backend selection, embedding cache, and query strategy work harder to evolve safely.

## What Changes

- Add facade request/result helpers for manager-level remember/recall/forget flows.
- Add an in-process embedding cache and cached embedding decorator.
- Add backend factory config for standard memory manager construction.
- Add memory snapshot/replay memento types.
- Add vector query strategy primitives for current similarity search and future filtered/hybrid search.

## Impact

- Affected specs: `macaca-memory-core`
- Affected code: `macaca-memory`
- Compatibility: existing public store/provider/vector traits and manager methods remain available.
- Non-impact: no changes to `macaca-kernel`, `macaca-web`, `macaca-agent`, `macaca-framework`, task scheduling, trace, session, driver, skill, or MCP behavior.
```

- [ ] **Step 3: Create design**

Create `openspec/changes/refactor-macaca-memory-patterns/design.md`:

```markdown
## Context

`macaca-memory` is a stage-1 infrastructure crate. It should provide stable memory primitives for long-running agents without depending on application-specific workflow logic.

Current managers duplicate three-layer orchestration: session memory, file memory, optional embedding, and optional vector search. This change introduces small additive primitives around that behavior rather than replacing it.

## Goals

- Preserve existing behavior 1:1.
- Keep existing public methods compiling.
- Add extension seams for facade calls, embedding cache, backend construction, snapshots, and query strategy.
- Keep the crate generic and application-agnostic.

## Non-Goals

- Do not migrate upper-crate consumers.
- Do not remove existing public methods.
- Do not change file layout, vector payload shape, Milvus endpoints, DashScope request shape, session TTL semantics, or isolation rules.
- Do not add external dependencies.

## Decisions

- Facade methods call existing `store`, `retrieve`, `list`, `get`, and `delete` methods internally.
- `CachedEmbeddingProvider<E>` wraps an existing provider and caches per input text.
- Backend factory starts with standard in-process/file/in-memory-vector construction only; Milvus factory wiring remains additive and explicit.
- Snapshot types are serializable mementos and do not become the persistence backend.
- Vector query strategy starts with current similarity search behavior and optional metadata filter support.
```

- [ ] **Step 4: Create tasks**

Create `openspec/changes/refactor-macaca-memory-patterns/tasks.md` with the same five implementation slices as this plan:

```markdown
## 1. Preparation

- [ ] 1.1 Run GitNexus impact for `MemoryManager` upstream.
- [ ] 1.2 Run GitNexus impact for `IsolatedMemoryManager` upstream.
- [ ] 1.3 Run baseline `cargo test -p macaca-memory -- --nocapture`.

## 2. Facade slice

- [ ] 2.1 Add facade request/result types.
- [ ] 2.2 Add manager facade methods.
- [ ] 2.3 Add isolated manager facade methods.
- [ ] 2.4 Add tests proving facade methods call existing behavior.

## 3. Embedding cache slice

- [ ] 3.1 Add `EmbeddingCache`.
- [ ] 3.2 Add `CachedEmbeddingProvider<E>`.
- [ ] 3.3 Add tests for cache hits, misses, and batch behavior.

## 4. Backend factory slice

- [ ] 4.1 Add backend config types.
- [ ] 4.2 Add factory methods for standard managers.
- [ ] 4.3 Add tests for deterministic construction.

## 5. Snapshot memento slice

- [ ] 5.1 Add snapshot schema.
- [ ] 5.2 Add snapshot/replay helpers for session and file stores.
- [ ] 5.3 Add tests for snapshot round-trip.

## 6. Vector query strategy slice

- [ ] 6.1 Add vector query strategy traits and request types.
- [ ] 6.2 Add default similarity strategy.
- [ ] 6.3 Add metadata-filter strategy tests.

## 7. Verification

- [ ] 7.1 Run `cargo fmt`.
- [ ] 7.2 Run `cargo test -p macaca-memory -- --nocapture`.
- [ ] 7.3 Run `cargo check -p macaca-memory -p macaca-kernel -p macaca-agent -p macaca-framework -p macaca-web`.
- [ ] 7.4 Run `openspec validate refactor-macaca-memory-patterns --strict`.
- [ ] 7.5 Run `gitnexus_detect_changes(scope: "all")`.
```

- [ ] **Step 5: Create delta spec**

Create `openspec/changes/refactor-macaca-memory-patterns/specs/macaca-memory-core/spec.md`:

```markdown
## ADDED Requirements

### Requirement: Memory manager facade APIs

`macaca-memory` SHALL provide additive facade APIs for common remember, recall, list, get, and forget operations while preserving existing manager methods.

#### Scenario: Remember and recall through facade

- **GIVEN** a memory manager with session and file stores
- **WHEN** a caller stores text through the facade and recalls it by query
- **THEN** the returned entries match the existing `store` and `retrieve` behavior.

### Requirement: Cached embedding provider

`macaca-memory` SHALL provide a decorator that caches embedding vectors per text input without changing the `EmbeddingProvider` trait.

#### Scenario: Repeated text uses cached vector

- **GIVEN** a cached embedding provider wrapping a counting provider
- **WHEN** the same text is embedded twice
- **THEN** the wrapped provider is called once for that text
- **AND** both embedding responses are identical.

### Requirement: Memory backend factory

`macaca-memory` SHALL provide additive backend factory configuration for standard manager construction without requiring upper crates to hardcode all concrete stores.

#### Scenario: Standard test manager construction

- **GIVEN** a file path and session TTL
- **WHEN** a standard in-memory-vector manager is created through the factory
- **THEN** storing and retrieving memory behaves like direct `MemoryManager::new` construction.

### Requirement: Memory snapshot memento

`macaca-memory` SHALL provide serializable snapshot types and replay helpers for session/file memory debugging and resume support.

#### Scenario: Snapshot round-trip

- **GIVEN** a memory store with two entries
- **WHEN** a snapshot is captured and replayed into an empty store
- **THEN** listing the destination store returns the same entries by memory id and content.

### Requirement: Vector query strategy

`macaca-memory` SHALL provide vector query strategy primitives that preserve current similarity search behavior and allow metadata filtering.

#### Scenario: Similarity search remains default

- **GIVEN** an in-memory vector store with multiple vectors
- **WHEN** the default strategy searches with a query vector
- **THEN** result ordering matches the existing vector store similarity ordering.
```

- [ ] **Step 6: Validate OpenSpec**

Run:

```bash
cd /Users/quantum/Code/dev/agent
openspec validate refactor-macaca-memory-patterns --strict
```

Expected:

```text
Change 'refactor-macaca-memory-patterns' is valid
```

## Task 2: GitNexus Impact And Baseline

**Files:**

- No file edits.

- [ ] **Step 1: Run impact for `MemoryManager`**

Run GitNexus:

```text
gitnexus_impact({
  "repo": "agent",
  "target": "MemoryManager",
  "direction": "upstream",
  "maxDepth": 3,
  "includeTests": true
})
```

Expected:

- Risk and direct callers are recorded before editing.
- If risk is HIGH or CRITICAL, report the blast radius before modifying code.

- [ ] **Step 2: Run impact for `IsolatedMemoryManager`**

Run GitNexus:

```text
gitnexus_impact({
  "repo": "agent",
  "target": "IsolatedMemoryManager",
  "direction": "upstream",
  "maxDepth": 3,
  "includeTests": true
})
```

Expected:

- Risk and direct callers are recorded before editing.
- If risk is HIGH or CRITICAL, report the blast radius before modifying code.

- [ ] **Step 3: Run baseline memory tests**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-memory -- --nocapture
```

Expected:

```text
test result: ok. 32 passed; 0 failed; 2 ignored
```

## Task 3: Facade Slice

**Files:**

- Create: `macaca/crates/macaca-memory/src/facade.rs`
- Modify: `macaca/crates/macaca-memory/src/lib.rs`
- Modify: `macaca/crates/macaca-memory/src/manager.rs`
- Modify: `macaca/crates/macaca-memory/src/isolated.rs`

- [ ] **Step 1: Add facade request/result types**

Create `macaca/crates/macaca-memory/src/facade.rs`:

```rust
use macaca_proto::{AgentId, MemoryEntry, MemoryId, MemoryLayer};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct RememberText {
    pub content: String,
    pub layer: MemoryLayer,
    pub metadata: Value,
    pub agent_id: Option<AgentId>,
}

impl RememberText {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            layer: MemoryLayer::Session,
            metadata: Value::Null,
            agent_id: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecallQuery {
    pub query: String,
    pub limit: usize,
}

impl RecallQuery {
    pub fn new(query: impl Into<String>, limit: usize) -> Self {
        Self {
            query: query.into(),
            limit,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RecallResult {
    pub entries: Vec<MemoryEntry>,
}

impl RecallResult {
    pub fn new(entries: Vec<MemoryEntry>) -> Self {
        Self { entries }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForgetMemory {
    pub id: MemoryId,
}
```

- [ ] **Step 2: Export facade module**

Modify `macaca/crates/macaca-memory/src/lib.rs`:

```rust
pub mod facade;
pub use facade::{ForgetMemory, RecallQuery, RecallResult, RememberText};
```

- [ ] **Step 3: Add facade methods to `MemoryManager`**

Add methods inside `impl<V: VectorStore, E: EmbeddingProvider> MemoryManager<V, E>` in `macaca/crates/macaca-memory/src/manager.rs`:

```rust
pub async fn remember_text(&self, input: crate::facade::RememberText) -> MacacaResult<MemoryId> {
    let entry = MemoryEntry {
        id: MemoryId::new(),
        layer: input.layer,
        content: input.content,
        metadata: input.metadata,
        agent_id: input.agent_id,
        created_at: chrono::Utc::now(),
        expires_at: None,
    };
    self.store(entry).await
}

pub async fn recall(&self, query: crate::facade::RecallQuery) -> MacacaResult<crate::facade::RecallResult> {
    self.retrieve(&query.query, query.limit)
        .await
        .map(crate::facade::RecallResult::new)
}

pub async fn forget(&self, input: crate::facade::ForgetMemory) -> MacacaResult<()> {
    let _ = self.session.delete(&input.id).await;
    let file_result = self.file.delete(&input.id).await;
    if let Some(vector) = &self.vector {
        let _ = vector.delete(&input.id.0.to_string()).await;
    }
    file_result
}
```

- [ ] **Step 4: Add facade methods to `IsolatedMemoryManager`**

Add methods inside `impl<V: VectorStore, E: EmbeddingProvider> IsolatedMemoryManager<V, E>` in `macaca/crates/macaca-memory/src/isolated.rs`:

```rust
pub async fn remember_text(&self, input: crate::facade::RememberText) -> MacacaResult<MemoryId> {
    let entry = MemoryEntry {
        id: MemoryId::new(),
        layer: input.layer,
        content: input.content,
        metadata: input.metadata,
        agent_id: input.agent_id,
        created_at: chrono::Utc::now(),
        expires_at: None,
    };
    self.store(entry).await
}

pub async fn recall(&self, query: crate::facade::RecallQuery) -> MacacaResult<crate::facade::RecallResult> {
    self.retrieve(&query.query, query.limit)
        .await
        .map(crate::facade::RecallResult::new)
}

pub async fn forget(&self, input: crate::facade::ForgetMemory) -> MacacaResult<()> {
    self.delete(&input.id).await
}
```

- [ ] **Step 5: Add facade tests**

Add tests to `manager.rs`:

```rust
#[tokio::test]
async fn facade_remember_recall_and_forget_text() {
    let dir = TempDir::new().unwrap();
    let mgr = make_manager(&dir);

    let id = mgr
        .remember_text(crate::facade::RememberText::new("facade memory"))
        .await
        .unwrap();
    let result = mgr
        .recall(crate::facade::RecallQuery::new("facade", 10))
        .await
        .unwrap();

    assert!(result.entries.iter().any(|entry| entry.id == id));

    mgr.forget(crate::facade::ForgetMemory { id }).await.unwrap();
    let result = mgr
        .recall(crate::facade::RecallQuery::new("facade", 10))
        .await
        .unwrap();
    assert!(result.entries.iter().all(|entry| entry.id != id));
}
```

Add tests to `isolated.rs`:

```rust
#[tokio::test]
async fn isolated_facade_forces_agent_scope() {
    let dir = TempDir::new().unwrap();
    let mgr = make_isolated(&dir);
    let id = mgr
        .remember_text(crate::facade::RememberText::new("isolated facade memory"))
        .await
        .unwrap();
    let entry = mgr.get(&id).await.unwrap().unwrap();

    assert_eq!(entry.agent_id, Some(mgr.agent_id()));
}
```

- [ ] **Step 6: Verify slice**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-memory facade -- --nocapture
cargo test -p macaca-memory -- --nocapture
```

Expected:

- New facade tests pass.
- Full crate remains `32+` passed, 2 ignored.

## Task 4: Embedding Cache Slice

**Files:**

- Create: `macaca/crates/macaca-memory/src/cache.rs`
- Modify: `macaca/crates/macaca-memory/src/lib.rs`

- [ ] **Step 1: Add embedding cache and decorator**

Create `macaca/crates/macaca-memory/src/cache.rs`:

```rust
use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use macaca_proto::MacacaResult;

use crate::store::EmbeddingProvider;

#[derive(Clone, Default)]
pub struct EmbeddingCache {
    inner: Arc<RwLock<HashMap<String, Vec<f32>>>>,
}

impl EmbeddingCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn get(&self, text: &str) -> Option<Vec<f32>> {
        self.inner.read().await.get(text).cloned()
    }

    pub async fn insert(&self, text: String, vector: Vec<f32>) {
        self.inner.write().await.insert(text, vector);
    }

    pub async fn len(&self) -> usize {
        self.inner.read().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }
}

pub struct CachedEmbeddingProvider<E> {
    inner: E,
    cache: EmbeddingCache,
}

impl<E> CachedEmbeddingProvider<E> {
    pub fn new(inner: E) -> Self {
        Self {
            inner,
            cache: EmbeddingCache::new(),
        }
    }

    pub fn with_cache(inner: E, cache: EmbeddingCache) -> Self {
        Self { inner, cache }
    }

    pub fn cache(&self) -> &EmbeddingCache {
        &self.cache
    }
}

#[async_trait]
impl<E: EmbeddingProvider> EmbeddingProvider for CachedEmbeddingProvider<E> {
    async fn embed(&self, texts: Vec<String>) -> MacacaResult<Vec<Vec<f32>>> {
        let mut output: Vec<Option<Vec<f32>>> = vec![None; texts.len()];
        let mut missing = Vec::new();
        let mut missing_indices = Vec::new();

        for (index, text) in texts.iter().enumerate() {
            if let Some(vector) = self.cache.get(text).await {
                output[index] = Some(vector);
            } else {
                missing_indices.push(index);
                missing.push(text.clone());
            }
        }

        if !missing.is_empty() {
            let vectors = self.inner.embed(missing.clone()).await?;
            for ((index, text), vector) in missing_indices
                .into_iter()
                .zip(missing.into_iter())
                .zip(vectors.into_iter())
            {
                self.cache.insert(text, vector.clone()).await;
                output[index] = Some(vector);
            }
        }

        Ok(output.into_iter().flatten().collect())
    }

    fn dimensions(&self) -> usize {
        self.inner.dimensions()
    }
}
```

- [ ] **Step 2: Export cache module**

Modify `macaca/crates/macaca-memory/src/lib.rs`:

```rust
pub mod cache;
pub use cache::{CachedEmbeddingProvider, EmbeddingCache};
```

- [ ] **Step 3: Add cache tests**

Add tests to `cache.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingEmbedding {
        calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl EmbeddingProvider for CountingEmbedding {
        async fn embed(&self, texts: Vec<String>) -> MacacaResult<Vec<Vec<f32>>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(texts
                .into_iter()
                .map(|text| vec![text.len() as f32])
                .collect())
        }

        fn dimensions(&self) -> usize {
            1
        }
    }

    #[tokio::test]
    async fn cached_provider_reuses_repeated_text() {
        let calls = Arc::new(AtomicUsize::new(0));
        let provider = CachedEmbeddingProvider::new(CountingEmbedding {
            calls: Arc::clone(&calls),
        });

        let first = provider.embed(vec!["hello".into()]).await.unwrap();
        let second = provider.embed(vec!["hello".into()]).await.unwrap();

        assert_eq!(first, second);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(provider.cache().len().await, 1);
    }

    #[tokio::test]
    async fn cached_provider_preserves_batch_order() {
        let calls = Arc::new(AtomicUsize::new(0));
        let provider = CachedEmbeddingProvider::new(CountingEmbedding { calls });

        let vectors = provider
            .embed(vec!["a".into(), "abcd".into(), "a".into()])
            .await
            .unwrap();

        assert_eq!(vectors, vec![vec![1.0], vec![4.0], vec![1.0]]);
    }
}
```

- [ ] **Step 4: Verify slice**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-memory cached_provider -- --nocapture
cargo test -p macaca-memory -- --nocapture
```

Expected:

- Cache tests pass.
- Existing memory tests still pass.

## Task 5: Backend Factory Slice

**Files:**

- Create: `macaca/crates/macaca-memory/src/backend.rs`
- Modify: `macaca/crates/macaca-memory/src/lib.rs`

- [ ] **Step 1: Add backend config and factory**

Create `macaca/crates/macaca-memory/src/backend.rs`:

```rust
use std::path::PathBuf;
use std::time::Duration;

use macaca_proto::{AgentId, ApplicationId};

use crate::embedding::MockEmbedding;
use crate::file::FileMemory;
use crate::isolated::IsolatedMemoryManager;
use crate::manager::MemoryManager;
use crate::session::SessionMemory;
use crate::vector::InMemoryVectorStore;

#[derive(Debug, Clone)]
pub struct MemoryBackendConfig {
    pub base_path: PathBuf,
    pub session_ttl: Duration,
    pub enable_vector: bool,
}

impl MemoryBackendConfig {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            base_path,
            session_ttl: Duration::from_secs(60),
            enable_vector: true,
        }
    }

    pub fn session_ttl(mut self, session_ttl: Duration) -> Self {
        self.session_ttl = session_ttl;
        self
    }

    pub fn enable_vector(mut self, enable_vector: bool) -> Self {
        self.enable_vector = enable_vector;
        self
    }
}

pub struct MemoryBackendFactory {
    config: MemoryBackendConfig,
}

impl MemoryBackendFactory {
    pub fn new(config: MemoryBackendConfig) -> Self {
        Self { config }
    }

    pub fn test_manager(&self) -> MemoryManager<InMemoryVectorStore, MockEmbedding> {
        let vector = self.config.enable_vector.then(InMemoryVectorStore::new);
        let embedding = self.config.enable_vector.then(MockEmbedding::default);
        MemoryManager::new(
            SessionMemory::new(self.config.session_ttl),
            FileMemory::new(self.config.base_path.clone()),
            vector,
            embedding,
        )
    }

    pub fn isolated_test_manager(
        &self,
        app_id: ApplicationId,
        agent_id: AgentId,
    ) -> IsolatedMemoryManager<InMemoryVectorStore, MockEmbedding> {
        let vector = self.config.enable_vector.then(InMemoryVectorStore::new);
        let embedding = self.config.enable_vector.then(MockEmbedding::default);
        IsolatedMemoryManager::new(
            app_id,
            agent_id,
            self.config.base_path.clone(),
            self.config.session_ttl,
            vector,
            embedding,
        )
    }
}
```

- [ ] **Step 2: Export backend module**

Modify `macaca/crates/macaca-memory/src/lib.rs`:

```rust
pub mod backend;
pub use backend::{MemoryBackendConfig, MemoryBackendFactory};
```

- [ ] **Step 3: Add factory tests**

Add tests to `backend.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn factory_builds_standard_test_manager() {
        let dir = TempDir::new().unwrap();
        let factory = MemoryBackendFactory::new(MemoryBackendConfig::new(dir.path().to_path_buf()));
        let manager = factory.test_manager();

        let id = manager
            .remember_text(crate::facade::RememberText::new("factory memory"))
            .await
            .unwrap();
        let result = manager
            .recall(crate::facade::RecallQuery::new("factory", 10))
            .await
            .unwrap();

        assert!(result.entries.iter().any(|entry| entry.id == id));
    }

    #[tokio::test]
    async fn factory_builds_isolated_test_manager() {
        let dir = TempDir::new().unwrap();
        let factory = MemoryBackendFactory::new(MemoryBackendConfig::new(dir.path().to_path_buf()));
        let app_id = ApplicationId::new();
        let agent_id = AgentId::new();
        let manager = factory.isolated_test_manager(app_id, agent_id);

        assert_eq!(manager.app_id(), app_id);
        assert_eq!(manager.agent_id(), agent_id);
    }
}
```

- [ ] **Step 4: Verify slice**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-memory factory -- --nocapture
cargo test -p macaca-memory -- --nocapture
```

Expected:

- Factory tests pass.
- Existing behavior remains unchanged.

## Task 6: Snapshot Memento Slice

**Files:**

- Create: `macaca/crates/macaca-memory/src/snapshot.rs`
- Modify: `macaca/crates/macaca-memory/src/lib.rs`
- Modify: `macaca/crates/macaca-memory/src/session.rs`
- Modify: `macaca/crates/macaca-memory/src/file.rs`

- [ ] **Step 1: Add snapshot schema and trait**

Create `macaca/crates/macaca-memory/src/snapshot.rs`:

```rust
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use macaca_proto::{MacacaResult, MemoryEntry};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    pub captured_at: DateTime<Utc>,
    pub entries: Vec<MemoryEntry>,
}

impl MemorySnapshot {
    pub fn new(entries: Vec<MemoryEntry>) -> Self {
        Self {
            captured_at: Utc::now(),
            entries,
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[async_trait]
pub trait MemorySnapshotStore {
    async fn snapshot(&self, limit: usize) -> MacacaResult<MemorySnapshot>;
    async fn replay_snapshot(&self, snapshot: &MemorySnapshot) -> MacacaResult<()>;
}
```

- [ ] **Step 2: Export snapshot module**

Modify `macaca/crates/macaca-memory/src/lib.rs`:

```rust
pub mod snapshot;
pub use snapshot::{MemorySnapshot, MemorySnapshotStore};
```

- [ ] **Step 3: Implement snapshot for session store**

Add to `session.rs`:

```rust
#[async_trait]
impl crate::snapshot::MemorySnapshotStore for SessionMemory {
    async fn snapshot(&self, limit: usize) -> MacacaResult<crate::snapshot::MemorySnapshot> {
        let entries = self.list(None, limit).await?;
        Ok(crate::snapshot::MemorySnapshot::new(entries))
    }

    async fn replay_snapshot(&self, snapshot: &crate::snapshot::MemorySnapshot) -> MacacaResult<()> {
        for entry in snapshot.entries.iter().cloned() {
            self.store(entry).await?;
        }
        Ok(())
    }
}
```

- [ ] **Step 4: Implement snapshot for file store**

Add to `file.rs`:

```rust
#[async_trait]
impl crate::snapshot::MemorySnapshotStore for FileMemory {
    async fn snapshot(&self, limit: usize) -> MacacaResult<crate::snapshot::MemorySnapshot> {
        let entries = self.list(None, limit).await?;
        Ok(crate::snapshot::MemorySnapshot::new(entries))
    }

    async fn replay_snapshot(&self, snapshot: &crate::snapshot::MemorySnapshot) -> MacacaResult<()> {
        for entry in snapshot.entries.iter().cloned() {
            self.store(entry).await?;
        }
        Ok(())
    }
}
```

- [ ] **Step 5: Add snapshot tests**

Add tests to `snapshot.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::FileMemory;
    use crate::store::MemoryStore;
    use chrono::Utc;
    use macaca_proto::{MemoryId, MemoryLayer};
    use tempfile::TempDir;

    fn entry(content: &str) -> macaca_proto::MemoryEntry {
        macaca_proto::MemoryEntry {
            id: MemoryId::new(),
            layer: MemoryLayer::File,
            content: content.to_string(),
            metadata: serde_json::Value::Null,
            agent_id: None,
            created_at: Utc::now(),
            expires_at: None,
        }
    }

    #[tokio::test]
    async fn file_snapshot_replays_entries() {
        let src_dir = TempDir::new().unwrap();
        let dst_dir = TempDir::new().unwrap();
        let src = FileMemory::new(src_dir.path().to_path_buf());
        let dst = FileMemory::new(dst_dir.path().to_path_buf());
        let first = entry("snapshot one");
        let second = entry("snapshot two");

        src.store(first.clone()).await.unwrap();
        src.store(second.clone()).await.unwrap();

        let snapshot = src.snapshot(10).await.unwrap();
        dst.replay_snapshot(&snapshot).await.unwrap();
        let restored = dst.list(None, 10).await.unwrap();

        assert_eq!(snapshot.len(), 2);
        assert!(restored.iter().any(|entry| entry.id == first.id));
        assert!(restored.iter().any(|entry| entry.id == second.id));
    }
}
```

- [ ] **Step 6: Verify slice**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-memory snapshot -- --nocapture
cargo test -p macaca-memory -- --nocapture
```

Expected:

- Snapshot tests pass.
- Existing behavior remains unchanged.

## Task 7: Vector Query Strategy Slice

**Files:**

- Create: `macaca/crates/macaca-memory/src/query.rs`
- Modify: `macaca/crates/macaca-memory/src/lib.rs`
- Modify: `macaca/crates/macaca-memory/src/vector.rs`

- [ ] **Step 1: Add query strategy primitives**

Create `macaca/crates/macaca-memory/src/query.rs`:

```rust
use async_trait::async_trait;
use serde_json::Value;

use macaca_proto::MacacaResult;

use crate::store::{VectorSearchResult, VectorStore};

#[derive(Debug, Clone)]
pub struct VectorQuery {
    pub vector: Vec<f32>,
    pub limit: usize,
    pub metadata_equals: Vec<(String, Value)>,
}

impl VectorQuery {
    pub fn new(vector: Vec<f32>, limit: usize) -> Self {
        Self {
            vector,
            limit,
            metadata_equals: Vec::new(),
        }
    }

    pub fn with_metadata_eq(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata_equals.push((key.into(), value));
        self
    }
}

#[async_trait]
pub trait VectorQueryStrategy<S: VectorStore>: Send + Sync {
    async fn search(&self, store: &S, query: VectorQuery) -> MacacaResult<Vec<VectorSearchResult>>;
}

pub struct SimilarityVectorQueryStrategy;

#[async_trait]
impl<S: VectorStore> VectorQueryStrategy<S> for SimilarityVectorQueryStrategy {
    async fn search(&self, store: &S, query: VectorQuery) -> MacacaResult<Vec<VectorSearchResult>> {
        let mut results = store.search(query.vector, query.limit).await?;
        if !query.metadata_equals.is_empty() {
            results.retain(|hit| {
                query
                    .metadata_equals
                    .iter()
                    .all(|(key, value)| hit.payload.get(key) == Some(value))
            });
        }
        Ok(results)
    }
}
```

- [ ] **Step 2: Export query module**

Modify `macaca/crates/macaca-memory/src/lib.rs`:

```rust
pub mod query;
pub use query::{SimilarityVectorQueryStrategy, VectorQuery, VectorQueryStrategy};
```

- [ ] **Step 3: Add vector strategy tests**

Add tests to `vector.rs` or `query.rs`:

```rust
#[cfg(test)]
mod strategy_tests {
    use super::*;
    use crate::query::{SimilarityVectorQueryStrategy, VectorQuery, VectorQueryStrategy};

    #[tokio::test]
    async fn default_strategy_matches_similarity_ordering() {
        let store = InMemoryVectorStore::new();
        store
            .upsert("a", vec![1.0, 0.0], serde_json::json!({"kind": "doc"}))
            .await
            .unwrap();
        store
            .upsert("b", vec![0.0, 1.0], serde_json::json!({"kind": "doc"}))
            .await
            .unwrap();

        let strategy = SimilarityVectorQueryStrategy;
        let results = strategy
            .search(&store, VectorQuery::new(vec![1.0, 0.0], 2))
            .await
            .unwrap();

        assert_eq!(results[0].id, "a");
    }

    #[tokio::test]
    async fn strategy_filters_by_metadata() {
        let store = InMemoryVectorStore::new();
        store
            .upsert("a", vec![1.0, 0.0], serde_json::json!({"kind": "doc"}))
            .await
            .unwrap();
        store
            .upsert("b", vec![1.0, 0.0], serde_json::json!({"kind": "note"}))
            .await
            .unwrap();

        let strategy = SimilarityVectorQueryStrategy;
        let results = strategy
            .search(
                &store,
                VectorQuery::new(vec![1.0, 0.0], 10)
                    .with_metadata_eq("kind", serde_json::json!("note")),
            )
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "b");
    }
}
```

- [ ] **Step 4: Verify slice**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-memory strategy -- --nocapture
cargo test -p macaca-memory -- --nocapture
```

Expected:

- Strategy tests pass.
- Existing vector store behavior remains unchanged.

## Task 8: Final Verification

**Files:**

- No new source edits.

- [ ] **Step 1: Format**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo fmt
```

Expected:

- Command exits with status 0.

- [ ] **Step 2: Run full memory tests**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-memory -- --nocapture
```

Expected:

- All non-ignored tests pass.
- Live DashScope and Milvus tests remain ignored unless explicitly enabled.

- [ ] **Step 3: Run dependent crate check**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo check -p macaca-memory -p macaca-kernel -p macaca-agent -p macaca-framework -p macaca-web
```

Expected:

- Command exits with status 0.
- Existing unrelated warnings may remain.

- [ ] **Step 4: Validate OpenSpec**

Run:

```bash
cd /Users/quantum/Code/dev/agent
openspec validate refactor-macaca-memory-patterns --strict
```

Expected:

```text
Change 'refactor-macaca-memory-patterns' is valid
```

- [ ] **Step 5: Run GitNexus detect changes**

Run GitNexus:

```text
gitnexus_detect_changes({
  "repo": "agent",
  "scope": "all"
})
```

Expected:

- Changed symbols are limited to `macaca-memory`, OpenSpec files, and the plan document.
- If `macaca-kernel`, `macaca-web`, `macaca-agent`, or `macaca-framework` behavior paths appear unexpectedly, inspect before committing.

## Task 9: Commit

**Files:**

- Commit only after Task 8 succeeds.

- [ ] **Step 1: Review diff**

Run:

```bash
cd /Users/quantum/Code/dev/agent
git diff --stat
git diff -- macaca/crates/macaca-memory openspec/changes/refactor-macaca-memory-patterns docs/superpowers/plans/2026-05-03-refactor-macaca-memory.md | sed -n '1,260p'
```

Expected:

- Diff contains additive `macaca-memory` primitives, tests, OpenSpec files, and this plan.
- No unrelated application-specific changes.

- [ ] **Step 2: Commit**

Run:

```bash
cd /Users/quantum/Code/dev/agent
git add macaca/crates/macaca-memory openspec/changes/refactor-macaca-memory-patterns docs/superpowers/plans/2026-05-03-refactor-macaca-memory.md
git commit -m "refactor: introduce macaca-memory pattern primitives"
```

Expected:

- Commit succeeds.
- If GitNexus reports stale index after commit, run `npx gitnexus analyze`.

## Self-Review

- Spec coverage: The plan covers all five slices from `macaca/docs/design-pattern-refactor-plans/macaca-memory.md`.
- Placeholder scan: No intentionally blank implementation steps are left; every code-edit task includes concrete file paths and code shape.
- Scope control: Upper-crate consumer migration is excluded from this refactor and should be a separate follow-up after the primitives are stable.
- Risk control: GitNexus impact is required before editing `MemoryManager` and `IsolatedMemoryManager`, and `detect_changes` is required before commit.
