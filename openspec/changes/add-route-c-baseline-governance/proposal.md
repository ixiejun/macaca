# Change: Add Route C baseline governance

## Why

Macaca OS Route C introduces a multi-phase microkernel ecosystem refactor. Before implementation starts, the project needs a durable baseline that defines kernel/service/plugin/optional-module boundaries and protects existing Agent OS behavior from regression.

## What Changes

- Add governance documentation for Route C microkernel boundaries.
- Add a regression matrix covering existing session, task, trace, driver, skill/MCP, and recovery behavior.
- Add a reusable phase implementation template for later Route C phases.
- Add an integration baseline test that keeps the no-network pipeline dry run and governance coverage visible.
- Link the canonical system overview and refactor order to the Route C governance documents.

## Impact

- Affected specs: `route-c-baseline-governance`
- Affected docs: `macaca/docs/agent-os-microkernel-boundaries.md`, `macaca/docs/route-c-regression-matrix.md`, `macaca/docs/route-c-phase-template.md`, `macaca/docs/route-c-architecture-governance.md`, `macaca/docs/SYSTEM_OVERVIEW.md`, `macaca/docs/design-pattern-refactor-plans/refactor-order.md`
- Affected tests: `macaca/crates/macaca-integration-tests/tests/route_c_baseline.rs`

