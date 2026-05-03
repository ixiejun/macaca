# Change: Migrate macaca-agent consumers to primitive boundaries

## Why

`macaca-agent` now exposes additive service, capability, and lifecycle primitives. Upper crates should stop calling deprecated construction helpers so future refactors can rely on one canonical primitive surface.

## What Changes

- Replace upper-crate `AgentServices::empty()` calls with `AgentServices::builder().build()`.
- Require upper crates to consume `macaca-agent` primitives through additive public entries instead of deprecated compatibility helpers.
- Add verification that deprecated direct construction helpers are not used outside `macaca-agent`.

## Impact

- Affected specs: `macaca-agent-consumer-migration`
- Affected code: `macaca-kernel`, `macaca-sdk`
- Non-impact: no runtime behavior change; no trace, task, planner, worker, coordinator, driver, skill, or MCP behavior changes.
