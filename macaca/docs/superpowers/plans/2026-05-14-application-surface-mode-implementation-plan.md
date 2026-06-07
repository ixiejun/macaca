# Application Surface Mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add manifest-declared application/session surface modes so app-owned WASM UI can replace the center chat column while session apps keep the chat shell.

**Architecture:** Keep `ui.runtime` as the loading strategy and add `ui.surface` as the shell placement strategy. Backend admission and service projection expose sanitized metadata; frontend chooses between application workspace and session workspace through a generic surface router.

**Tech Stack:** Rust, serde, macaca-app, macaca-proto, macaca-web, Next.js, React, TypeScript.

---

### Task 1: Backend Surface Contract

**Files:**
- Modify: `macaca/crates/application/macaca-app/src/ui_runtime.rs`
- Modify: `macaca/crates/foundation/macaca-proto/src/application_service.rs`
- Modify: `macaca/crates/application/macaca-app/src/service_projection.rs`

- [ ] Add `AppUiSurfaceConfig`, `AppUiSurfaceMode`, and `AppUiSurfaceChrome` with default `session` + `host`.
- [ ] Add `surface` to `AppUiRuntimeConfig` and `ApplicationUiRuntimeView`.
- [ ] Project surface metadata and log `surface_mode` / `surface_chrome`.
- [ ] Add tests for default session mode and application mode parsing.

### Task 2: OpenSpec Delta

**Files:**
- Modify: `openspec/changes/add-application-owned-ui-runtime/design.md`
- Modify: `openspec/changes/add-application-owned-ui-runtime/specs/application-ui-runtime/spec.md`
- Modify: `openspec/changes/add-application-owned-ui-runtime/tasks.md`

- [ ] Add requirements for application and session surface modes.
- [ ] Document compatibility defaults.
- [ ] Validate with `openspec validate add-application-owned-ui-runtime --strict`.

### Task 3: Frontend Surface Router

**Files:**
- Modify: `frontend/lib/types.ts`
- Create: `frontend/lib/app-surface.ts`
- Create: `frontend/components/ApplicationWorkspaceSurface.tsx`
- Modify: `frontend/app/chat/[appId]/page.tsx`
- Modify: `frontend/app/globals.css`

- [ ] Add frontend types for `ui.surface`.
- [ ] Add generic helpers that classify application vs session surface mode.
- [ ] Add a full-workspace component that hosts `AppOwnedUiSurface`.
- [ ] Route `surface.mode: application` away from the chat composer and main-thread tabs while preserving the universal right-side AgentPanel.
- [ ] Keep existing chat shell for default/session mode.

### Task 4: Stock App Manifest Migration

**Files:**
- Modify: `/Users/quantum/Code/dev/wasm-stock-agent-app/app.yaml`
- Modify: `/Users/quantum/Code/dev/wasm-stock-agent-app/dist/app.yaml`
- Copy: `/Users/quantum/Code/dev/wasm-stock-agent-app/dist/` to `/Users/quantum/.macaca/workspaces/apps/wasm-stock-agent-app/`

- [ ] Declare `ui.surface.mode: application` and `ui.surface.chrome: app_owned`.
- [ ] Sync installed package with `rsync -a --delete`.

### Task 5: Verification

**Commands:**
- `cargo test -p macaca-app ui_runtime -- --nocapture`
- `cargo test -p macaca-web app_ui -- --nocapture`
- `cargo build -p macaca-cli --bin macaca`
- `npm run lint`
- `npm run build`
- `openspec validate add-application-owned-ui-runtime --strict`

- [ ] Confirm all checks pass.
- [ ] Restart backend/frontend for manual verification.
- [ ] Confirm stock app page renders an app-owned center column rather than chat shell widgets, while preserving the right-side AgentPanel.
