## Context

The previous `refactor-macaca-kernel-patterns` change introduced `KernelBuilder`, `SchedulerFactory`, `ExecutorEventFactory`, and status transition primitives. `Kernel::new` and `SimpleScheduler` remain callable but deprecated.

Upper crates still call `Kernel::new` directly. Kernel scheduler tests also instantiate `SimpleScheduler` directly. These call sites keep consumers coupled to compatibility entries and make deprecation warnings harder to use as migration signals.

## Goals

- Keep behavior 1:1 compatible.
- Make `KernelBuilder` the canonical upper-crate construction entry.
- Make `SchedulerFactory` the canonical scheduler construction entry in tests.
- Keep deprecated kernel APIs callable for migration-period compatibility.
- Prevent new upper production usage of deprecated kernel construction APIs.

## Non-Goals

- Do not remove `Kernel::new`.
- Do not remove `SimpleScheduler`.
- Do not change `SchedulerFactory` behavior.
- Do not change `ExecutorEvent` or `TaskResult` payloads.
- Do not change web session, trace, EventLog, SSE, task board, planner, worker, coordinator, driver, skill, or MCP behavior.
- Do not introduce app-specific or workflow-specific code.

## Decisions

- Use `KernelBuilder::new(config.clone(), llm, tools).build()` when the caller owns only `&KernelConfig`.
- Use `KernelBuilder::new(config, llm, tools).build()` when the caller owns the config.
- Add a small helper in `macaca-cli` to avoid repeating builder construction across commands.
- Use `SchedulerFactory::build(SchedulerKind::Simple)` in scheduler tests instead of direct `SimpleScheduler` construction.
- Treat deprecated definitions and the `SchedulerFactory` internal bridge as valid remaining locations. All existing consumer call sites should migrate.

## Risks / Mitigations

- Risk: web startup could fail if kernel construction semantics change.
  Mitigation: replace only the construction entry; preserve LLM provider, toolset, app registry, runtime, session, trace, and routes.
- Risk: CLI command behavior could drift if each command constructs kernel differently.
  Mitigation: route all CLI command construction through one local helper.
- Risk: scheduler test migration could accidentally test a different strategy.
  Mitigation: pass `SchedulerKind::Simple` explicitly so the selected behavior remains equivalent to direct `SimpleScheduler`.
- Risk: grep audits can confuse deprecated definitions with calls.
  Mitigation: verify production upper crates separately from all-crate audits.
