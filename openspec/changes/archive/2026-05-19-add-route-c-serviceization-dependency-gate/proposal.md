# Change: Add Route C Serviceization Dependency Gate

## Why

Route C now has many microkernel, service, package, plugin, Web3/EVM, and thin-shell contracts, but the workspace can still drift back into macro-kernel coupling through direct crate dependencies. S0 must make these boundary violations executable and auditable before S1-S13 migrate providers and consumers.

The dependency gate does not remove current migration debt. It prevents new untracked debt and requires every temporary exception to appear in a documented allowlist with replacement service/facade path and target migration phase.

## What Changes

- Add an executable Route C dependency boundary test for the Rust workspace.
- Define a crate layer model aligned with `macaca/docs/agent-os-microkernel-boundaries.md`.
- Traverse `cargo metadata --no-deps --format-version 1` and classify direct workspace dependency edges.
- Enforce initial forbidden dependency rules using Specification + Visitor + Chain of Responsibility patterns.
- Add `macaca/docs/route-c-serviceization-allowlist.md` as a migration-debt memento.
- Update `macaca/docs/route-c-architecture-governance.md` to reference the executable dependency gate.
- Ensure diagnostics are deterministic, actionable, and audit-friendly.
- Require detailed English comments in new test/helper code explaining rule purpose, graph traversal, allowlist semantics, and non-goals.

## Impact

- Affected specs: `serviceization-dependency-gate`
- Affected code: `macaca/crates/macaca-integration-tests/tests/route_c_dependency_boundaries.rs`
- Affected docs: `macaca/docs/route-c-serviceization-allowlist.md`, `macaca/docs/route-c-architecture-governance.md`
- Affected validation: `cargo test -p macaca-integration-tests route_c_dependency_boundaries`, `cargo metadata --no-deps --format-version 1`

## Governance Alignment

- Follows `macaca/docs/agent-os-microkernel-boundaries.md`: kernel owns invariants, services own replaceable capabilities, Web/CLI are presentation shells, and optional modules must remain optional.
- Follows `macaca/docs/route-c-serviceization-allowlist.md`: current exceptions must be documented with migration phase and replacement path. If the file does not exist yet, this change creates it.
- Follows `macaca/docs/route-c-architecture-governance.md`: dependency violations must be represented as failing tests or documented allowlist rows.
- Uses Specification, Visitor, Chain of Responsibility, Strategy, and Memento patterns.

## Non-Goals

- Do not remove any provider dependencies in S0.
- Do not rewrite kernel, Web, CLI, SDK, task, LLM, memory, driver, skill, MCP, gateway, payment, Web3, or EVM runtime paths.
- Do not implement ServiceRuntime v1.
- Do not introduce a new dependency policy language or external dependency gate tool.
- Do not change user-visible `/api/chat/v2`, trace, task board, resume, driver, skill/MCP, or YAML application behavior.
