# Change: Migrate macaca-runtime consumers to template primitives

## Why

`macaca-runtime` now exposes non-deprecated template execution entrypoints (`execute`, `execute_with_events`, and `execute_with_pause`) while retaining legacy `run*` methods as deprecated compatibility wrappers. Upper consumers should use the new entrypoints and should not couple unrelated web resume messaging to `agentic_loop::ResumeReason`.

## What Changes

- Require upper consumers to avoid deprecated runtime execution APIs.
- Keep integration dry-run coverage on `AgenticLoop::execute_with_events`.
- Add a web-local, generic resume signal adapter for coordinator/goal resume messages.
- Migrate `macaca-web` active session, hook consumer, framework middleware, and goal completion paths away from direct `macaca_runtime::agentic_loop::ResumeReason` imports.
- Keep runtime deprecated wrappers and `ResumeReason` public for external compatibility.

## Non-Goals

- Do not remove deprecated runtime APIs.
- Do not move or rename `macaca-runtime::agentic_loop::ResumeReason`.
- Do not reintroduce `PausableAgenticLoop` into the web framework runner execution path.
- Do not change goal/delegate resume semantics or SSE/session behavior.
- Do not add application, workflow, provider, or driver-specific logic.

## Impact

- Affected specs: `macaca-runtime-consumers`
- Affected code: `macaca-web`, `macaca-integration-tests`
- Compatibility: `macaca-runtime` compatibility APIs remain callable.
