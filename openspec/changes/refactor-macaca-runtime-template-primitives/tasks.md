## 1. OpenSpec

- [x] 1.1 Create proposal, design, tasks, and delta spec.
- [x] 1.2 Validate `refactor-macaca-runtime-template-primitives` with `openspec validate --strict`.

## 2. Baseline and Impact

- [x] 2.1 Review current runtime code and direct consumers.
- [x] 2.2 Run GitNexus impact for runtime symbols before editing.
- [x] 2.3 Confirm affected callers and risk level.

## 3. Runtime Template Primitives

- [x] 3.1 Add template primitives for iteration outcome and stop handling.
- [x] 3.2 Add event sink wrapper for runtime event emission.
- [x] 3.3 Add tool execution command boundary.
- [x] 3.4 Keep all new runtime files under 500 lines.

## 4. Compatibility and Deprecation

- [x] 4.1 Add non-deprecated `AgenticLoop::execute`.
- [x] 4.2 Add non-deprecated `AgenticLoop::execute_with_events`.
- [x] 4.3 Add non-deprecated `PausableAgenticLoop::execute_with_pause`.
- [x] 4.4 Mark `run`, `run_with_events`, and `run_with_pause` deprecated without deleting them.
- [x] 4.5 Migrate direct repository consumers away from deprecated runtime methods.

## 5. Verification

- [x] 5.1 Run `cargo fmt`.
- [x] 5.2 Run `cargo test -p macaca-runtime -- --nocapture`.
- [x] 5.3 Run `cargo test -p macaca-integration-tests pipeline_dry_run -- --nocapture`.
- [x] 5.4 Run `cargo check -p macaca-runtime -p macaca-web -p macaca-integration-tests`.
- [x] 5.5 Run deprecated runtime usage grep for direct consumers.
- [x] 5.6 Run `openspec validate refactor-macaca-runtime-template-primitives --strict`.
- [x] 5.7 Run `npx gitnexus detect-changes --repo agent --scope all`.
