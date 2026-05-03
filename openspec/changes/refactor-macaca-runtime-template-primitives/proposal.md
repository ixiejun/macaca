# Refactor macaca-runtime with Template Primitives

## Why

`macaca-runtime` owns the core agentic execution loop, but `agentic_loop.rs` currently mixes loop orchestration, LLM calls, tool execution, event emission, pause/resume handling, and tests in one oversized file. This makes the runtime hard to extend safely and violates the project file-size guideline.

## What Changes

- Add runtime template primitives that make loop execution stages explicit.
- Add non-deprecated runtime facade methods for standard, evented, and pausable execution.
- Extract tool command execution into a dedicated command boundary.
- Extract runtime event emission behind an observer-style sink wrapper.
- Keep existing public runtime types callable for migration.
- Mark old direct `run`, `run_with_events`, and `run_with_pause` interfaces deprecated after replacements exist, without deleting them.

## Non-Goals

- Do not change LLM request/response behavior.
- Do not change tool execution semantics or error-as-tool-result behavior.
- Do not replace `PermissionChecker` with a permission chain in this slice.
- Do not move or rename `ResumeReason`.
- Do not migrate `macaca-web` framework runner behavior.
- Do not add new dependencies.

## Impact

- Affected specs: `runtime-template-primitives`
- Affected code: `macaca-runtime`, direct integration-test consumers of `AgenticLoop`
