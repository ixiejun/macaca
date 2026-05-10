# Change: Add Application Framework Service v1

## Why

Route C S7 requires Application Framework lifecycle to move out of `macaca-web` direct orchestration and into a provider-neutral Application Service. Today Web discovers, starts, and interprets YAML applications directly, while `AppRuntime` registers agents through kernel-facing compatibility paths; this keeps Web as a macro-coordinator and blocks the Web/CLI thin-shell target.

## What Changes

- Add an Application Service contract in `macaca-app` for discover/load/start/stop/remove/status/snapshot/session envelope/host dispatch/GenUI surface commands.
- Add runtime-host Application Service provider wiring that adapts existing `AppRegistry`, `AppRuntime`, `ApplicationHost`, ABI adapters, and lifecycle state without moving Application Framework semantics into runtime-host.
- Add SDK `SystemApplicationClient` and `SystemFacade` accessors so upper shells use a focused client instead of direct application runtime/registry access.
- Migrate Web startup, app routes, `/api/chat/v2` preflight, and GenUI surface lookup to prefer Application Service while preserving current response shapes and execution paths.
- Preserve existing direct `AppRuntime`, `AppLoader`, `AppRegistry`, and Web application state as deprecated compatibility anchors; do not delete them in this change.
- Keep WASM execution metadata-only and return structured runtime-unavailable rather than panic or hidden success.
- Update Route C governance and allowlist notes for Application Service ownership and remaining compatibility debt.

## Impact

- Affected specs: `application-service`, `application-runtime-host-provider`, `application-sdk-client`, `application-web-adapter`
- Affected code:
  - `macaca/crates/macaca-app/src/service_contract.rs`
  - `macaca/crates/macaca-app/src/service_adapter.rs`
  - `macaca/crates/macaca-app/src/service_admission.rs`
  - `macaca/crates/macaca-app/src/runtime.rs`
  - `macaca/crates/macaca-app/src/abi.rs`
  - `macaca/crates/macaca-app/src/lifecycle.rs`
  - `macaca/crates/macaca-runtime-host/src/application_service_provider.rs`
  - `macaca/crates/macaca-sdk/src/application_client.rs`
  - `macaca/crates/macaca-sdk/src/system_facade.rs`
  - `macaca/crates/macaca-web/src/lib.rs`
  - `macaca/crates/macaca-web/src/state.rs`
  - `macaca/crates/macaca-web/src/routes.rs`
  - `macaca/crates/macaca-web/src/chat_orchestrator.rs`
  - `macaca/crates/macaca-web/src/genui_routes.rs`
  - Route C governance and dependency-boundary docs/tests as needed
- Regression focus: RC-APP-001, RC-CHAT-001, RC-GOAL-001, RC-TRACE-001, RC-DRIVER-001, RC-SKILL-001

