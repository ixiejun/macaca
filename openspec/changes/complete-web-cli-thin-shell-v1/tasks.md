## 1. Preparation

- [x] 1.1 Read `docs/superpowers/plans/2026-05-10-s12-web-cli-thin-shell-completion-plan.md`, `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`, `macaca/docs/agent-os-microkernel-boundaries.md`, `macaca/docs/route-c-serviceization-allowlist.md`, `macaca/docs/route-c-architecture-governance.md`, `macaca/docs/route-c-regression-matrix.md`, and `macaca/docs/design_patterns.md`.
- [x] 1.2 Review previous `add-web-cli-thin-shell-v0` proposal/design/tasks/spec and confirm this change extends rather than rewrites it.
- [x] 1.3 Audit current `macaca-web/src/lib.rs`, `state.rs`, `shell.rs`, `routes.rs`, `session.rs`, `sse.rs`, `framework_runner.rs`, `framework_toolkit.rs`, `skill_mcp.rs`, `web3_status.rs`, and `service_runtime_client.rs`.
- [x] 1.4 Audit current `macaca-cli/src/commands.rs`, `command_handlers.rs`, `main.rs`, `lib.rs`, and `Cargo.toml`.
- [x] 1.5 Run GitNexus impact before editing existing symbols such as `serve_web_server`, `AppState`, `WebRuntimeSystemServiceClient`, `WebShellFacade`, `WebCommandHandler`, `execute_show_status`, and any moved provider registration helper.
- [x] 1.6 Warn before editing any HIGH or CRITICAL GitNexus impact symbol.
- [x] 1.7 Confirm each touched Rust file remains below 500 lines; split bootstrap, client construction, compatibility scanning, and route adapters before files grow too large.

## 2. Runtime-Host Bootstrap Boundary

- [x] 2.1 Add a runtime-host host bootstrap module with detailed English comments explaining why provider lifecycle belongs outside presentation shells.
- [x] 2.2 Define typed bootstrap input handles for service runtime config and the existing provider dependencies needed by the moved service families.
- [x] 2.3 Define `RouteCHostRuntimeBundle` or equivalent with `ServiceRuntime`, started service ids, sanitized diagnostics, and client-factory inputs.
- [x] 2.4 Implement Builder/Abstract Factory style registration and startup steps for Store/Entitlement, Payment/A2A, Web3, and EVM first.
- [x] 2.5 Preserve existing service ids, command names, trace ids, unavailable behavior, and fail-closed policy behavior.
- [x] 2.6 Add structured tracing logs for bootstrap begin, provider registration, provider start, provider unavailable, failure, and completion.
- [x] 2.7 Ensure bootstrap logs redact secrets, provider credentials, prompt bodies, raw package bytes, private keys, wallet secrets, raw signed transactions, raw ABI/bytecode, raw tool payload, and unbounded user input.
- [x] 2.8 Export only the minimal typed bootstrap API from `macaca-runtime-host/src/lib.rs`.
- [x] 2.9 Add runtime-host tests for successful bootstrap, unavailable optional services, duplicate registration rejection, and sanitized diagnostics.
- [x] 2.10 Run `cargo test -p macaca-runtime-host route_c_bootstrap service_runtime`.

## 3. Web Startup Consumes Bootstrap

- [x] 3.1 Update `macaca-web/src/lib.rs` so S9-S11 service families are registered/started via the runtime-host bootstrap boundary instead of direct Web procedural registration.
- [x] 3.2 Keep Web-owned HTTP/SSE/session/chat compatibility state in Web; do not move presentation state into runtime-host.
- [x] 3.3 Preserve existing Web startup ordering and trace ids where they are user/debug visible.
- [x] 3.4 Keep any replaced Web-local registration helper as deprecated compatibility code if rollback or migration search requires it.
- [x] 3.5 Build Store/Entitlement, Payment, Web3, and EVM SDK clients from the bootstrap/runtime bundle.
- [x] 3.6 If moving Application, LLM, Memory, Context, Driver, Skill, or MCP registration creates new forbidden dependency edges, stop that slice and document it as remaining debt instead of forcing it.
- [x] 3.7 Run `cargo check -p macaca-web`.

## 4. Web SystemFacade And Route Adapter Migration

- [x] 4.1 Add or extend a Web-local SystemFacade bundle adapter that constructs a route-safe bundle from runtime-backed focused clients.
- [x] 4.2 Store the facade or focused clients as the primary route path in `AppState` without deleting deprecated compatibility anchors.
- [x] 4.3 Migrate low-risk status/service inspection routes through `SystemFacade` or focused clients while preserving response JSON.
- [x] 4.4 Keep Web3/EVM status paths on `SystemWeb3Client` / `SystemEvmClient` and ensure unavailable state is data, not panic.
- [x] 4.5 Migrate Store/Entitlement/Payment inspection surfaces if present; do not invent package/payment semantics in Web.
- [x] 4.6 Add structured logs for scope validation, command construction, facade execution, success, rejection, and failure.
- [x] 4.7 Add tests for response-shape preservation and unavailable-state behavior.
- [x] 4.8 Run `cargo test -p macaca-web shell` and `cargo test -p macaca-web web3_status`.

## 5. CLI Thin Shell Completion

- [x] 5.1 Keep CLI command handlers as terminal adapters with detailed English comments explaining what semantics they delegate.
- [x] 5.2 Migrate read-only CLI status/service/application inspection paths through `SystemFacade` or focused SDK clients where available.
- [x] 5.3 Narrow the `web` command to a stable server-start adapter seam; do not duplicate Web runtime semantics inside CLI.
- [x] 5.4 Mark replaced direct CLI helper paths as deprecated compatibility anchors.
- [x] 5.5 If `macaca-cli -> macaca-web` cannot be removed safely, document the narrowed server-start-only edge in allowlist with an expiry condition.
- [x] 5.6 Add or update CLI tests/checks proving migrated inspection commands do not depend on Web internals.
- [x] 5.7 Run `cargo check -p macaca-cli` and `cargo test -p macaca-cli`.

## 6. Deprecated Field And Direct Consumer Guard

- [x] 6.1 Scan production uses of deprecated Web provider/runtime fields: `runtime`, `registry`, `llm`, `llm_router`, `memory_runtime`, `mcp_runtime`, `driver_registry`, and `driver_runtime`.
- [x] 6.2 Migrate low-risk reads to SDK clients or service-backed snapshots.
- [x] 6.3 For high-risk chat/framework/session/toolkit paths, keep compatibility fields and add English comments naming the future owner proposal or removal condition.
- [x] 6.4 Add or update guards/tests that block new presentation-owned semantic helpers and new callers of replaced direct paths.
- [x] 6.5 Ensure guards avoid false positives for lower-layer service provider implementations and explicit deprecated compatibility definitions.
- [x] 6.6 Run the deprecated-field scan command recorded in the S12 plan and summarize remaining anchors.

## 7. Governance And Dependency Gate

- [x] 7.1 Update `macaca/docs/route-c-architecture-governance.md` with S12 completion ownership rules for runtime-host bootstrap, SDK facade, Web/CLI adapters, and deprecated anchor discipline.
- [x] 7.2 Update `macaca/docs/route-c-serviceization-allowlist.md` only for direct edges actually removed or narrowed.
- [x] 7.3 Update `macaca/crates/macaca-integration-tests/tests/route_c_dependency_boundaries.rs` only if direct dependency edges or allowlist exceptions actually change.
- [x] 7.4 Run `cargo test -p macaca-integration-tests route_c_dependency_boundaries`.

## 8. Regression And Verification

- [x] 8.1 Run `openspec validate complete-web-cli-thin-shell-v1 --strict`.
- [x] 8.2 Run `cargo fmt --all --check`.
- [x] 8.3 Run `cargo check --workspace`.
- [x] 8.4 Run `cargo test -p macaca-runtime-host route_c_bootstrap`.
- [x] 8.5 Run `cargo test -p macaca-sdk`.
- [x] 8.6 Run `cargo test -p macaca-web`.
- [x] 8.7 Run `cargo check -p macaca-cli`.
- [x] 8.8 Run `cargo test -p macaca-cli`.
- [x] 8.9 Run `cargo test -p macaca-integration-tests --test route_c_baseline`.
- [x] 8.10 Run `cargo test -p macaca-integration-tests route_c_dependency_boundaries`.
- [x] 8.11 If frontend files change, run `cd frontend && npm run lint && npx tsc --noEmit`.
- [x] 8.12 Run hardcode scan over touched runtime-host, Web, CLI, and SDK files for app/workflow/provider/driver/gateway/model/chain/package/business-specific names.
- [x] 8.13 Run GitNexus `detect-changes` before commit and review affected scope.
