## 1. Preparation

- [x] 1.1 Read `macaca/docs/agent-os-microkernel-boundaries.md`, `macaca/docs/route-c-regression-matrix.md`, `macaca/docs/route-c-phase-template.md`, `macaca/docs/route-c-architecture-governance.md`, and `docs/superpowers/plans/route-c-microkernel-ecosystem/phase-06-genui-runtime-v0.md`.
- [x] 1.2 Review existing Application ABI v0 code in `macaca-proto`, `macaca-app`, and `macaca-sdk`.
- [x] 1.3 Review current `macaca-web` routes, EventLog trace paths, session replay paths, and frontend chat/trace shell components.
- [x] 1.4 Run GitNexus impact before modifying each selected symbol; warn before editing any HIGH or CRITICAL impact symbol.

## 2. UI Protocol Contracts

- [x] 2.1 Add `macaca/crates/macaca-proto/src/ui.rs` with UI schema v0 value objects and data contracts.
- [x] 2.2 Export UI contracts from `macaca/crates/macaca-proto/src/lib.rs`.
- [x] 2.3 Define `UiIntent`, `UiComponentTree`, `UiComponent`, `UiEvent`, `UiEventCommand`, `UiAction`, `UiBinding`, `UiPermissionPrompt`, `UiApprovalPrompt`, and `UiTraceMarker`.
- [x] 2.4 Support component kinds for text, markdown, form, button, table, card, list, chart placeholder, trace panel mount, approval prompt, and structured custom/unsupported components.
- [x] 2.5 Add serde roundtrip tests for intent, component tree, component, event command, binding, prompt, trace marker, and structured error payloads.
- [x] 2.6 Add tests proving unknown component kinds remain structured and render/validate as unsupported instead of panicking.

## 3. Application GenUI API

- [x] 3.1 Add `macaca/crates/macaca-app/src/genui.rs` with GenUI runtime/facade, validator, and UI intent builder helpers.
- [x] 3.2 Integrate `macaca:ui/render` with `ApplicationHost` or a pluggable GenUI host backend without exposing `macaca-web` internals.
- [x] 3.3 Require app id, session id, and trace context for UI intent emission and UI event commands.
- [x] 3.4 Validate component trees with Visitor/Specification-style traversal for supported components, safe bindings, trace markers, and no raw script payloads.
- [x] 3.5 Return structured unsupported/unavailable errors for unsupported components or missing renderer/service backends.
- [x] 3.6 Add structured logs for intent build, validation start/pass/reject, host render dispatch, and unsupported component decisions.
- [x] 3.7 Add `cargo test -p macaca-app genui` coverage using fixture intents with app/session trace context.

## 4. Web Shell Renderer Boundary

- [x] 4.1 Add `macaca/crates/macaca-web/src/genui_routes.rs` as a thin route adapter for GenUI surfaces and UI events.
- [x] 4.2 Register GenUI routes from existing router setup without making `routes.rs` a giant file.
- [x] 4.3 Persist UI intent and UI event records to EventLog or equivalent trace path with app id, session id, trace id, surface id, component id, event id, and action.
- [x] 4.4 Return structured errors for missing session/application/trace scope, unsupported components, unavailable app surface, or disabled renderer.
- [x] 4.5 Ensure applications without custom UI continue using the existing chat shell.
- [x] 4.6 Add route tests proving UI events are trace persisted and missing scope is rejected.

## 5. Frontend GenUI Renderer

- [x] 5.1 Add `frontend/lib/genui.ts` with TypeScript schema types and API helpers.
- [x] 5.2 Add `frontend/components/genui/GenUiRenderer.tsx` that renders controlled component trees through a generic visitor/strategy dispatch.
- [x] 5.3 Add `frontend/components/genui/TraceOverlay.tsx` that decorates UI with trace markers, permission prompts, and approval prompts.
- [x] 5.4 Add frontend integration so an app with GenUI surface can mount the renderer while no-surface apps continue to show the chat shell.
- [x] 5.5 Ensure unsupported components render bounded unsupported placeholders and never execute remote JS or raw script payloads.
- [x] 5.6 Ensure button clicks and form submissions produce traced UI event commands through API helpers.
- [x] 5.7 Add `frontend/components/genui/GenUiRenderer.test.tsx` scenario coverage for supported components, unsupported components, trace overlay, and event emission. The current frontend package has no executable test runner, so this is documented scenario coverage plus lint/typecheck validation.

## 6. Regression And Verification

- [x] 6.1 Run `openspec validate add-genui-runtime-v0 --strict`.
- [x] 6.2 Run `cargo test -p macaca-proto ui`.
- [x] 6.3 Run `cargo test -p macaca-app genui`.
- [x] 6.4 Run targeted `macaca-web` GenUI route tests.
- [x] 6.5 Run `cargo test -p macaca-integration-tests --test route_c_baseline`.
- [x] 6.6 Run `cargo check -p macaca-web`.
- [x] 6.7 Run `cargo check --workspace`.
- [x] 6.8 Run `cd frontend && npm run lint && npx tsc --noEmit`.
- [x] 6.9 Run frontend GenUI renderer tests if the existing frontend test runner is available; otherwise document the missing runner.
- [x] 6.10 Run a hardcode scan over new GenUI files for demo app names, workflow names, provider names, driver names, gateway names, model names, chain names, and business-specific routing.
- [x] 6.11 Run `npx gitnexus detect-changes --repo agent` before committing and verify affected flows match the expected Phase 06 scope.
