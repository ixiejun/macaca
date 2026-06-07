# Design

## Context

`macaca-runtime` is the generic agentic loop layer. It depends on `macaca-llm`, `macaca-proto`, and `macaca-tools`, and must remain application-agnostic. The current `AgenticLoop` implementation is behaviorally useful but structurally overloaded.

## Goals

- Keep runtime behavior 1:1 compatible.
- Make the fixed loop skeleton visible through template primitives.
- Keep `AgenticLoop` as the compatibility facade.
- Provide new non-deprecated execution entrypoints before deprecating old ones.
- Keep every new source file under 500 lines.

## Non-Goals

- No new provider, model, workflow, app, or driver-specific logic.
- No permission semantics rewrite in this slice.
- No public pause/resume type migration in this slice.
- No new third-party dependencies.

## Pattern Mapping

- Template Method: expose named execution entrypoints and move repeated loop setup/stop handling behind a shared template.
- Facade: keep `AgenticLoop` and `PausableAgenticLoop` as stable compatibility facades.
- Observer: wrap optional `AgentExecutionEvent` senders in a runtime event sink.
- Command: wrap tool calls as runtime tool commands with timeout and trace forwarding.
- Future Strategy: context compaction and loop detection stay behavior-compatible now and can become strategies later.
- Future Chain of Responsibility: `DefaultPermissionChecker` remains intact now and can delegate to a rule chain later.

## Decisions

- Add non-deprecated methods named `execute`, `execute_with_events`, and `execute_with_pause`.
- Mark `run`, `run_with_events`, and `run_with_pause` as deprecated wrappers that delegate to the new methods.
- Keep `ResumeReason` where it is to avoid forcing `macaca-web` migration during this slice.
- Extract event emission and tool execution helpers into small modules without changing public behavior.
- Keep deprecated wrappers callable for migration searches and backward compatibility.

## Risks / Trade-offs

- Deprecating existing methods will surface warnings in unmigrated callers.
- Mitigation: migrate direct repository consumers in the same change and keep wrappers available.

- The loop is a critical path and small ordering changes can be regressions.
- Mitigation: preserve current tests and add deterministic event ordering coverage.

- Public trait expansion can create long-term compatibility debt.
- Mitigation: keep first-slice primitives minimal and avoid public strategy traits unless needed.

## Migration Plan

1. Add OpenSpec artifacts and validate them.
2. Add internal template/event/execution modules.
3. Add new non-deprecated entrypoints on `AgenticLoop` and `PausableAgenticLoop`.
4. Mark old direct entrypoints deprecated and keep them delegating.
5. Migrate direct repository callers to new entrypoints.
6. Run runtime, integration, OpenSpec, grep, and GitNexus checks.
