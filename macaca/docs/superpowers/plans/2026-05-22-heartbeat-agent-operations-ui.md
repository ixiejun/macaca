# Heartbeat Agent Operations UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an application-level Heartbeat Operations dialog surface beside the existing scheduled-task surface.

**Architecture:** Web remains a thin shell that adapts HTTP requests to typed SDK/service commands. Heartbeat profile state and run mementos stay in `service.heartbeat`; manifest-declared agent projections stay in Application Service.

**Tech Stack:** Rust/Axum, Macaca SDK focused clients, `macaca-proto` DTOs, Next.js/React frontend components.

---

### Task 1: OpenSpec Delta

**Files:**
- Create: `openspec/changes/add-heartbeat-agent-operations-ui/proposal.md`
- Create: `openspec/changes/add-heartbeat-agent-operations-ui/design.md`
- Create: `openspec/changes/add-heartbeat-agent-operations-ui/tasks.md`
- Create: `openspec/changes/add-heartbeat-agent-operations-ui/specs/web-cli-thin-shell-v0/spec.md`
- Create: `openspec/changes/add-heartbeat-agent-operations-ui/specs/sdk-system-facade/spec.md`
- Create: `openspec/changes/add-heartbeat-agent-operations-ui/specs/serviceization-escape-hatches/spec.md`

- [ ] Document app-scoped Heartbeat Operations UI as a shell/facade feature.
- [ ] Require no scheduler-owned heartbeat target semantics.
- [ ] Validate with `openspec validate add-heartbeat-agent-operations-ui --strict`.

### Task 2: Heartbeat Contract And SDK

**Files:**
- Modify: `macaca/crates/foundation/macaca-proto/src/heartbeat_service.rs`
- Modify: `macaca/crates/services/macaca-heartbeat/src/service_contract.rs`
- Modify: `macaca/crates/services/macaca-heartbeat/src/local_provider/command_handler.rs`
- Modify: `macaca/crates/facade/macaca-sdk/src/heartbeat_client.rs`

- [ ] Add typed profile update command/result support for enabled state, interval, and metadata.
- [ ] Keep updates provider-neutral and trace-required.
- [ ] Add tests that profile edits update snapshots and emit audit ids.

### Task 3: Web Routes

**Files:**
- Modify: `macaca/crates/shells/macaca-web/src/state.rs`
- Modify: `macaca/crates/shells/macaca-web/src/lib.rs`
- Modify: `macaca/crates/shells/macaca-web/src/bootstrap.rs`
- Modify: `macaca/crates/shells/macaca-web/src/routes.rs`

- [ ] Add `heartbeat_client` to `AppState` through service-backed SDK client wiring.
- [ ] Add GET `/api/apps/{app_id}/autonomy/heartbeat` for declarations, native profiles, recent runs, trace id, and safe summary counts.
- [ ] Add PATCH `/api/apps/{app_id}/autonomy/heartbeat/profiles/{profile_id}` for native profile edits.
- [ ] Log key route execution nodes with app id, profile id, trace id, and audit ids.

### Task 4: Frontend UI

**Files:**
- Modify: `frontend/lib/autonomy-types.ts`
- Modify: `frontend/lib/autonomy.ts`
- Modify: `frontend/components/autonomy/ApplicationOperationsDialog.tsx`
- Create: `frontend/components/autonomy/HeartbeatOperationsPanel.tsx`
- Create: `frontend/components/autonomy/HeartbeatAgentList.tsx`
- Create: `frontend/components/autonomy/HeartbeatProfileList.tsx`
- Create: `frontend/components/autonomy/HeartbeatRunTimeline.tsx`
- Create: `frontend/components/autonomy/HeartbeatProfileEditorDrawer.tsx`

- [ ] Add typed fetch/update helpers.
- [ ] Add adjacent Scheduler/Heartbeat mode controls in the operations dialog.
- [ ] Render declared heartbeat agents, native profile state, recent run mementos, and edit drawer.
- [ ] Keep copy generic and avoid application-specific labels.

### Task 5: Verification

**Commands:**
- `openspec validate add-heartbeat-agent-operations-ui --strict`
- `cargo fmt --all -- --check`
- `cargo test -p macaca-heartbeat -- --nocapture`
- `cargo test -p macaca-web heartbeat -- --nocapture`
- `cargo check -p macaca-proto -p macaca-sdk -p macaca-heartbeat -p macaca-runtime-host -p macaca-web`
- `cd frontend && npm run lint`
- `cd frontend && npx tsc --noEmit`

- [ ] Run GitNexus detect changes after implementation.
- [ ] Report any existing warnings separately from new failures.
