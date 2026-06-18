## 1. Immediate Actions

- [x] 1.1 Move `macaca-web` task decomposition keyword logic behind a service-owned task/autonomy decomposition command and Strategy implementation.
- [x] 1.2 Add SDK/SystemFacade or focused client access for the decomposition command so Web remains a thin adapter.
- [x] 1.3 Preserve current Web response shapes and task-board behavior with targeted regression tests.
- [x] 1.4 Split `serviceization_escape_hatches.rs` into smaller policy-specific boundary gates with shared support fixtures.
- [x] 1.5 Add a 450-line advisory production Rust source-size gate while keeping the 500-line hard failure gate.
- [x] 1.6 Add a shell semantic ownership gate rejecting task/planning/decomposition semantics in Web/CLI unless the code only delegates through SDK/facade/service clients.
- [x] 1.7 Update governance docs to describe the decomposition ownership transfer, advisory size gate, and shell semantic gate.

## 2. Short-Term Actions

- [x] 2.1 Identify near-limit runtime-host provider modules and split the highest-risk files by descriptor, command DTO handling, state, handler, adapter, and tests.
- [x] 2.2 Add or preserve module-level English comments explaining ownership, design pattern intent, runtime behavior, trace/audit behavior, and non-goals.
- [x] 2.3 Add request-local `HashSet`/`HashMap` indexes for repeated membership scans in capability catalog, route projection, task dependency selection, and skill mappings where the path is request/event hot.
- [x] 2.4 Add deterministic tests proving indexed paths preserve ordering, missing-record behavior, authorization scope, and existing result shapes.
- [x] 2.5 Document lifecycle, reset/test-isolation behavior, and restart semantics for existing `OnceLock` or static registry/lock modules.
- [x] 2.6 Replace static state with explicit composition-root state where the impact analysis shows low risk and behavior can be preserved.
- [x] 2.7 Move repeated integration-test fixtures into small support modules without weakening boundary assertions.

## 3. Long-Term Actions

- [x] 3.1 Define an extraction-readiness checklist for runtime-host provider families before any provider family can become a dedicated service crate.
- [x] 3.2 Apply the extraction-readiness checklist to at least one mature provider family and record the decision as documentation or an implementation memo.
- [x] 3.3 Split dense `macaca-proto` DTO modules by command family while preserving public type names, serde compatibility, and re-export stability.
- [x] 3.4 Add serde and compatibility tests for split protocol modules.
- [x] 3.5 Replace remaining text/name-based routing with typed capability descriptors, declarative mapping records, and audited fallback policies.
- [x] 3.6 Add an architecture-smell CI/reporting lane that emits non-failing complexity/coupling/file-size-headroom trend diagnostics.
- [x] 3.7 Ensure smell trend diagnostics are deterministic, sanitized, and linked to rule identifiers.
- [x] 3.8 Re-run smell analysis and update `tasks/smell-report-*` or create a follow-up report documenting residual risks.

## 4. Verification

- [x] 4.1 Run `openspec validate refactor-architecture-smell-roadmap --strict`.
- [x] 4.2 Run targeted tests for task/autonomy decomposition, Web adapter behavior, and SDK/facade client behavior.
- [x] 4.3 Run serviceization and dependency boundary gates.
- [x] 4.4 Run file-size and shell semantic ownership gates.
- [x] 4.5 Run targeted crate checks for touched crates.
- [x] 4.6 Run `cargo check --workspace` after shared protocol or cross-crate boundary changes.
- [x] 4.7 Run GitNexus impact analysis before symbol edits and `gitnexus_detect_changes()` before committing; HIGH/CRITICAL findings are recorded as memo per this change's instruction.
