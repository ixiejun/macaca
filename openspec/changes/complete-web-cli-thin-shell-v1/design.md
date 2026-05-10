# Design: Web / CLI Thin Shell Completion v1

## Context

Route C makes Macaca an agent operating system with a microkernel, replaceable system services, optional modules, and thin presentation shells. Earlier serviceization stages added focused SDK clients and runtime-host providers for Application, LLM, Memory, Context, Driver, Skill, MCP, Store, Entitlement, Payment/A2A, Web3, and EVM.

The remaining S12 problem is not lack of service clients; it is ownership. `macaca-web` still registers and starts many providers in one procedural startup function, and `macaca-cli` still depends on Web for server startup. That makes Web/CLI behave like coordination hubs instead of adapters.

## Goals

- Move service provider lifecycle composition toward `macaca-runtime-host`.
- Keep `macaca-sdk::SystemFacade` and focused clients as the shell-facing Facade.
- Keep Web as HTTP/SSE/GenUI/approval adapter only.
- Keep CLI as terminal command adapter only.
- Preserve all user-visible behavior and Route C regression scenarios.
- Keep deprecated direct paths as searchable migration anchors, not normal paths.
- Keep all new code traceable, auditable, documented with detailed English comments, and under the 500-line-per-file rule.

## Non-Goals

- No big-bang Web state purge.
- No kernel ownership of provider/service/application presentation behavior.
- No SDK ownership of provider construction.
- No service locator or untyped RPC dumping ground.
- No deletion of deprecated compatibility anchors in this change.
- No new business/application/provider hardcode.

## Design Patterns

- **Abstract Factory**: runtime-host host bootstrap creates service provider bundles without Web route code knowing concrete construction steps.
- **Builder**: bootstrap is split into explicit registration/start phases, making ordering and rollback visible.
- **Facade**: `SystemFacade` remains the stable API consumed by Web and CLI shells.
- **Adapter**: Web routes/SSE/GenUI and CLI commands adapt transports to typed commands.
- **Bridge**: SDK service clients bridge to local `ServiceRuntime` now and future remote/plugin transports later.
- **Command**: migrated route/CLI operations use typed, trace-scoped command objects.
- **Observer**: SSE/trace observes event sources and snapshots without redefining trace semantics.
- **Memento**: session replay, trace cursors, task-board snapshots, and service snapshots remain replayable data.
- **Specification**: dependency gate, thin-shell guardrails, validation, and allowlist expiry are executable constraints.

## Architecture

### Runtime-Host Bootstrap Boundary

Introduce a runtime-host-owned bootstrap boundary, for example `RouteCHostBootstrap` and `RouteCHostRuntimeBundle`.

The bootstrap boundary SHALL accept typed input handles instead of reading Web state directly. Candidate handles include application registry/runtime/kernel handles, service runtime config, persistence/store handles, LLM/provider handles, driver runtime handles, skill/MCP handles, and optional-module configuration.

The bootstrap SHALL return:

- the `ServiceRuntime`,
- started service identifiers,
- sanitized diagnostics,
- enough typed handles for Web to build runtime-backed SDK clients.

The bootstrap SHALL emit structured logs for bootstrap begin, provider registration, provider start, provider unavailable, failure, and completion. Logs SHALL NOT include secrets, prompt bodies, raw package bytes, credentials, private keys, raw tool payloads, raw signed transactions, or unbounded user input.

### Web Shell Consumption

Web startup SHALL consume the bootstrap output and build SDK focused clients or `SystemFacade` through a small Web-local adapter. Web may still own HTTP server state, SSE transport, frontend response mapping, active session caches, and compatibility state needed by existing chat/session execution.

Initial implementation SHOULD move the safest provider families first:

- Store / Entitlement,
- Payment / A2A,
- Web3 / EVM.

Application, LLM, Memory, Context, Driver, Skill, and MCP provider registration can move through the same seam only when dependency gates prove the move does not introduce new forbidden edges.

### CLI Shell Completion

CLI SHALL keep parsing flags, formatting output, handling process lifecycle, and launching the Web server. CLI system inspection SHOULD use `SystemFacade` or focused SDK clients.

The remaining `macaca-cli -> macaca-web` edge SHALL be either removed or narrowed to a documented server-start compatibility edge. The implementation SHALL NOT move Web runtime semantics into CLI to fake dependency cleanup.

### Deprecated Compatibility Anchors

Deprecated fields and helpers remain until separate proof shows they can be removed. The implementation SHALL scan and classify production uses of:

- `AppState::runtime`,
- `AppState::registry`,
- `AppState::llm`,
- `AppState::llm_router`,
- `AppState::memory_runtime`,
- `AppState::mcp_runtime`,
- `AppState::driver_registry`,
- `AppState::driver_runtime`,
- deprecated CLI direct helpers.

Low-risk reads SHOULD migrate to SDK/service snapshots. High-risk chat/framework/session paths SHALL remain documented compatibility anchors.

## Tradeoffs

### Why not delete all deprecated Web fields now?

Because `/api/chat/v2`, session resume, framework toolkit assembly, and host-local MCP/driver/skill attachment still depend on mutable in-process state. Removing all fields before dedicated session/task/toolkit service ownership exists would create a hidden macro-service or break regression scenarios.

### Why runtime-host bootstrap instead of SDK bootstrap?

SDK must stay provider-neutral and shell-facing. If SDK constructs providers, it becomes the hidden host and gains forbidden concrete dependencies. Runtime-host already owns service provider lifecycle and is the correct ownership layer.

### Why not remove `macaca-cli -> macaca-web` immediately?

The CLI `web` command is a process launcher for the Web server. Removing the edge without a stable host/server abstraction would duplicate server startup inside CLI. This change may narrow and document the edge first, then remove it only if implementation creates a clean adapter seam.

## Trace And Audit

Every new bootstrap, Web shell, and CLI shell boundary SHALL log:

- shell or bootstrap kind,
- operation name,
- service id when available,
- command name when available,
- trace id when available,
- app/session/task scope when available,
- success/rejection/failure status,
- sanitized reason code.

Mutating service calls SHALL continue to require trace through existing `ServiceRuntime` decorators. Thin-shell logs are not a replacement for service trace; they are an audit bridge at the presentation boundary.

## Dependency Governance

The implementation SHALL run the dependency boundary gate before changing allowlist rows. Allowlist rows are removed only when the executable gate and `cargo metadata` prove the direct edge is gone. If an edge is merely narrowed, documentation must state the exact remaining reason and expiry condition.

## Risks And Mitigations

- Risk: Web startup order changes.
  Mitigation: preserve existing service ids, trace ids, and deterministic start order in the first extraction.
- Risk: bootstrap becomes a service locator.
  Mitigation: expose typed bundle fields and focused client construction, not arbitrary string lookup.
- Risk: new forbidden dependency edges appear.
  Mitigation: run GitNexus impact and dependency boundary tests per slice.
- Risk: trace/SSE or task-board regression.
  Mitigation: keep Route C regression matrix checks mandatory and do not change `/api/chat/v2` wire format.
- Risk: deprecated anchors keep growing.
  Mitigation: add direct-consumer guards and document remaining compatibility paths.

## Verification

Required before implementation is considered complete:

```bash
openspec validate complete-web-cli-thin-shell-v1 --strict
cargo fmt --all --check
cargo check --workspace
cargo test -p macaca-runtime-host service_runtime
cargo test -p macaca-sdk
cargo test -p macaca-web
cargo check -p macaca-cli
cargo test -p macaca-integration-tests --test route_c_baseline
cargo test -p macaca-integration-tests route_c_dependency_boundaries
```

If frontend files change:

```bash
cd frontend && npm run lint && npx tsc --noEmit
```

Hardcode scan and GitNexus:

```bash
rg -n "FULLSTACK|AUTODEV|workflow|driver name|gateway name|model name|provider name|chain name|package name|business|localhost|127\\.0\\.0\\.1" macaca/crates/macaca-runtime-host/src macaca/crates/macaca-web/src macaca/crates/macaca-cli/src macaca/crates/macaca-sdk/src
npx gitnexus detect-changes -r agent
```
