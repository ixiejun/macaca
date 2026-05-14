# Application-Owned UI Runtime Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first end-to-end Macaca application-owned web UI runtime so installed WASM applications can ship their own React/Vue/Svelte bundle while Macaca provides sandboxed hosting, bridge policy, trace, and audit.

**Architecture:** Add a provider-neutral `ui` manifest contract to `macaca-app`, project it into sanitized Application Service DTOs, expose a generic Web host surface, and route app-owned UI bridge calls through existing Application Service/WASM/service-router boundaries. The first slice uses iframe isolation in Web and keeps the protocol framework-neutral; React support is demonstrated by the independent `wasm-stock-agent-app`.

**Tech Stack:** Rust (`macaca-app`, `macaca-proto`, `macaca-web`, `macaca-runtime-host`), Axum, serde, TypeScript, Next.js 16, browser `postMessage`, standalone React/Vite-style static bundle artifacts.

---

## File Structure

- `macaca/crates/application/macaca-app/src/ui_runtime.rs`: new data-only manifest model and validation helpers for app-owned UI runtime declarations.
- `macaca/crates/application/macaca-app/src/model.rs`: attach optional `ui` declaration to `AppManifest`.
- `macaca/crates/application/macaca-app/src/service_admission.rs`: validate UI runtime declarations during manifest admission.
- `macaca/crates/application/macaca-app/src/service_projection.rs`: expose sanitized UI metadata through Application Service views.
- `macaca/crates/foundation/macaca-proto/src/application_service.rs`: add wire-safe UI runtime DTOs and bridge command DTO.
- `macaca/crates/shells/macaca-web/src/app_ui_routes.rs`: new generic API/static/bridge routes for app-owned UI bundles.
- `macaca/crates/shells/macaca-web/src/bootstrap.rs`: register UI runtime routes.
- `frontend/lib/app-ui-bridge.ts`: generic bridge client for iframe-hosted applications.
- `frontend/components/AppOwnedUiSurface.tsx`: generic iframe host surface, not application-specific.
- `frontend/app/chat/[appId]/page.tsx`: render the app-owned UI surface when manifest metadata declares one.
- `/Users/quantum/Code/dev/wasm-stock-agent-app/app.yaml`: declare `ui.runtime: web_bundle`.
- `/Users/quantum/Code/dev/wasm-stock-agent-app/ui/`: add a React-owned app surface that calls the Macaca bridge.
- `/Users/quantum/Code/dev/wasm-stock-agent-app/dist/ui/`: committed static bundle used by local install.

## Task 1: Manifest UI Contract

**Files:**
- Create: `macaca/crates/application/macaca-app/src/ui_runtime.rs`
- Modify: `macaca/crates/application/macaca-app/src/lib.rs`
- Modify: `macaca/crates/application/macaca-app/src/model.rs`
- Modify: `macaca/crates/application/macaca-app/src/loader.rs`
- Modify: `macaca/crates/application/macaca-app/src/service_admission.rs`

- [ ] **Step 1: Add failing manifest parser/admission tests**

Add tests that parse:

```yaml
name: ui-app
layer: L2Wasm
ui:
  runtime: web_bundle
  framework: react
  entry: dist/ui/index.html
  assets:
    - dist/ui/assets/**
  sandbox:
    isolation: iframe
    csp: strict
    network: declared
  bridge:
    required:
      - service.call
      - trace.emit
    optional:
      - session.read
  theme:
    mode: app_owned
```

Expected: parsed manifest has `ui.runtime == WebBundle`, bridge includes `service.call`, and unsafe entry `../escape.html` is rejected by admission.

- [ ] **Step 2: Implement `ui_runtime.rs`**

Define serde models:

```rust
AppUiRuntimeConfig
AppUiRuntimeKind
AppUiFramework
AppUiSandboxConfig
AppUiSandboxIsolation
AppUiCspMode
AppUiNetworkPolicy
AppUiBridgeConfig
AppUiThemeConfig
```

Add `validate_ui_runtime_config()` that rejects empty entry, absolute paths, `..`, empty bridge ids, unsupported runtime, unsupported isolation, and unsupported CSP mode.

- [ ] **Step 3: Wire manifest field and admission**

Add `pub ui: Option<AppUiRuntimeConfig>` to `AppManifest`, export the module, and invoke validation from `AppLoader::validate_manifest` and `ApplicationManifestSpec::validate`.

- [ ] **Step 4: Verify**

Run:

```bash
cargo test -p macaca-app ui_runtime -- --nocapture
cargo test -p macaca-app parse_manifest_with_ui_runtime_block -- --nocapture
```

Expected: both pass.

## Task 2: Sanitized UI Metadata And App Static Routes

**Files:**
- Modify: `macaca/crates/foundation/macaca-proto/src/application_service.rs`
- Modify: `macaca/crates/application/macaca-app/src/service_projection.rs`
- Create: `macaca/crates/shells/macaca-web/src/app_ui_routes.rs`
- Modify: `macaca/crates/shells/macaca-web/src/bootstrap.rs`
- Modify: `macaca/crates/shells/macaca-web/src/lib.rs`
- Modify: `macaca/crates/shells/macaca-web/src/routes.rs`

- [ ] **Step 1: Add DTO tests by compiling route response**

Add `ApplicationUiRuntimeView` to `ApplicationServiceAppView` and expose it through `AppInfo`.

- [ ] **Step 2: Project UI metadata**

Project only sanitized, bounded fields:

```rust
runtime, framework, entry_url, sandbox, bridge.required, bridge.optional, theme.mode
```

Never expose raw filesystem paths directly to the frontend; expose URLs like `/api/apps/{id}/ui/assets/index.html`.

- [ ] **Step 3: Add static route**

Add `GET /api/apps/{app_id}/ui/assets/*path` that resolves the app's installed package directory from Application Service metadata, verifies the requested path is within declared `entry` or `assets`, and returns the static file with conservative content type.

- [ ] **Step 4: Verify**

Run:

```bash
cargo test -p macaca-web app_ui -- --nocapture
cargo build -p macaca-cli --bin macaca
```

Expected: tests and build pass.

## Task 3: UI Bridge Route And Audit Envelope

**Files:**
- Modify: `macaca/crates/foundation/macaca-proto/src/application_service.rs`
- Modify: `macaca/crates/shells/macaca-web/src/app_ui_routes.rs`
- Modify: `macaca/crates/shells/macaca-web/src/bootstrap.rs`

- [ ] **Step 1: Define bridge request/response DTOs**

Use a generic JSON payload shape:

```json
{
  "bridge_version": "macaca.ui.bridge.v1",
  "session_id": "session-id",
  "surface_id": "chat",
  "trace_id": "trace-id",
  "command_id": "command-id",
  "capability": "service.call",
  "operation": "finance.lookup",
  "payload": {}
}
```

- [ ] **Step 2: Enforce declared bridge capability**

Bridge route denies calls if `capability` is missing from `ui.bridge.required + optional`.

- [ ] **Step 3: Route service calls**

For first slice, support `service.call` by building an `ApplicationHostCommand` with `ApplicationImport::ServiceCall` metadata and dispatching it through `state.application_client.host_dispatch`.

- [ ] **Step 4: Verify**

Add route tests for allowed and denied bridge capabilities. Run:

```bash
cargo test -p macaca-web app_ui_bridge -- --nocapture
```

Expected: allowed call reaches host dispatch mock or returns structured runtime unavailable; denied call fails before routing.

## Task 4: Generic Frontend Host And Bridge Client

**Files:**
- Modify: `frontend/lib/types.ts`
- Modify: `frontend/lib/api.ts`
- Create: `frontend/lib/app-ui-bridge.ts`
- Create: `frontend/components/AppOwnedUiSurface.tsx`
- Modify: `frontend/app/chat/[appId]/page.tsx`
- Modify: `frontend/app/globals.css`

- [ ] **Step 1: Add typed UI metadata**

Add optional `ui?: AppUiRuntimeInfo` to `AppInfo`.

- [ ] **Step 2: Implement bridge client**

Implement `createMacacaAppBridge()` that listens for iframe messages, validates origin/source, forwards `macaca.call` to `/api/apps/{id}/ui/bridge`, and posts `macaca.result`.

- [ ] **Step 3: Implement generic iframe host**

`AppOwnedUiSurface` accepts `app`, `sessionId`, and `className`, renders a sandboxed iframe using `app.ui.entry_url`, and attaches the bridge client.

- [ ] **Step 4: Integrate with chat page**

When `currentApp.ui.runtime === 'web_bundle'`, show the app-owned surface in the main workspace. Existing chat stream remains available for fallback and audit.

- [ ] **Step 5: Verify**

Run:

```bash
npm run lint
npm run build
```

Expected: both pass.

## Task 5: Standalone Stock App React Bundle

**Files:**
- Modify: `/Users/quantum/Code/dev/wasm-stock-agent-app/app.yaml`
- Modify: `/Users/quantum/Code/dev/wasm-stock-agent-app/dist/app.yaml`
- Create: `/Users/quantum/Code/dev/wasm-stock-agent-app/ui/index.html`
- Create: `/Users/quantum/Code/dev/wasm-stock-agent-app/ui/src/main.jsx`
- Create: `/Users/quantum/Code/dev/wasm-stock-agent-app/dist/ui/index.html`
- Create: `/Users/quantum/Code/dev/wasm-stock-agent-app/dist/ui/assets/app.js`
- Modify: `/Users/quantum/Code/dev/wasm-stock-agent-app/README.md`

- [ ] **Step 1: Add manifest UI declaration**

Declare:

```yaml
ui:
  runtime: web_bundle
  framework: react
  entry: dist/ui/index.html
  assets:
    - dist/ui/assets/**
  sandbox:
    isolation: iframe
    csp: strict
    network: declared
  bridge:
    required:
      - service.call
      - trace.emit
      - session.read
    optional:
      - storage.kv
      - theme.read
  theme:
    mode: app_owned
```

- [ ] **Step 2: Add app-owned UI source and static bundle**

The static demo UI should use its own CSS/brand, call `window.parent.postMessage({ type: "macaca.call", capability: "service.call", ... })`, and render returned data. It must not import Macaca internals.

- [ ] **Step 3: Verify**

Run:

```bash
cargo test --workspace
```

Expected: existing stock app tests pass.

## Task 6: End-To-End Verification And Commit

**Files:**
- All files above.

- [ ] **Step 1: Run backend verification**

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-app ui_runtime -- --nocapture
cargo test -p macaca-web app_ui -- --nocapture
cargo build -p macaca-cli --bin macaca
```

- [ ] **Step 2: Run frontend verification**

```bash
cd /Users/quantum/Code/dev/agent/frontend
npm run lint
npm run build
```

- [ ] **Step 3: Run stock app verification**

```bash
cd /Users/quantum/Code/dev/wasm-stock-agent-app
cargo test --workspace
```

- [ ] **Step 4: Run OpenSpec validation**

```bash
cd /Users/quantum/Code/dev/agent
openspec validate add-application-owned-ui-runtime --strict
```

- [ ] **Step 5: Commit**

Create separate commits for Macaca runtime/frontend and standalone app if both repositories changed.

