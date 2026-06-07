# Design: GenUI Runtime v0

## Context

Phase 05 introduced Application ABI v0 and an `ApplicationHost` facade with a declared `macaca:ui/render` host import. Phase 06 builds on that boundary by introducing controlled declarative UI contracts and a Web Shell renderer. GenUI is not a chat widget replacement; it is the Application Framework's UI capability surface.

Route C governance requires GenUI to stay out of the kernel, remain additive, preserve existing chat/trace behavior, and make UI events traceable. The implementation must treat frontend rendering as a presentation strategy, not as the owner of system semantics.

## Goals

- Define UI schema v0 in `macaca-proto` as data-only contracts.
- Support controlled component types: text, markdown, form, button, table, card, list, chart placeholder, trace panel mount, and approval prompt.
- Provide `macaca-app` GenUI APIs for applications to emit `UiIntent` and `UiComponentTree` through Application ABI / `ApplicationHost`.
- Add a Web Shell renderer boundary that can mount GenUI surfaces without replacing the default chat shell.
- Convert user interactions into traced `UiEventCommand` requests that flow back to application/session boundaries.
- Persist UI intent/event records into EventLog or equivalent trace path so real-time and historical trace remain consistent.
- Keep renderer logic generic: no application-specific layouts or business rules in frontend.
- Add logs and trace/audit markers at critical execution nodes.

## Non-Goals

- No arbitrary remote JS execution.
- No WASM UI code execution.
- No GenUI takeover of existing chat/trace shell.
- No payment settlement or entitlement enforcement.
- No desktop/mobile renderer implementation beyond strategy-ready contracts.
- No application-specific frontend pages.

## Superpowers Brainstorm Summary

### Current Problem

Macaca can run chat-oriented applications, but non-chat applications need first-class UI intent, controlled rendering, and traced user interaction. Without a declarative UI contract, teams will inevitably hardcode application UI in frontend or pass opaque HTML/JS around, both of which break auditability and OS boundaries.

### Why Phase 06 Must Solve It

Phase 05 already gives applications an ABI and a host import for UI rendering. Phase 06 must define what that import means before later Store, plugin, approval, payment, desktop/mobile, and paid application phases depend on UI surfaces.

### Options Considered

1. **Controlled declarative schema with Web Shell renderer.**
   - Pros: safe by default, traceable, renderer-strategy-ready, preserves chat shell, supports future CLI/desktop/mobile renderers.
   - Cons: limited component expressiveness in v0.
   - Verdict: recommended.

2. **Allow applications to ship arbitrary React/JS UI.**
   - Pros: maximum UI flexibility.
   - Cons: security risk, untraceable behavior, hard to audit/policy-wrap, violates Phase 06 non-goals.
   - Verdict: rejected.

3. **Hardcode custom UI per application in frontend.**
   - Pros: fastest for demos.
   - Cons: destroys platform extensibility and violates no application-specific hardcoding.
   - Verdict: rejected.

4. **Defer UI event feedback until later.**
   - Pros: smaller initial renderer.
   - Cons: UI would be display-only and not a real application surface.
   - Verdict: rejected; v0 must include traced event feedback.

## Recommended Plan

Implement GenUI additively: first UI protocol contracts, then application GenUI API, then web route boundary, then frontend renderer, then UI event feedback. Existing applications without GenUI metadata continue using the chat shell.

## Design Patterns

- **Composite**: `UiComponentTree` and nested `UiComponent` represent controlled UI structure.
- **Visitor**: backend validators and frontend renderers traverse component trees for rendering, unsupported-component detection, trace marker injection, and future policy checks.
- **Command**: user interactions become `UiEventCommand` objects with trace/session/application scope.
- **Decorator**: trace overlays, permission prompts, and approval prompts wrap application UI without giving applications privileged UI control.
- **Strategy**: Web renderer is one renderer strategy; CLI, desktop, and mobile can implement the same schema later.
- **Facade**: `ApplicationHost` exposes UI render capability while hiding EventLog, route, and frontend details.
- **Specification**: schema validation, supported component validation, trace requirements, and prompt constraints are explicit rules.
- **Observer**: UI intent/event lifecycle emits trace/audit/log events.
- **Null Object**: unsupported components render structured unsupported placeholders instead of panics or blank UI.

## Protocol Shape

### `macaca-proto/src/ui.rs`

The protocol module should define:

- `UiIntent`
- `UiComponentTree`
- `UiComponent`
- `UiComponentKind`
- `UiEvent`
- `UiEventCommand`
- `UiAction`
- `UiBinding`
- `UiPermissionPrompt`
- `UiApprovalPrompt`
- `UiTraceMarker`
- `UiRenderSurface`
- `UiRenderError`

The schema must be serde-friendly and extensible through `Custom(String)` variants or metadata maps where appropriate. Unknown component kinds must remain structured data and render as unsupported placeholders.

## Application Framework Shape

### `macaca-app/src/genui.rs`

Application Framework should provide:

- `GenUiRuntime` or equivalent facade for building/validating `UiIntent`.
- `GenUiHostBackend` or integration point with `ApplicationHost` for `macaca:ui/render`.
- `UiIntentValidator` that enforces trace context, supported components, safe bindings, and no raw script payloads.
- Tests proving fixture intents carry app/session trace context and unsupported components are structured.

The GenUI API must not depend on `macaca-web`, frontend component code, or concrete application names.

## Web Shell Shape

### `macaca-web/src/genui_routes.rs`

The web layer should expose thin adapter routes such as:

- `GET /api/apps/{app_id}/genui/surface?session_id=...`
- `POST /api/apps/{app_id}/genui/events`

Exact route names can be adjusted to existing router conventions, but routes must:

- require session/application scope;
- require trace context or create a traceable command envelope;
- persist UI intent/event records into EventLog or equivalent trace path;
- return structured unsupported/unavailable errors;
- avoid owning application UI semantics.

## Frontend Shape

### `frontend/lib/genui.ts`

The frontend library should define TypeScript types matching UI schema v0 and API helpers for fetching surfaces and posting UI events.

### `frontend/components/genui/GenUiRenderer.tsx`

The renderer should:

- render supported component types generically;
- use a visitor-like dispatch over component kind;
- render unsupported components as bounded unsupported placeholders;
- never evaluate scripts or raw JS;
- emit traced UI event commands on user interaction;
- stay visually subordinate to existing shell unless an app explicitly provides a GenUI surface.

### `frontend/components/genui/TraceOverlay.tsx`

The trace overlay should decorate rendered UI with system trace markers and approval/prompt state without letting application UI forge privileged system UI.

## Trace, Audit, And Logging

Phase 06 implementation must log and trace:

- UI intent emitted;
- UI schema validation started/passed/rejected;
- renderer selected;
- unsupported component encountered;
- trace overlay applied;
- permission/approval prompt rendered;
- UI event command created;
- UI event persisted;
- UI event dispatched to application/session boundary;
- UI event rejected due to missing trace/session/app scope.

Trace/log payloads should include app id, session id, trace id, surface id, component id, event id, action name, component kind, renderer strategy, structured status, and structured error code. Payloads must not include secrets, private keys, provider credentials, raw payment credentials, or unbounded user input.

## Compatibility And Regression

GenUI must preserve:

- `RC-TRACE-001`: real-time trace updates still appear without refresh.
- `RC-TRACE-002`: historical trace replay remains complete and non-duplicated.
- `RC-RECOVERY-001`: restart/recovery loads historical data and continues live increments.

Applications without GenUI surface declarations must continue to display the current chat shell.

## Risks / Trade-offs

- **Risk: schema too limited for application UX.** Mitigation: v0 uses extensible metadata/custom variants but renders unknown components safely.
- **Risk: frontend becomes application-specific.** Mitigation: renderer dispatches by schema kind only; hardcode scan rejects app/business names.
- **Risk: UI events bypass trace.** Mitigation: route/API and host boundary reject missing session/application/trace scope.
- **Risk: approval prompts can be spoofed by apps.** Mitigation: system prompt/approval components are decorated by shell and carry system trace markers.
- **Risk: GenUI breaks chat shell.** Mitigation: mount GenUI as optional surface; no-surface fallback remains current chat UI.

## Open Questions

- None blocking Phase 06. Desktop/mobile renderers, arbitrary third-party UI code, Store entitlement, payment settlement, and richer component libraries belong to later phases.
