# Unified Application Execution Path Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Converge WASM, YAML, GenUI, headless, and shell-initiated application execution onto one provider-neutral execution path.

**Architecture:** `service.application_execution` is the only ingress for application runs, `service.task` is the only owner of execution task graph and task lifecycle, and `service.agent_execution` is the only production boundary for agent work. Application adapters produce typed intent commands; shells render projections and never own semantic execution state.

**Tech Stack:** Rust workspace under `macaca/`, OpenSpec, ServiceRuntime, `macaca-proto`, `macaca-task`, `macaca-runtime-host`, `macaca-sdk`, `macaca-web`, Next.js frontend, app-owned UI bundles.

---

## File Structure

- Create: `openspec/changes/unify-application-execution-path/proposal.md`
  - Describes the architecture problem, why multiple execution paths are invalid, and the target single path.
- Create: `openspec/changes/unify-application-execution-path/design.md`
  - Defines ownership, service boundaries, state model, adapter migration, and risk controls.
- Create: `openspec/changes/unify-application-execution-path/tasks.md`
  - Tracks implementation phases and verification gates.
- Create: `openspec/changes/unify-application-execution-path/specs/unified-application-execution-path/spec.md`
  - Adds normative requirements for one ingress, one task graph, one terminal projection, and shell adapter limits.
- Modify: `macaca/crates/foundation/macaca-proto/src/application_execution.rs`
  - Add execution envelope fields only if existing DTOs cannot already represent execution source, ownership, and terminal projection scope.
- Modify: `macaca/crates/services/macaca-task/src/runtime.rs`
  - Add or tighten command handling so application execution can create exactly one session-scoped task graph and mark ownership.
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/application_execution_hosted.rs`
  - Stop terminal aggregation from treating unrelated compatibility/fallback host command rows as authoritative run failure.
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/host_import_bridge.rs`
  - Ensure WASM `agent.delegate` only emits application intent through the unified service path and does not create a second semantic graph.
- Modify: `macaca/crates/shells/macaca-web/src/loop_manager.rs`
  - Move fallback decomposition behind Task Service strategy/provider seams or compatibility adapter guardrails.
- Modify: `macaca/crates/shells/macaca-web/src/routes.rs`
  - Keep HTTP routes as thin adapters over SDK/SystemFacade clients.
- Modify: `apps/codex-wasm-workbench/ui/src/WorkbenchApp.tsx`
  - Render unified projections only; do not infer terminal state from local event caches.

## Task 1: OpenSpec And Governance Baseline

**Files:**
- Create: `openspec/changes/unify-application-execution-path/proposal.md`
- Create: `openspec/changes/unify-application-execution-path/design.md`
- Create: `openspec/changes/unify-application-execution-path/tasks.md`
- Create: `openspec/changes/unify-application-execution-path/specs/unified-application-execution-path/spec.md`

- [ ] **Step 1: Validate the new OpenSpec change**

Run:

```bash
openspec validate unify-application-execution-path --strict
```

Expected: `Change 'unify-application-execution-path' is valid`.

- [ ] **Step 2: Re-read governance documents before implementation**

Run:

```bash
sed -n '1,260p' macaca/docs/macaca-os-architecture-governance.md
sed -n '1,260p' macaca/docs/macaca-os-microkernel-boundaries.md
sed -n '1,260p' macaca/docs/macaca-os-serviceization-allowlist.md
```

Expected: Implementation notes explicitly state that the kernel owns only invariants, `service.application_execution` owns run ingress/projection, `service.task` owns task graph lifecycle, `service.agent_execution` owns agent work, and shells/app UIs are adapters.

- [ ] **Step 3: Run GitNexus impact analysis before code edits**

Run the available GitNexus impact command for every existing symbol touched. If the CLI or MCP index reports HIGH or CRITICAL risk, record it as a memo and continue only after confirming the concrete direct-call risk is understood.

Expected: A short implementation note per edited symbol with direct callers, affected process names when available, and risk memo.

## Task 2: Define The Unified Execution Envelope

**Files:**
- Modify: `macaca/crates/foundation/macaca-proto/src/application_execution.rs`
- Test: existing application-execution proto tests in `macaca/crates/runtime/macaca-runtime-host/src/application_execution_*_tests.rs` or matching proto unit tests.

- [ ] **Step 1: Write failing tests for envelope ownership**

Add tests that construct a start command for WASM and YAML shaped inputs and assert both produce the same provider-neutral fields:

```rust
assert_eq!(command.application_id, expected_application_id);
assert_eq!(command.session_id.as_deref(), Some("session-a"));
assert_eq!(command.run_id.as_deref(), Some("run-a"));
assert!(command.policy_context.contains_key("application_execution.profile"));
assert!(!serde_json::to_value(&command).unwrap().to_string().contains("codex-wasm-workbench"));
```

Run targeted tests and expect failure until the envelope carries the required ownership fields consistently.

- [ ] **Step 2: Implement the minimal DTO tightening**

If existing DTOs are sufficient, add comments and tests only. If they are insufficient, add bounded fields such as:

```rust
/// Identifies which service owns terminal state for this run.
/// The field is provider-neutral and never names an application, workflow,
/// model, driver, or business domain. It lets adapters reject side-channel
/// terminal claims from compatibility paths that are not authoritative for
/// this execution envelope.
pub terminal_projection_owner: Option<String>,
```

Keep comments in English and explain ownership, replay, and sanitization.

- [ ] **Step 3: Verify DTO serialization**

Run:

```bash
cargo test -p macaca-proto application_execution --lib
```

Expected: serialization preserves application/session/run/trace ownership and does not require app-specific fields.

## Task 3: Make Task Service The Only Task Graph Owner

**Files:**
- Modify: `macaca/crates/services/macaca-task/src/commands.rs`
- Modify: `macaca/crates/services/macaca-task/src/runtime.rs`
- Modify: `macaca/crates/services/macaca-task/src/todo_board.rs`
- Test: `macaca/crates/services/macaca-task/src/runtime.rs`

- [ ] **Step 1: Write failing tests for single graph admission**

Create a test with one application-execution session. First create the execution-owned graph, then attempt a compatibility fallback graph for the same session without explicit compatibility scope.

Expected assertions:

```rust
assert_eq!(board.count_for_session("session-a"), 1);
assert!(second_graph_result.is_denied_or_compatibility_scoped());
```

- [ ] **Step 2: Add graph ownership to Task Service commands**

Add typed command fields only at the Task Service boundary. Do not branch on app names.

```rust
/// Describes why this task graph exists and which service may aggregate it.
/// It prevents legacy adapters, fallback decomposition, or UI helpers from
/// creating a second authoritative graph in the same application execution
/// session. The value is a service-owned category, not an application name.
pub graph_owner: TaskGraphOwner,
```

Use enum values like `ApplicationExecution`, `TaskServiceCompatibility`, and `DiagnosticOnly`.

- [ ] **Step 3: Add structured logs**

At graph create/deny/admit nodes, log bounded fields:

```rust
tracing::info!(
    application_id = %command.application_id,
    session_id = %command.session_id,
    graph_owner = ?command.graph_owner,
    trace_id = %command.trace.trace_id,
    "task graph admission evaluated"
);
```

- [ ] **Step 4: Verify task tests**

Run:

```bash
cargo test -p macaca-task --lib
```

Expected: no duplicate authoritative task graph can exist in one session.

## Task 4: Move Fallback Decomposition Behind Task Service Strategy

**Files:**
- Modify: `macaca/crates/shells/macaca-web/src/loop_manager.rs`
- Modify: `macaca/crates/services/macaca-task/src/runtime.rs`
- Test: `macaca/crates/shells/macaca-web/src/loop_manager.rs` or extracted Task Service tests.

- [ ] **Step 1: Write failing regression for fallback pollution**

Reproduce the observed shape:

1. Application execution creates coordinator/planner/coder/reviewer delegate tasks.
2. Planner compatibility fallback fails.
3. The session terminal projection still evaluates the execution-owned graph.

Expected:

```rust
assert_eq!(execution_projection.lifecycle_state, ApplicationExecutionLifecycleState::Completed);
assert_eq!(task_projection.compatibility_failures.len(), 1);
assert!(!task_projection.compatibility_failures[0].is_authoritative_terminal_failure);
```

- [ ] **Step 2: Convert Web fallback into adapter command**

Keep Web as an adapter. The fallback path calls Task Service with `TaskGraphOwner::TaskServiceCompatibility` and a trace context. It must not directly create authoritative application-execution tasks.

- [ ] **Step 3: Add logs at fallback adapter boundaries**

Log that a compatibility fallback was requested, admitted, denied, or ignored. Include service owner, session, trace, and reason code. Do not include raw prompts or provider payloads.

- [ ] **Step 4: Verify loop manager tests**

Run:

```bash
cargo test -p macaca-web loop_manager --lib
```

Expected: fallback no longer changes application-execution terminal state.

## Task 5: Fix Hosted Execution Terminal Aggregation

**Files:**
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/application_execution_hosted.rs`
- Test: `macaca/crates/runtime/macaca-runtime-host/src/application_execution_hosted_tests.rs`

- [ ] **Step 1: Write failing test for mixed host command rows**

Build a fake hosted result with:

- execution-owned task rows completed
- compatibility fallback row failed
- diagnostic row pending

Expected:

```rust
assert_eq!(signals.last().unwrap().event_type, ApplicationExecutionEventType::ExecutionCompleted);
assert_eq!(signals.last().unwrap().summary, "hosted application execution graph completed");
```

- [ ] **Step 2: Aggregate only authoritative rows**

Filter terminal aggregation by graph owner or explicit execution ownership marker. Unknown rows should emit diagnostic events but not terminal failure.

Add comments explaining that this is not an app-specific exception; it is an ownership rule for all application types.

- [ ] **Step 3: Add structured diagnostic signal**

When non-authoritative rows fail, emit a bounded warning signal with `reason_code = "non_authoritative_host_command_failed"` and counts only.

- [ ] **Step 4: Verify hosted tests**

Run:

```bash
cargo test -p macaca-runtime-host application_execution_hosted --lib
```

Expected: terminal state follows the authoritative execution graph only.

## Task 6: Force WASM And YAML Adapters Through The Same Path

**Files:**
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/host_import_bridge.rs`
- Modify: YAML workflow adapter files identified by `rg "service.agent_execution|agent.delegate|workflow" macaca/crates -n`
- Test: runtime-host tests for WASM and YAML execution adapters.

- [ ] **Step 1: Write cross-adapter equivalence tests**

For one WASM delegate and one YAML step, assert both call:

```text
service.application_execution -> service.task -> service.agent_execution
```

Expected: event phases and task ownership markers match.

- [ ] **Step 2: Replace direct semantic task creation**

Adapters may create `ApplicationExecutionEnvelope` or `AgentExecutionCommand`; they must not create independent semantic task graphs outside Task Service.

- [ ] **Step 3: Verify adapter tests**

Run:

```bash
cargo test -p macaca-runtime-host wasm_host_import --lib
cargo test -p macaca-app yaml --lib
```

Expected: WASM and YAML use the same service calls and trace phases.

## Task 7: Make Shells And App-Owned UI Projection-Only

**Files:**
- Modify: `macaca/crates/shells/macaca-web/src/routes.rs`
- Modify: `macaca/crates/shells/macaca-web/src/application_execution_routes.rs`
- Modify: `frontend/`
- Modify: `apps/codex-wasm-workbench/ui/src/WorkbenchApp.tsx`

- [ ] **Step 1: Write route tests for thin-shell behavior**

Tests assert Web routes call SDK/SystemFacade clients and do not instantiate providers, task planners, or agent runners directly.

- [ ] **Step 2: Remove UI terminal inference**

Workbench UI should render `current-state` and replayed events. Local arrays are caches only.

- [ ] **Step 3: Verify frontend/app UI tests**

Run existing frontend and app UI tests. At minimum:

```bash
npm test -- --runInBand
```

from the relevant frontend/app UI package when available.

Expected: refresh/replay does not lose state and does not infer completion from local event count.

## Task 8: End-To-End Verification And Commit

**Files:**
- Create proof notes under `docs/evidence/unify-application-execution-path/` when live verification produces sanitized evidence.

- [ ] **Step 1: Run OpenSpec validation**

```bash
openspec validate unify-application-execution-path --strict
```

Expected: valid.

- [ ] **Step 2: Run Rust verification**

```bash
cd macaca
cargo fmt
cargo test -p macaca-task --lib
cargo test -p macaca-runtime-host --lib
cargo check -p macaca-web
```

Expected: all targeted tests pass; unrelated warnings are recorded but not treated as blockers unless they identify a concrete correctness issue.

- [ ] **Step 3: Run live proof**

Start backend and frontend from newly built artifacts. Run one WASM app-owned UI task and one YAML app task. Confirm both produce:

- one authoritative execution session
- one authoritative task graph
- one terminal current-state
- replayable EventLog rows after refresh
- no duplicate fallback graph in Task Board

- [ ] **Step 4: Run GitNexus detect changes**

Run the available GitNexus detect-changes command. CRITICAL/HIGH warnings are recorded as memo only unless they identify a direct correctness issue in edited symbols.

- [ ] **Step 5: Commit**

```bash
git add openspec/changes/unify-application-execution-path docs/superpowers/plans/2026-06-07-unify-application-execution-path.md
git commit -m "spec: unify application execution path"
```

Expected: planning and OpenSpec artifacts are committed before implementation starts.

## Self-Review

- Spec coverage: The plan covers single ingress, task graph ownership, fallback migration, terminal projection, adapter convergence, shell projection-only behavior, trace/audit/logging, and verification.
- Placeholder scan: The plan contains no deferred-work markers. Every task has files, steps, commands, and expected results.
- Type consistency: Proposed ownership concepts use service-level names (`ApplicationExecution`, `TaskServiceCompatibility`, `DiagnosticOnly`) rather than app-specific or provider-specific names.
