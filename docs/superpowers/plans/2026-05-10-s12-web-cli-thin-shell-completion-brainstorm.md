# S12 Web / CLI Thin Shell Completion Brainstorm

## Problem

Route C S12 must finish the Web / CLI thin shell direction after S0-S11 serviceization. The earlier `add-web-cli-thin-shell-v0` work created the first SDK facade, one Web route adapter, CLI command handlers, and guardrails. The remaining debt is larger: `macaca-web` still acts as the host composition root for many services, `macaca-cli` still depends on `macaca-web` for server startup, and `AppState` still exposes provider/runtime fields as deprecated compatibility anchors.

The completion phase must reduce Web and CLI to adapters without breaking existing `/api/chat/v2`, SSE trace, session replay, task board, application startup, driver/skill/MCP tools, memory/context recall, Store/Entitlement, Payment/A2A, Web3, or EVM behavior.

## Current Evidence

- `macaca/docs/agent-os-microkernel-boundaries.md` says Web/CLI are presentation shells and must not define core session/task/trace/payment/package semantics.
- `macaca/docs/route-c-serviceization-allowlist.md` still tracks presentation debt, especially `macaca-cli -> macaca-web`, `macaca-web -> macaca-persist`, `macaca-web -> macaca-tools`, and service provider composition still living in Web startup.
- `macaca/crates/macaca-web/src/lib.rs` registers and starts Application, LLM, Memory, Context, Driver, Skill, MCP, Store, Entitlement, Payment, Web3, and EVM providers directly.
- `macaca/crates/macaca-web/src/state.rs` contains service clients, but also deprecated provider/runtime fields such as `runtime`, `registry`, `llm`, `llm_router`, `memory_runtime`, `mcp_runtime`, `driver_registry`, and `driver_runtime`.
- `macaca/crates/macaca-cli/Cargo.toml` still depends on `macaca-web`; `WebCommandHandler` starts `macaca_web::WebServerBuilder` directly.
- `macaca/crates/macaca-sdk/src/system_facade.rs` is already the correct shell-facing Facade seam and now carries focused clients for S5-S11 services.

## Design Pattern Options

### Option A: Big-bang Web state purge

Patterns: Facade + Adapter + Command.

Approach: Remove all provider/runtime fields from `AppState`, delete direct provider construction from Web startup, and update every caller in one phase.

Benefits:

- Fastest conceptual endpoint.
- Removes many allowlist rows immediately if successful.

Risks:

- Very high regression blast radius for chat/session/SSE/task/resume.
- Current framework toolkit and agent execution still use host-local mutable objects; forcing all of them through services at once risks creating fake service abstractions.
- Hard to review, hard to rollback, likely violates the small reversible change rule.

Decision: Reject. S12 completion must be additive-first and slice-based.

### Option B: Shared runtime bootstrap plus Web/CLI adapters

Patterns: Abstract Factory + Builder + Facade + Adapter + Bridge + Command + Observer + Memento + Specification.

Approach: Move provider/service construction into a reusable host bootstrap boundary owned outside presentation route code, then let Web and CLI consume a prepared shell runtime handle and `SystemFacade`. Web remains an HTTP/SSE/GenUI adapter; CLI remains a terminal adapter. Deprecated direct fields remain temporarily as compatibility anchors where required, but new production paths use the bootstrap/facade seam.

Benefits:

- Directly attacks the largest remaining S9-S11 debt: Web as composition root.
- Enables CLI to start a host without depending on `macaca-web` internals.
- Keeps service provider ownership in `macaca-runtime-host` or shared bootstrap logic, not route handlers.
- Allows thin shell migration to proceed while preserving `/api/chat/v2` and SSE behavior.

Risks:

- Requires careful dependency design to avoid introducing a new crate or dependency cycle.
- If bootstrap boundary becomes too broad, it can become a new macro-service.
- Some host-local objects still need compatibility exposure until framework/toolkit/session orchestration are service-ready.

Mitigations:

- Keep bootstrap focused on constructing existing runtime/service clients and returning explicit handles.
- Do not create a generic service locator or stringly-typed registry.
- Use Builder/Abstract Factory for construction, Facade for consumption, and typed Strategy clients for capabilities.
- Keep deprecated compatibility handles visible and documented until each consumer migrates.

Decision: Recommended.

### Option C: Move Web server startup into `macaca-cli`

Patterns: Adapter only.

Approach: Delete `macaca-cli -> macaca-web` by copying or reimplementing server startup in CLI.

Benefits:

- Removes one allowlist edge quickly.

Risks:

- Duplicates presentation startup logic.
- Makes CLI another composition root.
- Violates thin shell because CLI would own Web runtime semantics.

Decision: Reject.

### Option D: Move all Web startup composition into SDK

Patterns: Facade + Builder.

Approach: Make `macaca-sdk` construct service runtimes, providers, persistence, LLM router, memory/runtime, and application lifecycle.

Benefits:

- Web and CLI become thin immediately.

Risks:

- SDK would depend on provider/runtime crates, breaking its purpose as a shell-facing stable API.
- Creates dependency cycles and makes SDK a hidden host.

Decision: Reject. SDK must remain consumer-facing and provider-neutral.

### Option E: Host bootstrap module inside `macaca-runtime-host`

Patterns: Abstract Factory + Builder + Facade + Bridge.

Approach: Add host bootstrap code to `macaca-runtime-host`, where service runtime/provider ownership already lives. The bootstrap returns a host runtime bundle and SDK clients/facade. Web consumes it; CLI can call a thin Web adapter or future host runner without owning providers.

Benefits:

- Aligns with S1-S11 ownership: runtime-host owns provider lifecycle.
- Avoids adding a new crate.
- Provides a clear path to remove Web as provider construction hub.

Risks:

- `macaca-runtime-host` may need dependencies currently only used by Web, such as LLM, driver, skill, memory, persist, task, or app.
- Existing dependency graph must be checked before adding edges.

Mitigations:

- Start with a bootstrap module that assembles already-owned runtime-host providers and returns clients; only move construction that does not create new forbidden edges.
- For Web-only persistence/session/toolkit state, define provider-neutral input handles instead of making runtime-host own presentation state.

Decision: Recommended as the main implementation direction, with dependency-gate checks per moved dependency.

## Recommended Architecture

Use Option B with Option E as the concrete direction:

- `macaca-runtime-host` owns a `RouteCHostBootstrap` / `HostRuntimeFactory` boundary that registers and starts system services.
- `macaca-sdk` continues to expose `SystemFacade` and focused clients only.
- `macaca-web` owns HTTP/SSE/GenUI adapters and Web-specific state, but receives service clients/runtime bundle from bootstrap instead of directly registering providers in route startup.
- `macaca-cli` owns terminal parsing/formatting and should no longer depend on Web internals for system inspection or host bootstrap. The remaining `web` command must call a stable server adapter, not route/service internals.
- Deprecated Web fields stay as searchable compatibility anchors until each route/tool/framework consumer migrates, but new call paths must use `SystemFacade` or focused clients.

## Design Pattern Fit

- Abstract Factory: `HostRuntimeFactory` creates provider-neutral runtime bundles without hardcoding application/provider-specific behavior.
- Builder: bootstrap construction steps are explicit and testable, avoiding a giant constructor.
- Facade: `SystemFacade` remains the only shell-facing system API.
- Adapter: Web HTTP/SSE/GenUI and CLI terminal commands adapt transports to typed commands.
- Bridge: runtime-backed clients bridge SDK traits to local `ServiceRuntime`, future remote service bus, or plugin transports.
- Command: routes and CLI commands become typed command objects with trace scope.
- Observer: SSE/trace only observes event streams and never redefines trace semantics.
- Memento: session replay, task board snapshots, service snapshots, and trace cursors remain replayable state.
- Specification: dependency boundary, thin shell guardrails, trace/policy requirements, and allowlist expiry are executable rules.

## Risks

- Web startup is currently a long procedural function; extracting slices can accidentally change startup ordering.
- Application startup, skill loading, driver loading, memory exposure, and framework toolkit assembly are interdependent.
- Moving provider construction can introduce forbidden dependency edges.
- CLI cannot become truly independent if the only server implementation remains in `macaca-web`.
- Some direct Web state is still necessary for chat orchestration until Task/Trace/Session service ownership is complete.

## Risk Controls

- Start by adding an explicit host bootstrap boundary and tests before deleting direct code.
- Move one service registration family at a time, starting with low-risk S9-S11 services that already have runtime-host providers and SDK clients.
- Preserve existing Web startup order and trace ids in the first extraction.
- Keep old Web composition helpers deprecated, not deleted.
- Run `cargo test -p macaca-integration-tests route_c_dependency_boundaries` after every dependency-affecting slice.
- Use hardcode scans for app/workflow/provider/driver/gateway/model/chain/package/business names.

## Open Questions For Implementation

- Whether the first OpenSpec should move all service provider registrations into one bootstrap module or only S9-S11 provider families first.
- Whether `macaca-cli -> macaca-web` can be removed without a new crate, or whether Web server adapter ownership must remain until a later dedicated host crate exists.
- Which Web direct fields can be converted from public deprecated fields into private compatibility handles without breaking tests.
- Whether `macaca-persist` session/trace access can be routed through an SDK trace/session client in this phase or needs a dedicated Persistence Service proposal.

## Recommendation

Proceed with a single S12 completion OpenSpec, but implement it in strict slices:

1. Runtime-host bootstrap boundary.
2. Web startup consumes bootstrap instead of registering S9-S11 providers directly.
3. Expand bootstrap to S5-S8 providers where dependency gates allow.
4. Introduce a CLI server adapter seam and reduce direct `macaca-cli -> macaca-web` usage to a deprecated compatibility path.
5. Migrate remaining low-risk Web routes to `SystemFacade`.
6. Update allowlist and executable gates only when direct dependency edges are actually removed.

Do not delete deprecated compatibility code in this phase unless GitNexus and dependency gates prove no production caller remains.
