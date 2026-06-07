## 1. Preparation

- [x] 1.1 Read `macaca/docs/agent-os-microkernel-boundaries.md`, `macaca/docs/route-c-regression-matrix.md`, `macaca/docs/route-c-phase-template.md`, `macaca/docs/route-c-architecture-governance.md`, `docs/superpowers/plans/2026-05-07-macaca-os-route-c-microkernel-ecosystem-plan.md`, and `docs/superpowers/plans/route-c-microkernel-ecosystem/phase-12-web-cli-thin-shell.md`.
- [x] 1.2 Review current SDK facade, Web `route_command`, Web session/task/trace routes, GenUI routes, SSE handling, CLI command handlers, and CLI command implementations.
- [x] 1.3 Run GitNexus impact before modifying each selected existing symbol; warn before editing any HIGH or CRITICAL impact symbol.

## 2. SDK System Facade

- [x] 2.1 Add `macaca/crates/macaca-sdk/src/system_facade.rs`.
- [x] 2.2 Define typed shell-facing commands/results for task board query, session event query, trace tail intent, service inspection, package inspection, and approval decision.
- [x] 2.3 Add facade traits/adapters that can be backed by current stores/kernel services now and service bus later.
- [x] 2.4 Export the facade from `macaca/crates/macaca-sdk/src/lib.rs`.
- [x] 2.5 Add tests proving commands are typed, replay-safe, and do not depend on `macaca-web`.

## 3. Web Route Command Adapter

- [x] 3.1 Add or extend `macaca/crates/macaca-web/src/shell.rs` as the Web Shell command adapter boundary.
- [x] 3.2 Migrate one low-risk read-only route, preferably task board/session events, to call the SDK system facade while preserving response JSON shape.
- [x] 3.3 Add structured logs for request scope validation, command construction, facade call, success, and rejection.
- [x] 3.4 Add tests proving the migrated route remains session-scoped and response-compatible.

## 4. Trace/SSE Thin Shell

- [x] 4.1 Define trace/SSE shell rules where Web subscribes to trace/event sources and forwards presentation data without redefining trace semantics.
- [x] 4.2 Ensure replay cursors and live events remain session-scoped.
- [x] 4.3 Add tests or smoke checks for no duplicate historical/live events where feasible.
- [x] 4.4 Mark any replaced direct trace semantic helper as deprecated or compatibility-only.

## 5. GenUI and Frontend Shell Guardrails

- [x] 5.1 Document or implement frontend shell mount guardrails for generic GenUI surfaces.
- [x] 5.2 Preserve chat/trace shell fallback when no GenUI surface exists.
- [x] 5.3 Ensure rendering dispatch is based on schema/component/event kind rather than application-specific names.
- [x] 5.4 Run frontend lint/typecheck if frontend files are changed.

## 6. CLI Facade Migration

- [x] 6.1 Update CLI command handlers or command implementations to delegate read-only system inspection paths through the SDK system facade.
- [x] 6.2 Keep CLI responsible only for terminal parsing, output formatting, process lifecycle, and Web server startup.
- [x] 6.3 Keep deprecated compatibility helpers present until all consumers migrate.
- [x] 6.4 Add tests or checks proving CLI does not depend on Web internals for migrated inspection commands.

## 7. Deprecation and Migration Guards

- [x] 7.1 Mark replaced direct Web/CLI semantic helpers as deprecated or compatibility-only.
- [x] 7.2 Add or update guard scripts/tests to prevent new callers from using deprecated presentation-owned semantic paths.
- [x] 7.3 Ensure guards avoid false positives for lower-layer service/kernel/application APIs.

## 8. Regression and Verification

- [x] 8.1 Run `openspec validate add-web-cli-thin-shell-v0 --strict`.
- [x] 8.2 Run `cargo test -p macaca-sdk`.
- [x] 8.3 Run `cargo test -p macaca-web`.
- [x] 8.4 Run `cargo check -p macaca-cli`.
- [x] 8.5 Run `cargo test -p macaca-integration-tests --test route_c_baseline`.
- [x] 8.6 Run frontend lint/typecheck if frontend files change.
- [x] 8.7 Run hardcode scan over new shell/facade/frontend files for app/workflow/provider/driver/gateway/model/chain/package/business constants.
- [x] 8.8 Run `npx gitnexus detect-changes --repo agent` before committing and verify affected flows align with Phase 12 scope.
