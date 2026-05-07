# Change: Add GenUI Runtime v0

## Why

Route C Phase 06 must make UI a first-class Application Framework capability rather than a hardcoded chat-only presentation. Applications need to declare controlled UI intent and component trees through the Application ABI, while the Web Shell renders them under system trace, permission, approval, and recovery boundaries.

Without GenUI Runtime v0, application-specific interfaces would either be hardcoded into the frontend or bypass system observability. That would violate the microkernel boundary, make UI events untraceable, and block future Store, paid application, approval, desktop/mobile renderer, and non-chat application flows.

## What Changes

- Add provider-neutral UI protocol contracts in `macaca-proto` for `UiIntent`, `UiComponentTree`, `UiComponent`, `UiEvent`, `UiAction`, `UiBinding`, `UiPermissionPrompt`, and `UiTraceMarker`.
- Add `macaca-app` GenUI runtime API that lets applications emit `UiIntent` through the existing Application ABI / `ApplicationHost` boundary without exposing internal web state.
- Add a Web Shell GenUI renderer boundary in `macaca-web` and frontend that renders controlled declarative component trees while preserving the existing chat/trace shell as default.
- Add UI event feedback APIs so button clicks and form submissions become traced `UiEventCommand` records and flow back to the session/application boundary.
- Add trace overlay and approval/prompt decoration as system wrappers around application UI rather than as application-owned privileged UI.
- Add structured logs and trace/audit records for UI intent emission, renderer selection, unsupported components, UI events, permission prompts, approval prompts, and EventLog persistence.
- Require detailed English comments in all new Rust and TypeScript/TSX code explaining GenUI schema invariants, renderer operation, trace/audit behavior, event flow, unsupported component handling, and non-goals.

## Impact

- Affected specs: `genui-runtime-v0`
- Affected crates/apps: `macaca-proto`, `macaca-app`, `macaca-web`, `frontend`
- Affected code: `macaca-proto/src/ui.rs`, `macaca-app/src/genui.rs`, `macaca-app/src/host.rs`, `macaca-web/src/genui_routes.rs`, `macaca-web/src/routes.rs`, frontend GenUI renderer/lib/components
- Affected tests: UI serde tests, app GenUI tests, web route tests, frontend renderer tests/lint/typecheck
- Regression matrix references: `RC-TRACE-001`, `RC-TRACE-002`, `RC-RECOVERY-001`

## Governance Alignment

- Follows `macaca/docs/agent-os-microkernel-boundaries.md`: GenUI belongs to Application Framework / UI Service / Presentation Shell, not kernel; Web Shell renders and adapts but does not define session/task/trace semantics.
- Follows `macaca/docs/route-c-regression-matrix.md`: Phase 06 must preserve real-time trace, historical trace replay, and restart/recovery behavior.
- Follows `macaca/docs/route-c-phase-template.md`: Superpowers brainstorm, OpenSpec proposal/design/tasks/spec, GitNexus impact before symbol edits, additive-first implementation, targeted tests, integration smoke, detect_changes before commit.
- Follows `macaca/docs/route-c-architecture-governance.md`: UI events must carry trace, privileged prompts must be policy-ready, unsupported components must be structured, and no app/provider/driver/gateway/chain hardcoding is allowed.

## Non-Goals

- Do not execute arbitrary remote JavaScript, remote React components, inline scripts, or untrusted UI code.
- Do not replace the existing chat shell, trace viewer, task board, or session log UI.
- Do not implement a full desktop/mobile renderer; keep renderer contracts strategy-ready.
- Do not implement Store, entitlement, payment settlement, Web3, EVM, or paid UI enforcement beyond inert approval/prompt schema.
- Do not hardcode application-specific UI layouts, app names, workflow names, provider names, driver names, gateway names, model names, chain names, or business routing.
- Do not allow UI events without trace context or session/application scope.
