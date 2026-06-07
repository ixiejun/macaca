# Change: Migrate kernel consumers to KernelBuilder

## Why

`macaca-kernel` now exposes design-pattern primitives such as `KernelBuilder`, `SchedulerFactory`, and `ExecutorEventFactory`. Upper crates should stop constructing kernels through deprecated compatibility APIs so future kernel refactors can rely on one canonical construction path.

## What Changes

- Replace upper-crate production `Kernel::new` calls with `KernelBuilder`.
- Replace regular upper-crate test helpers with `KernelBuilder`.
- Migrate direct `SimpleScheduler` compatibility test usage to `SchedulerFactory`.
- Keep deprecated `Kernel::new` and `SimpleScheduler` definitions callable but unused by upper consumers.
- Add verification that upper production crates do not call deprecated kernel construction APIs.

## Impact

- Affected specs: `macaca-kernel-consumer-migration`
- Affected code: `macaca-web`, `macaca-cli`, `macaca-app`, `macaca-sdk`, `macaca-integration-tests`, selected `macaca-kernel` tests
- Non-impact: no runtime behavior change; no scheduler behavior change; no trace, EventLog, SSE, planner, worker, coordinator, driver, skill, or MCP behavior changes.
