# Change: Refactor macaca-kernel with design pattern primitives

## Why

`macaca-kernel` is the Agent OS coordination center. Executor event construction, scheduler selection, agent status transitions, executor payload construction, and kernel construction are currently scattered across kernel and web call sites, which makes trace/EventLog correctness and future scheduler/executor extensions harder to maintain.

This change performs all five planned kernel refactor slices in one proposal while keeping each implementation step small and behavior-compatible.

## What Changes

- Add canonical executor lifecycle event/result factory helpers.
- Add scheduler factory primitives while preserving `SimpleScheduler` behavior.
- Add explicit agent status transition policy helpers.
- Move executor payload construction toward kernel-owned primitives.
- Add an additive kernel builder/facade entry while keeping `Kernel::new` callable but deprecated.
- Mark replaced legacy direct-construction interfaces deprecated, without deleting them, so later consumer migrations can find and remove old usage.

## Impact

- Affected specs: `macaca-kernel-patterns`
- Affected code: `macaca-kernel`, `macaca-web` loop manager helper usage
- Non-impact: no application-specific logic, no scheduler behavior change, no `ExecutorEvent` payload shape change, no EventLog/SSE behavior change.

## Non-Goals

- Do not split `ApplicationExecutor` or `ForkManager` into many files in this change.
- Do not remove deprecated interfaces.
- Do not migrate all `Kernel::new` consumers in this change.
- Do not change worker supervision, fork resume, queue semantics, SSE, EventLog, session restore, or web trace behavior.
- Do not hardcode application, workflow, driver, or agent names.
