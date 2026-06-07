# Design

## Context

The runtime template refactor added non-deprecated execution entrypoints while keeping old `run*` methods deprecated and callable. Direct Cargo consumers are limited to `macaca-web` and `macaca-integration-tests`.

`macaca-integration-tests` already calls `execute_with_events`. `macaca-web` does not execute `AgenticLoop`, but it still imports `macaca_runtime::agentic_loop::ResumeReason` as an internal coordinator resume DTO.

## Goals

- Ensure upper crates do not call deprecated runtime execution APIs.
- Keep integration dry-run on the evented template entrypoint.
- Isolate web resume signaling behind a local generic adapter.
- Preserve existing session, SSE, goal completion, and delegate completion behavior.

## Non-Goals

- Do not change runtime internals.
- Do not remove or relocate `ResumeReason`.
- Do not make web execution depend on `PausableAgenticLoop`.
- Do not introduce app/workflow-specific resume types.

## Pattern Mapping

- Facade: upper execution consumers use `AgenticLoop::execute*` facade methods.
- Adapter: `macaca-web` owns a local resume signal type that adapts prior runtime resume semantics to web/framework needs.
- Observer: integration dry-run continues to exercise evented runtime execution.

## Decisions

- Add `macaca-web::runtime_resume::RuntimeResumeSignal`.
- Use `RuntimeResumeSignal` for web active session channels and framework middleware.
- Do not add conversion back to `ResumeReason` unless code still calls `PausableAgenticLoop`; current web path does not.
- Update stale integration-test comments from `run_with_events` to `execute_with_events`.
- Allow deprecated runtime methods only inside `macaca-runtime` compatibility wrappers.

## Risks / Trade-offs

- Web resume path touches active sessions, hook events, goal completion, and middleware.
- Mitigation: change only the DTO type and variant names; keep channel sizes, pause signal writes, SSE behavior, and appended completion text unchanged.

- Runtime `ResumeReason` remains public and may still be imported by external users.
- Mitigation: this change targets repository consumers; compatibility remains available for external migration.

## Migration Plan

1. Create and validate OpenSpec change.
2. Run impact analysis for web and integration symbols.
3. Add web-local resume signal type.
4. Replace web imports/channel types/constructors/matches with local type.
5. Update integration dry-run documentation reference.
6. Run grep, tests, check, OpenSpec, and GitNexus verification.
