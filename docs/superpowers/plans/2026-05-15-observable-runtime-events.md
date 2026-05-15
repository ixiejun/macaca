# Observable Runtime Events Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make skill loading and generic data retrieval visible as durable session events and live SSE updates.

**Architecture:** Add a small Web adapter helper that appends sanitized runtime events to EventLog before SSE. Wire skill snapshot lifecycle and service-call audit lifecycle through this helper while preserving service/runtime ownership.

**Tech Stack:** Rust, Axum SSE, macaca-web, macaca-persist EventLog, OpenSpec.

---

### Task 1: OpenSpec Change

**Files:**
- Create: `openspec/changes/add-observable-runtime-events/proposal.md`
- Create: `openspec/changes/add-observable-runtime-events/design.md`
- Create: `openspec/changes/add-observable-runtime-events/tasks.md`
- Create: `openspec/changes/add-observable-runtime-events/specs/session-event-log/spec.md`
- Create: `openspec/changes/add-observable-runtime-events/specs/skill-service/spec.md`
- Create: `openspec/changes/add-observable-runtime-events/specs/service-runtime-audit/spec.md`

- [x] **Step 1: Write proposal and deltas**

Define requirements for persisted-before-SSE runtime events, sanitized skill snapshot visibility, and session-scoped service-call audit visibility.

- [ ] **Step 2: Validate OpenSpec**

Run: `openspec validate add-observable-runtime-events --strict`
Expected: validation succeeds.

### Task 2: Runtime Event Helper

**Files:**
- Create: `macaca/crates/shells/macaca-web/src/runtime_event_bridge.rs`
- Modify: `macaca/crates/shells/macaca-web/src/lib.rs`
- Test: `macaca/crates/shells/macaca-web/src/runtime_event_bridge.rs`

- [ ] **Step 1: Write failing helper test**

Add a unit test proving the helper appends to EventLog even when no SSE sender is attached.

- [ ] **Step 2: Run failing test**

Run: `cargo test -p macaca-web runtime_event_bridge -- --nocapture`
Expected: fail because the helper module does not exist yet.

- [ ] **Step 3: Implement helper**

Create a helper with a function that takes `AppState`, `session_id`, `event_type`, `source`, `agent_name`, payload, appends `AppendEventCommand`, and then sends `Event::default().event(event_type).data(payload)`.

- [ ] **Step 4: Run helper test**

Run: `cargo test -p macaca-web runtime_event_bridge -- --nocapture`
Expected: pass.

### Task 3: Skill Snapshot Lifecycle Events

**Files:**
- Modify: `macaca/crates/shells/macaca-web/src/skill_mcp.rs`
- Test: `macaca/crates/shells/macaca-web/src/skill_mcp.rs`

- [ ] **Step 1: Write failing test**

Add tests for a pure payload builder proving snapshot events include counts and exclude full skill prompt/body.

- [ ] **Step 2: Run failing test**

Run: `cargo test -p macaca-web skill_mcp_snapshot_event -- --nocapture`
Expected: fail because the payload builder does not exist.

- [ ] **Step 3: Implement snapshot event emission**

Emit cache hit, build started, ready, failed, and cached events through `runtime_event_bridge`.

- [ ] **Step 4: Run skill tests**

Run: `cargo test -p macaca-web skill_mcp -- --nocapture`
Expected: pass.

### Task 4: Service-Call Audit Session Events

**Files:**
- Modify: `macaca/crates/shells/macaca-web/src/chat_orchestrator.rs`
- Modify: `macaca/crates/shells/macaca-web/src/runtime_event_bridge.rs`
- Test: `macaca/crates/shells/macaca-web/src/runtime_event_bridge.rs`

- [ ] **Step 1: Write failing audit payload test**

Add a test proving service-call audit payloads contain stage/service/trace/hash fields and no raw output.

- [ ] **Step 2: Run failing test**

Run: `cargo test -p macaca-web service_call_audit_event -- --nocapture`
Expected: fail because the audit event conversion does not exist.

- [ ] **Step 3: Bridge service-call audit events**

Use existing replayable audit sink data to append session-scoped `service_call_audit` events through the helper when the active chat path has a session id.

- [ ] **Step 4: Run audit tests**

Run: `cargo test -p macaca-web service_call_audit_event -- --nocapture`
Expected: pass.

### Task 5: Final Verification

**Files:**
- All touched files

- [ ] **Step 1: Validate OpenSpec**

Run: `openspec validate add-observable-runtime-events --strict`
Expected: pass.

- [ ] **Step 2: Run targeted Rust tests**

Run: `cargo test -p macaca-web skill_mcp runtime_event_bridge service_call_audit_event -- --nocapture`
Expected: pass.

- [ ] **Step 3: Run compile check**

Run: `cargo check -p macaca-web`
Expected: pass.

- [ ] **Step 4: Inspect change impact**

Run GitNexus detect changes for all local edits and confirm scope is limited to observable runtime events.
