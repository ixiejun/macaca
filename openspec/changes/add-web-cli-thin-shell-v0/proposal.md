# Change: Add Web / CLI Thin Shell v0

## Why

Route C Phase 12 must move `macaca-web`, frontend, and `macaca-cli` toward presentation shells instead of system coordination layers. Web/CLI should adapt HTTP, SSE, GenUI, approval, package UI, and terminal input into typed system commands, then delegate to SDK/Application/Kernel facades.

Without this boundary, core session, task, trace, package, service, payment, Web3, EVM, plugin, and entitlement semantics remain coupled to the Web server or CLI process. That prevents Macaca from behaving like an agent OS with multiple interchangeable shells.

## What Changes

- Add a shell-facing SDK system facade for session, task, trace, package, service, approval, and inspection commands.
- Add Web shell command adapter rules so HTTP routes validate scope, log execution, call SDK facades, and preserve existing response shapes during migration.
- Add trace/SSE thin-shell rules: Web subscribes, forwards, and renders trace data without defining core trace semantics.
- Add GenUI shell mount guardrails so frontend renders generic schema/component surfaces and falls back to chat/trace shell when no application UI exists.
- Add CLI thin-shell rules so CLI command handlers delegate to SDK facade commands rather than owning system semantics directly.
- Add deprecation/migration guard requirements for old direct presentation-owned semantic helpers after facade-backed replacements exist.
- Preserve Route C regression matrix scenarios `RC-CHAT-001`, `RC-CHAT-002`, `RC-TRACE-001`, `RC-TRACE-002`, and `RC-TASK-001`.
- Require detailed English comments and structured logs for all new Rust code during implementation.

## Impact

- Affected specs: `web-cli-thin-shell-v0`
- Affected crates/areas: `macaca-sdk`, `macaca-web`, `macaca-cli`, frontend shell/GenUI/trace surfaces, and integration tests
- Affected code areas: SDK system facade, Web route command adapters, trace/SSE shell boundaries, GenUI mount behavior, CLI command handlers, migration guards, and documentation
- Regression matrix references: `RC-CHAT-001`, `RC-CHAT-002`, `RC-TRACE-001`, `RC-TRACE-002`, `RC-TASK-001`

## Governance Alignment

- Follows `macaca/docs/agent-os-microkernel-boundaries.md`: Web/CLI/frontend are presentation shells and must consume system facades.
- Follows `macaca/docs/route-c-regression-matrix.md`: Phase 12 explicitly protects chat session creation/resume, real-time trace, trace replay, and session-scoped task board behavior.
- Follows `macaca/docs/route-c-phase-template.md`: includes Superpowers brainstorm/write-plan, OpenSpec proposal/design/tasks/spec, GitNexus impact, additive implementation, targeted tests, integration smoke, detect_changes, and commit gates.
- Follows `macaca/docs/route-c-architecture-governance.md`: uses Facade, Command, Adapter/Bridge, Observer, Visitor, Specification, and Memento; presentation code must not redefine kernel/service/application contracts.

## Non-Goals

- Do not rewrite the whole `macaca-web`.
- Do not change `/api/chat/v2` wire behavior in the first migration slice.
- Do not remove existing Web/CLI compatibility paths until consumers migrate.
- Do not move Web/CLI presentation responsibilities into kernel.
- Do not implement new Store, payment, Web3, EVM, plugin, or package runtime features beyond facade/shell contracts.
- Do not hardcode application names, workflow names, driver names, gateway names, model names, provider names, chain names, package names, or business routes.
- Do not add frontend branches for a specific application UI.
