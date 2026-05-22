# Autonomous Execution Envelope Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a minimal OS-owned execution envelope so heartbeat and scheduled task dispatches preserve source instructions and generic evidence policy before Agent Execution.

**Architecture:** `macaca-proto` owns provider-neutral DTOs, `macaca-runtime-host` compiles envelopes at autonomy dispatch boundaries, and `macaca-web` renders the envelope as the highest-priority delegated execution contract while existing evidence gates validate completion.

**Tech Stack:** Rust, serde DTOs, runtime-host service dispatch strategies, OpenSpec.

---

### Task 1: Contract DTOs

**Files:**
- Modify: `/Users/quantum/Code/dev/agent/macaca/crates/foundation/macaca-proto/src/agent_execution_service.rs`

- [ ] Add `AutonomousExecutionEnvelope`, source kind, instruction priority, execution mode, and completion policy DTOs.
- [ ] Add a deterministic compiler from source instruction plus generic metadata.
- [ ] Add round-trip and evidence-policy unit tests.

### Task 2: Runtime Dispatch

**Files:**
- Modify: `/Users/quantum/Code/dev/agent/macaca/crates/runtime/macaca-runtime-host/src/autonomy_dispatch.rs`
- Modify: `/Users/quantum/Code/dev/agent/macaca/crates/runtime/macaca-runtime-host/src/autonomy_supervisor/heartbeat_agent_dispatch.rs`

- [ ] Attach envelopes to scheduled-agent-task commands after metadata normalization.
- [ ] Attach envelopes to heartbeat commands after declaration metadata is copied.
- [ ] Log source kind and completion policy, never raw instruction text.

### Task 3: Agent Execution Rendering

**Files:**
- Modify: `/Users/quantum/Code/dev/agent/macaca/crates/shells/macaca-web/src/agent_execution_backend.rs`
- Modify: `/Users/quantum/Code/dev/agent/macaca/crates/shells/macaca-web/src/agent_execution_backend/tests.rs`

- [ ] Render the envelope before ordinary delegated evidence context.
- [ ] Add a rendering test that proves highest-priority wording appears.

### Task 4: Validation

- [ ] Run focused proto/runtime/web tests.
- [ ] Run OpenSpec strict validation.
- [ ] Run GitNexus detect changes.
