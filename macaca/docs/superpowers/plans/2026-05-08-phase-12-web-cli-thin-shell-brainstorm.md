# Phase 12 Web / CLI Thin Shell Brainstorm

## Problem

`macaca-web` and `macaca-cli` still contain too much system coordination behavior. Web routes and CLI commands should adapt HTTP, SSE, UI, and terminal input into system commands, then delegate to SDK/Application/Kernel facades. They should not keep owning session, task, trace, service, package, entitlement, plugin, Web3, EVM, or GenUI system semantics.

This matters because Route C turns Macaca into a microkernel Agent OS. If Web/CLI remain the de facto orchestration layer, new applications and non-Web shells will depend on presentation internals instead of stable OS contracts.

## Design Pattern Options

### Option A: Big-bang Web/CLI rewrite

- Pattern: Facade + Command, applied broadly.
- Benefit: Fast conceptual cleanup.
- Risk: High regression risk for `/api/chat/v2`, SSE, session replay, task board, and frontend behavior.
- Rejected because Phase 12 explicitly forbids rewriting the whole Web shell at once.

### Option B: SDK-first thin shell migration

- Pattern: Facade + Command + Adapter + Observer + Visitor + Specification.
- Benefit: Creates a stable `macaca-sdk` system facade, then lets Web/CLI migrate route-by-route and command-by-command.
- Risk: Some Web semantics remain temporarily duplicated until all consumers migrate.
- Mitigation: mark direct presentation-owned semantic paths as legacy/deprecated after each slice and add migration guards.
- Recommended because it is additive, testable, and reversible.

### Option C: Move semantics directly into Kernel

- Pattern: Facade, but too low-level.
- Benefit: Removes Web ownership quickly.
- Risk: Violates microkernel boundary by moving replaceable service/application behavior into kernel.
- Rejected because kernel must own invariants only.

### Option D: Service-bus-only migration

- Pattern: Command + Bridge.
- Benefit: Clean service-call architecture.
- Risk: Requires broader service coverage before Web/CLI can migrate; too much infrastructure for this phase.
- Deferred because SDK facade can hide service-bus details now and route to service bus later.

## Recommended Approach

Use Option B: SDK-first thin shell migration.

Phase 12 should define:

- `macaca-sdk::SystemFacade` as a stable facade for shell-facing session, task, trace, package, and service inspection commands.
- Web route command adapters that convert request data into SDK commands and preserve existing response shapes.
- CLI command handlers that call the SDK facade rather than constructing lower-level runtime state directly.
- Trace/SSE shell boundaries where Web subscribes and renders events without defining trace semantics.
- GenUI shell mount rules where frontend renders application UI through generic schema/visitor paths, not application hardcode.
- Explicit deprecation/migration guard for direct Web/CLI semantic helpers after they receive facade-backed replacements.

## Risks

- Existing Web routes may have implicit behavior not captured by SDK commands.
- Frontend may depend on exact JSON shape or event ordering.
- SSE trace deduplication can regress if migration changes EventLog subscription order.
- CLI commands may currently bootstrap isolated state; facade migration must not require a running Web server.
- Over-abstracting the facade could create a generic dumping ground.

## Mitigations

- Start with low-risk read-only routes such as task board/session events/service inspection.
- Preserve external API response shapes in the first migration slice.
- Keep `RC-CHAT-001`, `RC-CHAT-002`, `RC-TRACE-001`, `RC-TRACE-002`, and `RC-TASK-001` as regression gates.
- Keep facade methods concrete and shell-facing instead of introducing broad dynamic RPC.
- Add structured logs and trace/audit fields to every shell command execution boundary.
- Avoid new dependencies unless implementation proves they are required.

## Rollback

Each migrated route or CLI command should be independently reversible by switching the adapter back to the previous internal call. New SDK facade contracts are additive and can remain unused if one slice is rolled back.
