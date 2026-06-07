# Design: Route C baseline governance

## Context

The Route C plan moves Macaca toward a microkernel + system services + WASM Application ABI + Store + optional Web3/EVM architecture. This is a long-running refactor with high regression risk. Phase 0 must create a governance and verification baseline before any later architecture changes.

## Goals

- Define which capabilities belong in kernel, system services, plugins, optional modules, application framework, and presentation shells.
- Make existing user-visible behavior explicit as regression scenarios.
- Provide a mandatory implementation template for later Route C phases.
- Add a no-network automated baseline so contributors can verify the current autonomous execution substrate without external LLMs.

## Non-Goals

- Do not implement Route C primitives.
- Do not introduce new runtime behavior.
- Do not implement WASM, GenUI, Store, entitlement, Web3, or EVM.
- Do not migrate `macaca-web` or `macaca-kernel` internals.

## Decisions

- Use documentation as the governance source for Phase 0. This phase defines rules and gates, not runtime code.
- Add an integration test that runs the existing no-network pipeline dry run and checks governance docs cover required baseline topics.
- Keep the baseline test independent from live services, real LLM providers, browsers, and front-end servers.

## Risks / Trade-offs

- Documentation-only governance can drift. Mitigation: add tests that assert required governance sections and scenario names remain present.
- The baseline dry run does not cover every future Web UI behavior. Mitigation: the regression matrix names Web UI manual/automated scenarios that later phases must automate incrementally.
- Adding too much test scope in Phase 0 could make every refactor slow. Mitigation: use one fast no-network pipeline test plus documentation coverage checks.

## Migration Plan

1. Add governance documents.
2. Link them from canonical overview and refactor order.
3. Add `route_c_baseline.rs` integration test.
4. Validate OpenSpec and run targeted tests.

