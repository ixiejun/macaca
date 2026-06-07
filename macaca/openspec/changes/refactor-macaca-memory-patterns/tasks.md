## 1. Preparation

- [x] 1.1 Run GitNexus impact for `MemoryManager` upstream.
- [x] 1.2 Run GitNexus impact for `IsolatedMemoryManager` upstream.
- [x] 1.3 Run baseline `cargo test -p macaca-memory -- --nocapture`.

## 2. Facade slice

- [x] 2.1 Add facade request/result types.
- [x] 2.2 Add manager facade methods.
- [x] 2.3 Add isolated manager facade methods.
- [x] 2.4 Add tests proving facade methods call existing behavior.

## 3. Embedding cache slice

- [x] 3.1 Add `EmbeddingCache`.
- [x] 3.2 Add `CachedEmbeddingProvider<E>`.
- [x] 3.3 Add tests for cache hits, misses, and batch behavior.

## 4. Backend factory slice

- [x] 4.1 Add backend config types.
- [x] 4.2 Add factory methods for standard managers.
- [x] 4.3 Add tests for deterministic construction.

## 5. Snapshot memento slice

- [x] 5.1 Add snapshot schema.
- [x] 5.2 Add snapshot/replay helpers for session and file stores.
- [x] 5.3 Add tests for snapshot round-trip.

## 6. Vector query strategy slice

- [x] 6.1 Add vector query strategy traits and request types.
- [x] 6.2 Add default similarity strategy.
- [x] 6.3 Add metadata-filter strategy tests.

## 7. Deprecated compatibility markers

- [x] 7.1 Mark superseded `MemoryManager` direct methods deprecated while retaining private non-deprecated implementation helpers.
- [x] 7.2 Mark superseded `IsolatedMemoryManager` direct methods deprecated while retaining private non-deprecated implementation helpers.
- [x] 7.3 Confirm low-level storage/provider/vector traits are not deprecated.

## 8. Verification

- [x] 8.1 Run `cargo fmt`.
- [x] 8.2 Run `cargo test -p macaca-memory -- --nocapture`.
- [x] 8.3 Run `cargo check -p macaca-memory -p macaca-kernel -p macaca-agent -p macaca-framework -p macaca-web`.
- [x] 8.4 Run `openspec validate refactor-macaca-memory-patterns --strict`.
- [x] 8.5 Run `gitnexus_detect_changes(scope: "all")`.
