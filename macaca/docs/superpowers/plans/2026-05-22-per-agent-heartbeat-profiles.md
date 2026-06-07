# Per-Agent Heartbeat Profiles Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Register and operate independent native Heartbeat profiles for each manifest-declared heartbeat agent.

**Architecture:** Application manifests declare per-agent heartbeat policy. Application Service projects sanitized declarations. Runtime-host adapts each declaration into a Heartbeat-owned native profile. Heartbeat evaluates each profile independently and dispatches only the matching declaration.

**Tech Stack:** Rust workspace under `macaca/`, OpenSpec, Next.js frontend under `frontend/`.

---

### Task 1: Contracts And Specs

**Files:**
- Modify: `macaca/crates/application/macaca-app/src/model.rs`
- Modify: `macaca/crates/foundation/macaca-proto/src/application_service.rs`
- Modify: `macaca/crates/foundation/macaca-proto/src/heartbeat_service.rs`
- Create: `openspec/changes/add-per-agent-heartbeat-profiles/*`

- [ ] Add per-agent cadence/gate manifest fields with serde defaults.
- [ ] Add sanitized declaration fields for native profile id and wake scope key.
- [ ] Add profile summary/update fields for fixed interval and cooldown.
- [ ] Validate OpenSpec strictly.

### Task 2: Application Projection

**Files:**
- Modify: `macaca/crates/application/macaca-app/src/service_projection.rs`

- [ ] Compute stable agent profile ids and scope keys without app-specific branches.
- [ ] Preserve raw manifest `profile_id` as a selector while exposing `native_profile_id`.
- [ ] Add projection tests for two agents with different policy values.

### Task 3: Runtime And Heartbeat Provider

**Files:**
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/autonomy_supervisor.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/autonomy_supervisor/heartbeat_agent_dispatch.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/autonomy_supervisor/heartbeat_lane.rs`
- Modify: `macaca/crates/services/macaca-heartbeat/src/local_provider.rs`
- Modify: `macaca/crates/services/macaca-heartbeat/src/local_provider/gates.rs`
- Modify: `macaca/crates/services/macaca-heartbeat/src/local_provider/command_handler.rs`
- Modify: `macaca/crates/services/macaca-heartbeat/src/local_provider/memento.rs`

- [ ] Register one native profile per valid enabled heartbeat declaration.
- [ ] Copy profile metadata into native wake commands for trace and gate policy.
- [ ] Evaluate cooldown per profile when profile policy supplies one.
- [ ] Dispatch only declarations matching the accepted profile.
- [ ] Add focused runtime/provider tests.

### Task 4: Web And Frontend

**Files:**
- Modify: `macaca/crates/shells/macaca-web/src/heartbeat_operations_routes.rs`
- Modify: `macaca/crates/shells/macaca-web/src/heartbeat_operations_routes/view.rs`
- Modify: `frontend/lib/heartbeat-types.ts`
- Modify: `frontend/lib/autonomy.ts`
- Modify: `frontend/components/autonomy/*.tsx`

- [ ] Aggregate profiles and runs across all per-agent scope keys.
- [ ] Let profile edits update fixed interval and cooldown independently.
- [ ] Display per-agent profile identity clearly in the operations panel.
- [ ] Run frontend lint and TypeScript checks.

### Task 5: Verification

- [ ] Run `cargo fmt`.
- [ ] Run focused cargo tests for `macaca-app`, `macaca-heartbeat`, `macaca-runtime-host`, and `macaca-web`.
- [ ] Run `cargo check -p macaca-proto -p macaca-app -p macaca-heartbeat -p macaca-runtime-host -p macaca-web`.
- [ ] Run `openspec validate add-per-agent-heartbeat-profiles --strict`.
- [ ] Run `git diff --check`, `git -C frontend diff --check`, and GitNexus detect changes.
