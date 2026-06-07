# Change: Complete Web / CLI Thin Shell v1

## Why

Route C S12 must finish the transition that `add-web-cli-thin-shell-v0` started. Web and CLI already have a first SDK facade and command-adapter pattern, but Web still owns service provider registration/startup for many capabilities, CLI still depends on Web internals for server startup, and deprecated Web provider/runtime fields remain easy to reuse as normal development paths.

This change implements the S12 completion plan from `docs/superpowers/plans/2026-05-10-s12-web-cli-thin-shell-completion-plan.md` while preserving existing `/api/chat/v2`, SSE trace, session replay, task board, application startup, Driver/Skill/MCP, Memory/Context, Store/Entitlement, Payment/A2A, Web3, and EVM behavior.

## What Changes

- Add a runtime-host-owned Route C host bootstrap boundary so service provider registration/startup moves out of Web procedural route startup where dependency gates allow.
- Make Web consume a prepared runtime/facade bundle and keep Web responsibilities limited to HTTP, SSE, GenUI, approval UI, response mapping, and presentation logging.
- Expand Web route migration through SDK/SystemFacade or focused service clients for low-risk status/inspection surfaces while preserving existing response shapes.
- Keep deprecated Web provider/runtime fields as explicit compatibility anchors, document remaining high-risk paths, and add guards so new code does not treat them as normal dependencies.
- Keep CLI as terminal parsing/formatting/process-launch shell, migrate read-only inspection paths through SDK/SystemFacade, and narrow or document the remaining `macaca-cli -> macaca-web` server-start compatibility edge.
- Update Route C governance and allowlist only when executable dependency gates prove actual direct edge changes.

## Impact

- Affected specs: `web-cli-thin-shell-completion`
- Affected code:
  - `macaca/crates/macaca-runtime-host/src/*`
  - `macaca/crates/macaca-web/src/lib.rs`
  - `macaca/crates/macaca-web/src/state.rs`
  - `macaca/crates/macaca-web/src/shell.rs`
  - selected `macaca/crates/macaca-web/src/routes.rs`, `web3_status.rs`, and service/status adapters
  - `macaca/crates/macaca-cli/src/*`
  - `macaca/crates/macaca-sdk/src/system_facade.rs` only if a small shell-facing accessor/bundle helper is needed
  - `macaca/docs/route-c-serviceization-allowlist.md`
  - `macaca/docs/route-c-architecture-governance.md`
  - `macaca/crates/macaca-integration-tests/tests/route_c_dependency_boundaries.rs` only if direct dependency edges actually change

## Non-Goals

- No full Web rewrite.
- No frontend redesign unless a specific shell guardrail requires documentation-only changes.
- No `/api/chat/v2` wire-format change.
- No deletion of chat/session/resume compatibility paths.
- No real provider implementation for LLM, Memory, Driver, Skill, MCP, Payment, Web3, or EVM.
- No new crate unless implementation proves a dependency cycle and the design is updated before coding.
- No false deletion of allowlist rows without `cargo metadata` and dependency-gate proof.
