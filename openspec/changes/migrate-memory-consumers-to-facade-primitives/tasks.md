## 1. OpenSpec

- [x] 1.1 Create proposal, design, task list, and delta spec.
- [x] 1.2 Validate with `openspec validate migrate-memory-consumers-to-facade-primitives --strict`.

## 2. Impact and Baseline

- [x] 2.1 Run GitNexus impact for `MemoryService`.
- [x] 2.2 Run GitNexus impact for `MemoryServiceAdapter`.
- [x] 2.3 Run baseline `cargo check -p macaca-agent -p macaca-kernel`.

## 3. macaca-agent Migration

- [x] 3.1 Add `macaca-memory` dependency to `macaca-agent`.
- [x] 3.2 Add facade-first `remember_text` and `recall` methods to `MemoryService`.
- [x] 3.3 Mark `MemoryService::store` and `MemoryService::retrieve` deprecated while keeping compatibility wrappers callable.
- [x] 3.4 Update `NoopMemoryService` and service tests to use facade methods.
- [x] 3.5 Add compatibility test proving deprecated methods still work under local `#[allow(deprecated)]`.

## 4. macaca-kernel Migration

- [x] 4.1 Migrate `MemoryServiceAdapter` to adapt facade-capable `MemoryManager` and `IsolatedMemoryManager`.
- [x] 4.2 Update adapter tests to construct managers via `MemoryBackendFactory`.
- [x] 4.3 Update adapter tests to call `remember_text` and `recall`.

## 5. Verification

- [x] 5.1 Run `cargo fmt`.
- [x] 5.2 Run `cargo test -p macaca-agent services -- --nocapture`.
- [x] 5.3 Run `cargo test -p macaca-kernel services -- --nocapture`.
- [x] 5.4 Run `cargo check -p macaca-memory -p macaca-agent -p macaca-kernel -p macaca-web`.
- [x] 5.5 Verify migrated production code no longer calls deprecated memory service APIs.
- [x] 5.6 Run `openspec validate migrate-memory-consumers-to-facade-primitives --strict`.
- [x] 5.7 Run `gitnexus_detect_changes(scope: "all")`.
