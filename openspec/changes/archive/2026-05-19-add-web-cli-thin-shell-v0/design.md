# Design: Web / CLI Thin Shell v0

## Context

Route C separates OS invariants, replaceable services, application behavior, and presentation shells. Earlier phases introduced kernel primitives, system services, package/runtime guardrails, GenUI, plugin runtime, store/entitlement, A2A payment, optional Web3, and optional EVM/DApp boundaries. Phase 12 changes the consumption direction: Web/CLI should consume these facades rather than remain the system coordinator.

The current code already contains useful stepping stones such as `macaca-web::route_command`, GenUI thin routes, CLI command handlers, and SDK facades. Phase 12 should formalize these patterns and migrate one low-risk slice first.

## Goals

- Introduce a typed SDK system facade for shell-facing commands.
- Keep Web routes as HTTP command adapters.
- Keep CLI handlers as terminal command adapters.
- Keep SSE/trace as observer/subscriber presentation behavior.
- Keep frontend GenUI rendering generic and schema-driven.
- Preserve existing user-visible Web API and frontend behavior during initial migration.
- Add trace/audit logs at shell command execution boundaries.
- Keep all new Rust files below 500 lines with detailed English comments.

## Non-Goals

- No full Web rewrite.
- No route wire-format breakage in the first slice.
- No frontend redesign.
- No new provider/plugin/payment/Web3/EVM implementation.
- No kernel ownership of presentation behavior.
- No generic untyped RPC dumping ground.

## Design Decisions

### 1. SDK system facade as shell boundary

Add `macaca-sdk/src/system_facade.rs` with typed commands/results for shell-facing operations:

- task board query,
- session event query,
- trace tail intent,
- service inspection,
- package inspection,
- approval decision.

Pattern: Facade + Command + Memento.

The facade should expose concrete methods and structs, not a generic `execute(name, json)` API. It may start with in-memory/mock adapters and current store/kernel adapters, then route to service bus later without changing Web/CLI contracts.

### 2. Web routes are command adapters

Web routes should validate request scope, build typed SDK commands, log the command execution, delegate to the SDK facade, and preserve existing response shape. `macaca-web` may own HTTP parsing, status mapping, SSE transport, and rendering adapters, but it must not own core session/task/trace/package/service semantics.

Pattern: Adapter/Bridge + Command.

The first implementation slice should migrate a low-risk read-only route such as task board or session events.

### 3. Trace/SSE is Observer presentation

Web should subscribe to EventLog/trace/service streams and forward/render events. It must not invent new trace semantics or duplicate persistence rules.

Pattern: Observer + Memento.

Events must carry session/task scope and replay cursors. Real-time and historical replay behavior must remain compatible with `RC-TRACE-001` and `RC-TRACE-002`.

### 4. Frontend is a generic shell and Visitor renderer

Frontend should render chat, trace, task board, package metadata, and GenUI surfaces through generic schemas. GenUI and trace rendering dispatch by schema/component/event kind, not by application name, workflow name, provider name, gateway name, driver name, or business route.

Pattern: Visitor + Strategy.

When no GenUI surface exists, existing chat/trace shell remains the default.

### 5. CLI is a facade-backed command shell

CLI command handlers should construct typed SDK commands and call the system facade. CLI can format output, handle terminal flags, and start the Web server, but it must not define core system semantics.

Pattern: Command + Facade.

Existing deprecated compatibility helpers can remain until consumers migrate, but new paths should use command handlers and SDK facade.

### 6. Deprecation and migration guard

After a direct Web/CLI semantic helper is replaced by a facade-backed path, it should be marked deprecated or restricted to compatibility usage. Tests or scripts should guard against new upper-layer direct calls to deprecated paths.

Pattern: Specification.

## Alternatives Considered

### Big-bang Web rewrite

Rejected because `/api/chat/v2`, SSE, session replay, frontend behavior, and recovery are high-risk user-visible paths.

### Move session/task/trace semantics into kernel

Rejected because kernel owns invariants only. Task, trace, package, approval, and service views are facades/services, not presentation or kernel internals.

### Service-bus-only migration

Deferred because it requires broader service coverage. SDK facade can hide service-bus details now and later route through service bus without changing Web/CLI.

### Keep Web as coordinator

Rejected because it violates the Route C presentation rule and blocks non-Web shells.

## Risks and Mitigations

- Risk: response shapes drift and break frontend.
  - Mitigation: first migrated route must preserve JSON shape and add tests.
- Risk: trace/SSE duplicates or drops events.
  - Mitigation: keep EventLog as replay source, preserve cursors, and verify `RC-TRACE-001`/`RC-TRACE-002`.
- Risk: facade becomes overbroad.
  - Mitigation: typed commands/results only; no generic dynamic RPC.
- Risk: CLI depends on running Web state.
  - Mitigation: CLI uses SDK facade and lower service adapters, not Web internals.
- Risk: frontend hardcodes application UI.
  - Mitigation: schema/component visitor rules and hardcode scans.

## Verification Plan

- `openspec validate add-web-cli-thin-shell-v0 --strict`
- `cargo test -p macaca-sdk`
- `cargo test -p macaca-web`
- `cargo check -p macaca-cli`
- `cargo test -p macaca-integration-tests --test route_c_baseline`
- `cd frontend && npm run lint && npx tsc --noEmit` if frontend files change
- hardcode scan over new shell/facade/frontend files
- `npx gitnexus detect-changes --repo agent`
