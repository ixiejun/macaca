# Session GenUI Crypto Signal App Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a standalone crypto signal WASM app that keeps Macaca's main session thread and renders card-style analysis through the built-in GenUI kit.

**Architecture:** Macaca gets only generic session GenUI support: manifest `builtin_kit` admission, `ApplicationImport::UiRender` storage, and `APPLICATION_GENUI_SURFACE_COMMAND` replay. The crypto app lives in `/Users/quantum/Code/dev/wasm-crypto-signal-app`, declares crypto service contracts, emits deterministic host-command metadata, and never owns provider secrets or direct networking.

**Tech Stack:** Rust 2021, Macaca Rust workspace, OpenSpec, Next.js/React GenUI renderer, WASM `wasm32-unknown-unknown`, existing portable component metadata adapter.

---

## File Structure

- Modify: `/Users/quantum/Code/dev/agent/macaca/crates/application/macaca-app/src/ui_runtime.rs`
  - Add `AppUiRuntimeKind::BuiltinKit`, validate it without `entry`, and document session-only semantics.
- Modify: `/Users/quantum/Code/dev/agent/macaca/crates/application/macaca-app/src/manifest_v1/yaml_adapter.rs`
  - Ensure sanitized metadata projects `builtin_kit` UI runtime and session surface.
- Modify: `/Users/quantum/Code/dev/agent/macaca/crates/runtime/macaca-runtime-host/src/application_service_provider.rs`
  - Add a small in-memory GenUI surface repository and implement `APPLICATION_GENUI_SURFACE_COMMAND`.
- Modify: `/Users/quantum/Code/dev/agent/macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/host_import_bridge.rs`
  - Route `ApplicationImport::UiRender` to a generic render sink without changing service-call policy.
- Modify: `/Users/quantum/Code/dev/agent/macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/component_model.rs`
  - Preserve render command dispatch in declared host-command plans.
- Modify: `/Users/quantum/Code/dev/agent/frontend/app/chat/[appId]/page.tsx`
  - Refresh the existing generic GenUI surface after WASM session completion, without app-specific checks.
- Create: `/Users/quantum/Code/dev/wasm-crypto-signal-app`
  - Standalone Rust workspace, app manifest, guest crate, contract crate, packaging crate, harness, scripts, tests, and audit artifacts.

## Task 1: Confirm Proposal and Impact Boundaries

**Files:**
- Read: `/Users/quantum/Code/dev/agent/openspec/changes/add-session-genui-crypto-signal-app/proposal.md`
- Read: `/Users/quantum/Code/dev/agent/openspec/changes/add-session-genui-crypto-signal-app/design.md`
- Read: `/Users/quantum/Code/dev/agent/openspec/changes/add-session-genui-crypto-signal-app/tasks.md`

- [ ] **Step 1: Validate OpenSpec change**

Run:

```bash
openspec validate add-session-genui-crypto-signal-app --strict
```

Expected: `Change 'add-session-genui-crypto-signal-app' is valid`

- [ ] **Step 2: Run impact checks before code edits**

Run GitNexus impact before editing each target symbol:

```text
impact(target: "AppUiRuntimeKind", direction: "upstream")
impact(target: "validate_ui_runtime_config", direction: "upstream")
impact(target: "ApplicationSystemServiceProvider", direction: "upstream")
impact(target: "WasmHostImportBridge", direction: "upstream")
impact(target: "ComponentModelWasmExecutionSession", direction: "upstream")
```

Expected: Identify direct callers and warn before proceeding if any check is HIGH or CRITICAL.

## Task 2: Add Builtin Kit Manifest Admission

**Files:**
- Modify: `/Users/quantum/Code/dev/agent/macaca/crates/application/macaca-app/src/ui_runtime.rs`
- Test: `/Users/quantum/Code/dev/agent/macaca/crates/application/macaca-app/src/ui_runtime.rs`

- [ ] **Step 1: Write failing tests**

Add tests that parse this YAML:

```yaml
runtime: builtin_kit
surface:
  mode: session
  chrome: host
presentation:
  schema: genui.v1
  preferred_components:
    - card
    - table
    - list
```

Expected assertions:

```rust
assert_eq!(config.runtime, AppUiRuntimeKind::BuiltinKit);
assert_eq!(config.surface.mode, AppUiSurfaceMode::Session);
assert_eq!(config.surface.chrome, AppUiSurfaceChrome::Host);
validate_ui_runtime_config(Some(&config)).unwrap();
```

- [ ] **Step 2: Run targeted test to verify failure**

Run:

```bash
cd macaca && cargo test -p macaca-app ui_runtime_accepts_builtin_kit_session_surface -- --nocapture
```

Expected: FAIL because `builtin_kit` is not a known runtime and `entry` is currently required.

- [ ] **Step 3: Implement minimal model changes**

Add `BuiltinKit` to `AppUiRuntimeKind`, make `entry` optional for built-in kit, and add a small `AppUiPresentationConfig` for schema/preferred components. Include English comments explaining that this is a host-rendered declarative surface, not executable remote UI.

- [ ] **Step 4: Run targeted tests**

Run:

```bash
cd macaca && cargo test -p macaca-app ui_runtime -- --nocapture
```

Expected: PASS.

## Task 3: Add Generic GenUI Surface Repository

**Files:**
- Modify: `/Users/quantum/Code/dev/agent/macaca/crates/runtime/macaca-runtime-host/src/application_service_provider.rs`
- Test: `/Users/quantum/Code/dev/agent/macaca/crates/runtime/macaca-runtime-host/src/application_service_provider.rs` or adjacent runtime-host test module.

- [ ] **Step 1: Write failing Application Service test**

Create a test that:

```rust
let intent = UiIntent { /* app_id, session_id, surface_id: "crypto-signal", card root */ };
let render = ApplicationHostCommand::with_trace(
    ApplicationImport::UiRender,
    serde_json::to_value(intent.clone()).unwrap(),
    TraceContext::new("test-render"),
);
```

Dispatch render through Application Service or the new repository helper, then query:

```rust
ApplicationGenUiSurfaceCommand {
    trace: TraceContext::new("test-query"),
    scope: ApplicationServiceScope::application(app_id),
    surface_id: Some("crypto-signal".into()),
}
```

Expected: returned JSON decodes as the same `UiIntent`.

- [ ] **Step 2: Run test to verify failure**

Run:

```bash
cd macaca && cargo test -p macaca-runtime-host genui_surface -- --nocapture
```

Expected: FAIL because GenUI surface query is currently unavailable.

- [ ] **Step 3: Implement repository**

Add a private `ApplicationGenUiSurfaceStore` with:

```rust
type SurfaceKey = (ApplicationId, String, String);
```

Store only validated, bounded `UiIntent` values. Use `RwLock<HashMap<SurfaceKey, UiIntent>>`, keep comments on why this is an in-memory Memento for the first slice, and avoid frontend-specific types.

- [ ] **Step 4: Implement query command**

Replace the unavailable branch for `APPLICATION_GENUI_SURFACE_COMMAND` with a lookup that returns either the stored intent or a structured empty surface result.

- [ ] **Step 5: Run tests**

Run:

```bash
cd macaca && cargo test -p macaca-runtime-host genui_surface -- --nocapture
```

Expected: PASS.

## Task 4: Route UiRender Host Commands

**Files:**
- Modify: `/Users/quantum/Code/dev/agent/macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/host_import_bridge.rs`
- Modify: `/Users/quantum/Code/dev/agent/macaca/crates/runtime/macaca-runtime-host/src/application_service_provider.rs`
- Test: `/Users/quantum/Code/dev/agent/macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/component_model_tests.rs`

- [ ] **Step 1: Write failing component model test**

Add a test where the embedded host-command plan includes:

```json
{"import":"ui_render","payload":{"surface":"crypto-signal","input":{"kind":"card"}},"trace":null,"metadata":{"surface.id":"crypto-signal"}}
```

Expected: dispatching `app:start` returns a host command result with `import_name` for `macaca:ui/render` and status `Ok`.

- [ ] **Step 2: Run test to verify failure**

Run:

```bash
cd macaca && cargo test -p macaca-runtime-host component_model_provider_dispatches_declared_ui_render_command -- --nocapture
```

Expected: FAIL because `WasmHostImportBridge` currently rejects non-`ServiceCall` imports.

- [ ] **Step 3: Implement generic render sink routing**

Add a narrow render dispatch path that accepts only `ApplicationImport::UiRender`, requires trace/app/session/surface scope, validates `UiIntent` or a render envelope, and stores it in the Application Service surface store. Keep service-call routing unchanged.

- [ ] **Step 4: Run targeted tests**

Run:

```bash
cd macaca && cargo test -p macaca-runtime-host component_model_provider_dispatches_declared_ui_render_command -- --nocapture
cd macaca && cargo test -p macaca-runtime-host wasm_host_import_bridge -- --nocapture
```

Expected: PASS with existing `service.call` tests unchanged.

## Task 5: Refresh Session GenUI in Web Shell

**Files:**
- Modify: `/Users/quantum/Code/dev/agent/frontend/app/chat/[appId]/page.tsx`
- Existing: `/Users/quantum/Code/dev/agent/frontend/lib/genui.ts`
- Existing: `/Users/quantum/Code/dev/agent/frontend/components/genui/GenUiRenderer.tsx`

- [ ] **Step 1: Add a focused frontend regression**

If the test harness is available, add or update a test that simulates a completed WASM session and verifies `fetchGenUiSurface(appId, sessionId)` is called without checking the app id.

- [ ] **Step 2: Implement refresh trigger**

Reuse existing `loadGenUiSurface(currentSession.session_id)`. Trigger it after a WASM chat stream completes for the active session. Keep comments on why this is a generic session-surface refresh and not a crypto renderer hook.

- [ ] **Step 3: Run frontend checks**

Run:

```bash
cd frontend && npm test -- --runInBand GenUiRenderer
cd frontend && npm run lint
```

Expected: PASS, or document unavailable scripts if the repo does not define them.

## Task 6: Create Standalone Crypto App

**Files:**
- Create: `/Users/quantum/Code/dev/wasm-crypto-signal-app/Cargo.toml`
- Create: `/Users/quantum/Code/dev/wasm-crypto-signal-app/app.yaml`
- Create: `/Users/quantum/Code/dev/wasm-crypto-signal-app/crates/crypto-signal-contract/src/lib.rs`
- Create: `/Users/quantum/Code/dev/wasm-crypto-signal-app/crates/crypto-signal-guest/src/lib.rs`
- Create: `/Users/quantum/Code/dev/wasm-crypto-signal-app/crates/crypto-signal-guest/src/input.rs`
- Create: `/Users/quantum/Code/dev/wasm-crypto-signal-app/crates/crypto-signal-guest/src/signals.rs`
- Create: `/Users/quantum/Code/dev/wasm-crypto-signal-app/crates/crypto-signal-packaging/src/lib.rs`
- Create: `/Users/quantum/Code/dev/wasm-crypto-signal-app/scripts/run_e2e_audit_chain.sh`

- [ ] **Step 1: Scaffold workspace**

Use the stock app as the template but remove React UI bundle files. The manifest must include:

```yaml
name: wasm-crypto-signal-app
layer: L2Wasm
ui_type: chat
service_contract:
  use_packs:
    - pack.crypto.v1
  required_services:
    - service.crypto_market
    - service.crypto_news
    - service.llm.analysis
ui:
  runtime: builtin_kit
  surface:
    mode: session
    chrome: host
  presentation:
    schema: genui.v1
    preferred_components:
      - card
      - table
      - list
      - badge
```

- [ ] **Step 2: Implement input tests**

Add tests:

```rust
assert_eq!(normalize_symbol("btc").unwrap().as_str(), "BTC");
assert_eq!(normalize_symbol("分析 ETH 买卖信号").unwrap().as_str(), "ETH");
assert!(normalize_symbol("../BTC").is_err());
```

- [ ] **Step 3: Implement deterministic signal model**

Create typed signal/risk structs with `analysis_only: true` and `not_financial_advice: true`. Avoid `dynamic` JSON construction in core logic; serialize typed structs at host-command boundaries.

- [ ] **Step 4: Embed component metadata**

Embed:

```text
macaca:component-model:v1
export=app:start
wit=macaca:application/runtime@1.0.0
host-command={... service.crypto_market ...}
host-command={... service.crypto_news ...}
host-command={... service.llm.analysis ...}
host-command={... ui_render ...}
```

- [ ] **Step 5: Build and test**

Run:

```bash
cd /Users/quantum/Code/dev/wasm-crypto-signal-app
cargo test
cargo build -p crypto-signal-guest --target wasm32-unknown-unknown --release
```

Expected: tests pass and the WASM artifact contains all marker strings.

## Task 7: Install and End-to-End Verify

**Files:**
- Install target: `/Users/quantum/.macaca/workspaces/apps/wasm-crypto-signal-app`
- Backend log: `/tmp/macaca-backend.log`
- Frontend: `/Users/quantum/Code/dev/agent/frontend`

- [ ] **Step 1: Install app**

Copy `component.wasm`, `manifest.json`, `release_bundle.json`, and `app.yaml` into the workspace app directory.

- [ ] **Step 2: Start services**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca && cargo run --bin macaca -- web
cd /Users/quantum/Code/dev/agent/frontend && npx next dev --port 3000
```

Expected: backend on 3001, frontend on 3000.

- [ ] **Step 3: Verify API discovery**

Run:

```bash
curl -s http://127.0.0.1:3001/api/apps | jq '.[] | select(.name=="wasm-crypto-signal-app")'
```

Expected: app appears with L2Wasm runtime metadata and session UI surface metadata.

- [ ] **Step 4: Verify real session in browser**

Open `http://localhost:3000`, launch the crypto app, and send:

```text
分析 BTC 买卖信号
```

Expected: main thread, composer, AgentPanel, trace/audit panels remain visible; a GenUI card-style analysis surface appears; service-call audit replay shows crypto market/news/LLM calls or structured unavailable results.

## Task 8: Final Checks

- [ ] **Step 1: Run focused Rust checks**

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-app ui_runtime -- --nocapture
cargo test -p macaca-runtime-host genui_surface -- --nocapture
cargo check
```

- [ ] **Step 2: Run crypto app checks**

```bash
cd /Users/quantum/Code/dev/wasm-crypto-signal-app
cargo test
strings dist/component.wasm | rg "macaca:component-model:v1|app:start|service.crypto_market|service.crypto_news|service.llm.analysis|ui_render"
```

- [ ] **Step 3: Run GitNexus change detection**

Run:

```text
detect_changes(scope: "all")
```

Expected: changed symbols align with this plan; investigate any unexpected execution flows before integration.
