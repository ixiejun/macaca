## 1. Specification and Design

- [x] 1.1 Review existing GenUI, application UI runtime, WASM host dispatch, and service-call audit specs for conflicts.
- [x] 1.2 Validate this change with `openspec validate add-session-genui-crypto-signal-app --strict`.
- [x] 1.3 Keep implementation scoped to generic platform behavior and the independent crypto app repository.

## 2. Generic Session GenUI Platform

- [x] 2.1 Add `builtin_kit` as a data-only UI runtime declaration for session-surface apps.
- [x] 2.2 Add tests for manifest parsing/admission of `ui.runtime: builtin_kit`, `surface.mode: session`, and host-owned chrome.
- [x] 2.3 Add a generic render-surface repository behind Application Service keyed by app id, session id, and surface id.
- [x] 2.4 Route `ApplicationImport::UiRender` through the WASM host boundary without using app-specific service ids.
- [x] 2.5 Implement `APPLICATION_GENUI_SURFACE_COMMAND` so Web shells can query the latest session GenUI surface.
- [x] 2.6 Preserve structured unavailable behavior when no surface exists.

## 3. Web Shell Integration

- [x] 3.1 Ensure `/api/chat/v2` keeps normalized `session_id`, `thinking`, `assistant`, `done`, and `error` SSE events for WASM session apps.
- [x] 3.2 Ensure the chat page refreshes the generic GenUI surface after session execution without branching on crypto app identity.
- [x] 3.3 Verify `GenUiRenderer` handles the crypto card/tree shape with existing generic component strategies.

## 4. Independent Crypto App Repository

- [x] 4.1 Create `/Users/quantum/Code/dev/wasm-crypto-signal-app` with a Rust workspace modeled after the stock app boundary.
- [x] 4.2 Add manifest/app.yaml with `layer: L2Wasm`, `ui_type: chat`, `service_contract.use_packs: [pack.finance.v1]`, and session `builtin_kit` UI declaration.
- [x] 4.3 Implement crypto symbol normalization and deterministic signal/risk data structures.
- [x] 4.4 Embed component metadata markers: `macaca:component-model:v1`, `export=app:start`, WIT package marker, service-call host commands, and a `ui.render` host command.
- [x] 4.5 Add packaging scripts and install layout for `/Users/quantum/.macaca/workspaces/apps/wasm-crypto-signal-app`.

## 5. Verification

- [x] 5.1 Run targeted Rust tests for `macaca-app`, `macaca-runtime-host`, and `macaca-web`.
- [x] 5.2 Build the crypto app WASM artifact and verify markers with `strings dist/component.wasm`.
- [x] 5.3 Install the crypto app into the Macaca workspace app directory.
- [x] 5.4 Verify declared host-command result chaining feeds market/news outputs into analysis and analysis output into `ui.render`.
- [x] 5.5 Start backend and frontend, open the Web UI, and run a real session such as `分析 BTC 买卖信号`.
- [x] 5.6 Verify the visible result preserves main thread, bottom composer, AgentPanel, trace/audit evidence, and a GenUI card-style analysis surface.
- [x] 5.7 Run `gitnexus_detect_changes(scope: "all")` before any commit or final integration request.
