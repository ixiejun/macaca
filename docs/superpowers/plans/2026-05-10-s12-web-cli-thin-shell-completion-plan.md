# S12 Web / CLI Thin Shell Completion Plan

## Objective

Complete Route C S12 by turning Web and CLI into presentation adapters over SDK/SystemFacade and runtime-host service bootstrap. This plan builds on `add-web-cli-thin-shell-v0` and S5-S11 serviceization. It must preserve existing Web UI, `/api/chat/v2`, SSE trace, session replay, task board, driver/skill/MCP, memory/context, Store/Entitlement, Payment/A2A, Web3, and EVM behavior.

## Hard Constraints

- Follow `macaca/docs/agent-os-microkernel-boundaries.md`.
- Follow `macaca/docs/route-c-serviceization-allowlist.md`.
- Follow `macaca/docs/route-c-architecture-governance.md`.
- Use OpenSpec before implementation.
- Run GitNexus impact before editing existing Rust symbols.
- Keep changes small, additive, reversible, and reviewable.
- Keep every Rust file below 500 lines.
- Do not add app/workflow/provider/driver/gateway/model/chain/package/business hardcode.
- Preserve deprecated compatibility code as migration anchors unless a separate deletion proposal proves it is unused.
- All new Rust code must include detailed English comments explaining functionality and runtime behavior.
- Key execution nodes must emit structured logs without secrets or unbounded user input.

## Design Pattern Selection

Use these patterns explicitly:

- Abstract Factory: runtime-host creates service provider bundles through a host factory, not Web route code.
- Builder: bootstrap steps are explicit and testable.
- Facade: Web/CLI consume `SystemFacade` and focused SDK clients.
- Adapter: Web routes/SSE/GenUI and CLI terminal commands only adapt transport and formatting.
- Bridge: SDK clients bridge to local `ServiceRuntime` now and future remote/plugin transports later.
- Command: all migrated routes/commands become typed, trace-scoped commands.
- Observer: SSE/trace surfaces observe streams and snapshots without owning trace semantics.
- Memento: session replay, task board, service snapshot, and trace cursor data remain replayable.
- Specification: dependency gate, thin shell guardrails, command validation, and allowlist expiry are executable constraints.

## Scope Boundaries

### In Scope

- Create or update an OpenSpec proposal for S12 completion.
- Move service runtime provider registration out of Web procedural startup into a reusable runtime-host bootstrap boundary where dependency gates allow.
- Keep Web startup as an HTTP/SSE/GenUI adapter consuming a prepared runtime/facade bundle.
- Reduce CLI dependence on Web internals by introducing a stable command/server adapter seam.
- Migrate additional low-risk Web routes and CLI inspection commands through SDK/SystemFacade.
- Update governance/allowlist with actual migrated edges and remaining debt.
- Add tests and scans proving Web/CLI stay thin.

### Out Of Scope

- No full Web rewrite.
- No frontend redesign.
- No change to `/api/chat/v2` wire format.
- No removal of chat/session/resume compatibility paths without separate proof.
- No new real provider implementation for LLM, Memory, Driver, Skill, MCP, Payment, Web3, or EVM.
- No new crate unless implementation proves dependency cycles make it unavoidable and OpenSpec explicitly approves it.
- No migration of all persistence semantics unless a dedicated Persistence Service boundary already exists or is added by proposal.

## Implementation Slices

### Slice S12.1: OpenSpec and Baseline Audit

1. Create `openspec/changes/complete-web-cli-thin-shell-v1/`.
2. Write `proposal.md`, `design.md`, `tasks.md`, and spec deltas for Web/CLI thin shell completion.
3. Reference previous `add-web-cli-thin-shell-v0` as the baseline already completed.
4. Audit current call sites:
   - `macaca-web/src/lib.rs` service registration/startup.
   - `macaca-web/src/state.rs` deprecated provider/runtime fields.
   - `macaca-web/src/routes.rs`, `session.rs`, `sse.rs`, `framework_runner.rs`, `framework_toolkit.rs`, `skill_mcp.rs`.
   - `macaca-cli/src/commands.rs`, `command_handlers.rs`, and `Cargo.toml`.
5. Run GitNexus impact for each edited symbol before code changes.

Expected result:

- Approved OpenSpec scope that separates bootstrap extraction, route migration, CLI migration, and allowlist cleanup.

Verification:

```bash
openspec validate complete-web-cli-thin-shell-v1 --strict
```

### Slice S12.2: Runtime-Host Bootstrap Boundary

1. Add a runtime-host bootstrap module, such as `macaca-runtime-host/src/route_c_bootstrap.rs` or `host_bootstrap.rs`.
2. Define typed bootstrap input handles instead of reading Web state directly:
   - application registry/runtime/kernel handles,
   - service runtime config,
   - optional persistence/store handles,
   - optional LLM/provider handles where already required by existing service providers,
   - optional driver/skill/MCP handles.
3. Define a `RouteCHostRuntimeBundle` that returns:
   - `Arc<ServiceRuntime>`,
   - generic runtime-backed service client factory input,
   - started service ids,
   - sanitized startup diagnostics.
4. Use Builder/Abstract Factory to register/start services in deterministic steps.
5. Add structured logs for bootstrap begin, provider register, provider start, provider unavailable, and bootstrap complete.
6. Do not leak secrets, prompt bodies, raw package bytes, provider credentials, wallet secrets, or raw tool payloads.

Expected result:

- Service provider lifecycle ownership begins moving out of Web procedural startup and into runtime-host.
- Web can request a service runtime bundle without owning provider registration semantics.

Verification:

```bash
cargo test -p macaca-runtime-host service_runtime
cargo test -p macaca-runtime-host route_c_bootstrap
cargo test -p macaca-integration-tests route_c_dependency_boundaries
```

### Slice S12.3: Web Startup Consumes Bootstrap

1. Replace direct Web startup registration for the safest service families first:
   - Store / Entitlement,
   - Payment / A2A,
   - Web3 / EVM.
2. Preserve existing service ids, trace ids, startup order, and SDK client behavior.
3. Keep old local Web registration helper code only as deprecated compatibility anchor if needed.
4. Extend to Application, LLM, Memory, Context, Driver, Skill, and MCP only after dependency boundary checks pass.
5. Keep Web-owned HTTP/SSE/session state in Web; do not move route/session presentation state into runtime-host.

Expected result:

- Web stops being the composition owner for S9-S11 services first.
- Later S5-S8 service startup can follow the same bootstrap seam.

Verification:

```bash
cargo check -p macaca-web
cargo test -p macaca-web
cargo test -p macaca-integration-tests route_c_dependency_boundaries
cargo test -p macaca-integration-tests --test route_c_baseline
```

### Slice S12.4: SystemFacade Bundle for Web

1. Add a small Web-local adapter that builds `SystemFacade::with_route_c_clients` from the runtime bundle.
2. Store the facade or focused clients in `AppState` as the primary path.
3. Convert selected routes to use the facade instead of deprecated fields:
   - status/service inspect,
   - Web3/EVM status,
   - Store/Entitlement/Payment inspection surfaces if present,
   - application list/status where response shape is already service-backed.
4. Preserve response JSON and status mapping.
5. Add structured logs for request scope validation, command creation, facade execution, success, rejection, and failure.

Expected result:

- Web routes become adapters over SDK clients.
- Deprecated `AppState` provider fields remain only for compatibility paths that still need them.

Verification:

```bash
cargo test -p macaca-web web3_status
cargo test -p macaca-web shell
cargo test -p macaca-web routes
```

### Slice S12.5: CLI Thin Shell Completion

1. Introduce a stable server/host adapter seam so CLI does not depend on Web internals for system inspection semantics.
2. Keep the `web` command as a terminal process launcher, but route startup through an intentionally narrow public API.
3. Migrate read-only CLI inspection commands to `SystemFacade` or focused SDK clients:
   - status,
   - service inspect,
   - application list/status if commands exist or are added.
4. Mark direct legacy CLI helpers as deprecated compatibility anchors.
5. If `macaca-cli -> macaca-web` direct dependency cannot be removed in this slice, update allowlist with exact remaining reason and expiry condition; do not pretend it is solved.

Expected result:

- CLI is terminal parsing/formatting/process lifecycle only.
- Any remaining Web dependency is isolated to server startup and explicitly tracked.

Verification:

```bash
cargo check -p macaca-cli
cargo test -p macaca-cli
cargo test -p macaca-integration-tests route_c_dependency_boundaries
```

### Slice S12.6: Deprecated Field and Direct Consumer Guard

1. Scan for production reads of deprecated Web fields:
   - `runtime`,
   - `registry`,
   - `llm`,
   - `llm_router`,
   - `memory_runtime`,
   - `mcp_runtime`,
   - `driver_registry`,
   - `driver_runtime`.
2. Migrate low-risk reads to SDK clients or service-backed snapshots.
3. For high-risk chat/framework/session paths, keep fields but add explicit comments explaining why they remain and which future proposal owns removal.
4. Add or update tests/scripts that prevent new presentation-owned semantic helpers.
5. Avoid false positives for lower-layer service provider implementations.

Expected result:

- Deprecated handles become constrained compatibility anchors, not a normal development path.

Verification:

```bash
rg -n "allow\\(deprecated\\)|deprecated\\(|state\\.(runtime|registry|llm|llm_router|memory_runtime|mcp_runtime|driver_registry|driver_runtime)" macaca/crates/macaca-web/src macaca/crates/macaca-cli/src
cargo test -p macaca-integration-tests route_c_dependency_boundaries
```

### Slice S12.7: Allowlist and Governance Update

1. Update `macaca/docs/route-c-serviceization-allowlist.md` only for edges actually removed or narrowed.
2. Update `macaca/docs/route-c-architecture-governance.md` with S12 completion ownership rules:
   - runtime-host owns host bootstrap,
   - SDK owns shell facade,
   - Web/CLI own adapters only,
   - deprecated compatibility handles must not receive new callers.
3. Update dependency boundary tests if direct edges are removed or a temporary exception changes.
4. Keep allowlist honest; do not delete rows unless `cargo metadata` and the executable gate prove the direct edge is gone.

Expected result:

- Architecture docs and executable gates match actual code.

Verification:

```bash
cargo test -p macaca-integration-tests route_c_dependency_boundaries
openspec validate complete-web-cli-thin-shell-v1 --strict
```

## Mandatory Regression Gates

Run these before declaring implementation complete:

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

Hardcode scan:

```bash
rg -n "FULLSTACK|AUTODEV|workflow|driver name|gateway name|model name|provider name|chain name|package name|business|localhost|127\\.0\\.0\\.1" \
  macaca/crates/macaca-runtime-host/src \
  macaca/crates/macaca-web/src \
  macaca/crates/macaca-cli/src \
  macaca/crates/macaca-sdk/src
```

GitNexus:

```bash
npx gitnexus detect-changes -r agent
```

## Acceptance Criteria

- Web no longer directly owns S9-S11 provider registration in route startup.
- New Web routes and migrated routes use SDK/SystemFacade or focused service clients.
- CLI read-only inspection paths use SDK/SystemFacade where available.
- CLI direct Web dependency is either removed or narrowed to a documented server-start compatibility edge.
- Deprecated Web provider/runtime fields remain only as clearly documented compatibility anchors.
- Dependency boundary allowlist is updated only where executable gates prove real edge changes.
- `/api/chat/v2`, SSE trace, session replay, and task board behavior do not regress.
- Logs exist for bootstrap, route command, CLI command, service registration/start, success, rejection, and failure nodes.
- No new business/application/provider hardcode is introduced.

## Rollback Strategy

- Keep old Web startup helpers until bootstrap extraction proves stable.
- Each migrated service family can switch back to the prior local registration path if a regression appears.
- Each route/CLI command migration is independent and can be reverted without deleting SDK facade types.
- Do not archive OpenSpec until code, tests, governance docs, and dependency gates all align.

## Suggested OpenSpec Change ID

`complete-web-cli-thin-shell-v1`
