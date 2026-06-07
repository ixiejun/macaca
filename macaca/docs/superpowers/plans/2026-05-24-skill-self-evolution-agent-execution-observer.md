# Skill Self-Evolution Agent Execution Observer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Route real Agent Execution completions into Skill self-evolution proposals through one audited service-boundary observer.

**Architecture:** Add a Web-shell Decorator around `WebAgentExecutionBackend` at provider registration time. The decorator observes the returned `AgentExecutionResult`, records sanitized EventLog checkpoints, and delegates proposal creation to the existing Skill service facade without changing runtime-host traits or Skill semantics.

**Tech Stack:** Rust, `macaca-web`, `macaca-runtime-host` service trait, `macaca-sdk` Skill facade, EventLog, OpenSpec.

---

### Task 1: Lock The Service-Boundary Contract With Tests

**Files:**
- Modify: `macaca/crates/shells/macaca-web/src/agent_execution_backend/tests.rs`
- Modify: `macaca/crates/shells/macaca-web/src/chat_orchestrator.rs`
- Modify: `macaca/crates/shells/macaca-web/src/event_persistence.rs`

- [ ] **Step 1: Add source guards for the new boundary**

Add tests that assert `agent_execution_backend.rs` no longer contains
`spawn_skill_self_evolution_observation`, `chat_orchestrator.rs` no longer calls
`observe_agent_execution_result_for_skill_self_evolution`, and
`event_persistence.rs` no longer calls
`observe_executor_event_for_skill_self_evolution`.

- [ ] **Step 2: Run the targeted tests and confirm they fail**

Run: `cargo test -p macaca-web agent_execution_backend -- --nocapture`

Expected: failure because the current working tree still contains the scattered
observer hooks.

### Task 2: Add The Decorator

**Files:**
- Create: `macaca/crates/shells/macaca-web/src/skill_self_evolution_execution_observer.rs`
- Modify: `macaca/crates/shells/macaca-web/src/lib.rs`

- [ ] **Step 1: Implement `SkillSelfEvolutionObservedAgentExecutionBackend`**

Create a focused wrapper implementing `macaca_runtime_host::AgentExecutionBackend`.
It should call the inner backend, emit `skill_self_evolution_observer` with
`agent_execution_completed_seen`, call
`observe_agent_execution_result_for_skill_self_evolution`, emit the final
observer status, and always return the original service result.

- [ ] **Step 2: Register the wrapper**

In `serve_web_server`, wrap `WebAgentExecutionBackend::new(...)` before passing
it to `AgentExecutionSystemServiceProvider::new(...)`.

- [ ] **Step 3: Add source guards**

Add tests that prove `lib.rs` imports and registers
`SkillSelfEvolutionObservedAgentExecutionBackend`.

### Task 3: Remove Scattered Observer Hooks

**Files:**
- Modify: `macaca/crates/shells/macaca-web/src/agent_execution_backend.rs`
- Modify: `macaca/crates/shells/macaca-web/src/chat_orchestrator.rs`
- Modify: `macaca/crates/shells/macaca-web/src/event_persistence.rs`

- [ ] **Step 1: Remove fire-and-forget observation from the backend**

Delete `spawn_skill_self_evolution_observation`,
`record_skill_self_evolution_observation`, the related imports, and the two call
sites before `evidence_observer.finish().await`.

- [ ] **Step 2: Remove chat orchestration observer duplication**

Delete `observe_chat_service_skill_self_evolution`, its imports, and the call
from `run_chat_main_thread_via_agent_service`.

- [ ] **Step 3: Restore event persistence to durable executor logging only**

Rename `spawn_session_event_collector_with_skill_observer` back to the existing
`spawn_session_event_collector` shape and remove Skill observer state from the
collector. Keep EventLog and RunTracer persistence unchanged.

### Task 4: Update OpenSpec Tracking

**Files:**
- Modify: `openspec/changes/add-self-evolution-evaluation-harness/design.md`
- Modify: `openspec/changes/add-self-evolution-evaluation-harness/tasks.md`

- [ ] **Step 1: Record the selected boundary**

Add a design note that live task-loop proposal extraction is observed at
`service.agent_execution` through a Web composition decorator.

- [ ] **Step 2: Keep live verification tasks truthful**

Leave 7.1 to 7.3 unchecked until live API evidence proves proposals grow and
can be promoted or rejected.

### Task 5: Verify

**Files:**
- Test: Rust tests and OpenSpec validation

- [ ] **Step 1: Run targeted tests**

Run: `cargo test -p macaca-web skill_self_evolution_observer -- --nocapture`

Expected: all observer tests pass.

- [ ] **Step 2: Run agent execution backend tests**

Run: `cargo test -p macaca-web agent_execution_backend -- --nocapture`

Expected: all backend source guards and execution-control tests pass.

- [ ] **Step 3: Run package check**

Run: `cargo check -p macaca-web`

Expected: package compiles.

- [ ] **Step 4: Validate OpenSpec**

Run: `openspec validate add-self-evolution-evaluation-harness --strict`

Expected: validation succeeds.

- [ ] **Step 5: Run diff and graph checks**

Run: `git diff --check`

Expected: no whitespace errors.

Run GitNexus `detect_changes(scope="all", repo="agent")`.

Expected: changed symbols are limited to the observer/decorator boundary,
registration, docs, and live verification tasks.
