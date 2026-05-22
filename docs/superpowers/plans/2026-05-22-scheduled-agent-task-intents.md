# Scheduled Agent Task Intents Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a generic scheduled-agent-task intent capability so users can either manually define recurring agent prompts or ask an application entry agent to create those tasks on their behalf.

**Architecture:** Introduce a serviceized Scheduled Agent Task Intent boundary that stores prompt material as controlled payload evidence, registers Scheduler jobs with payload references, and dispatches due runs through `service.agent_execution`. Scheduler remains a timing, lease, retry, and run-history service; Web/frontend and entry agents are adapters that submit typed commands.

**Tech Stack:** Rust workspace under `macaca/`, `macaca-proto` DTOs, `ServiceRuntime`, `macaca-sdk` focused clients, `macaca-web` Axum routes, Next.js/TypeScript frontend, OpenSpec, tracing, cargo tests, dependency-boundary gates.

---

## Design Constraints

- Preserve `macaca-os-architecture-governance.md`, `macaca-os-microkernel-boundaries.md`, and `macaca-os-serviceization-allowlist.md`.
- Do not store raw prompts in Scheduler jobs, run summaries, logs, snapshots, or frontend-safe responses.
- Do not hardcode application names, workflow names, provider names, model names, driver names, or business-domain logic.
- All Rust code added during implementation must include detailed English comments for ownership, operating principles, payload redaction, and audit behavior.
- Key execution nodes must log sanitized trace, audit, ids, reason codes, and result state.
- Use Command, Facade, Strategy, Memento, Observer, Decorator, Builder, and Specification patterns only where they directly clarify ownership.

## File Map

- Create `macaca/crates/foundation/macaca-proto/src/scheduled_agent_task_service.rs`: provider-neutral DTOs for intent commands, summaries, payload refs, audit refs, tool commands, and structured errors.
- Modify `macaca/crates/foundation/macaca-proto/src/lib.rs`: export scheduled-agent-task service contracts.
- Create `macaca/crates/services/macaca-scheduled-agent-task/`: service contract, unavailable provider, local provider, payload memento, audit event helpers, and tests.
- Modify `macaca/Cargo.toml` and relevant crate manifests: add the service crate without introducing new external dependencies unless existing workspace dependencies are insufficient.
- Modify `macaca/crates/runtime/macaca-runtime-host/src/lib.rs` and autonomy bootstrap files: register scheduled-agent-task provider and wire it into runtime-host composition.
- Modify `macaca/crates/runtime/macaca-runtime-host/src/autonomy_dispatch.rs`: add `SchedulerTargetCommand::AgentExecution` Strategy using payload refs and `service.agent_execution`.
- Modify `macaca/crates/facade/macaca-sdk/src/`: add a focused Scheduled Agent Task client and optional application/agent-facing helper.
- Modify `macaca/crates/shells/macaca-web/src/routes.rs` and `bootstrap.rs`: add serviceized app-scoped routes for manual task creation and inspection.
- Modify `frontend/lib/autonomy-types.ts`, `frontend/lib/autonomy.ts`, and autonomy components under `frontend/components/autonomy/`: add manual scheduled-agent-task form fields without frontend-owned scheduling semantics.
- Add or modify application/entry-agent tool projection files after locating the current tool catalog path: expose one generic tool for entry agents to create scheduled agent tasks through the service.
- Add OpenSpec and boundary tests under `macaca/crates/tests/macaca-integration-tests/tests/` and service unit tests in the new crate.

## Task 1: Service Contract

**Files:**
- Create: `macaca/crates/foundation/macaca-proto/src/scheduled_agent_task_service.rs`
- Modify: `macaca/crates/foundation/macaca-proto/src/lib.rs`
- Test: `macaca/crates/foundation/macaca-proto/src/scheduled_agent_task_service.rs`

- [ ] **Step 1: Add failing DTO tests**

Add unit tests that prove:

```rust
#[test]
fn create_command_requires_trace_prompt_and_target_agent() {
    let command = CreateScheduledAgentTaskCommand::new(
        TraceContext::new("trace-scheduled-agent-task"),
        AutonomyScope::application(ApplicationId(uuid::Uuid::nil())),
        ScheduledAgentTaskSchedule::Every { interval_ms: 60_000 },
        "technical_analyst",
        "Analyze the latest market state and record an auditable summary.",
    );
    assert!(command.is_ok());

    let missing_prompt = CreateScheduledAgentTaskCommand::new(
        TraceContext::new("trace-scheduled-agent-task"),
        AutonomyScope::application(ApplicationId(uuid::Uuid::nil())),
        ScheduledAgentTaskSchedule::Every { interval_ms: 60_000 },
        "technical_analyst",
        " ",
    );
    assert!(missing_prompt.is_err());
}

#[test]
fn safe_summary_never_contains_raw_prompt() {
    let summary = ScheduledAgentTaskSummary::redacted_fixture_for_tests(
        "task-1",
        "digest.prompt.123",
        "Daily analysis task",
    );
    let encoded = serde_json::to_string(&summary).unwrap();
    assert!(!encoded.contains("Analyze the latest market state"));
    assert!(encoded.contains("digest.prompt.123"));
}
```

Run: `cd macaca && cargo test -p macaca-proto scheduled_agent_task_service --lib`

Expected: fail because the module and types do not exist.

- [ ] **Step 2: Define provider-neutral DTOs**

Create the service contract with:

```rust
pub const SCHEDULED_AGENT_TASK_SERVICE_ID: &str = "service.scheduled_agent_task";
pub const SCHEDULED_AGENT_TASK_CREATE_COMMAND: &str = "scheduled_agent_task.create";
pub const SCHEDULED_AGENT_TASK_GET_COMMAND: &str = "scheduled_agent_task.get";
pub const SCHEDULED_AGENT_TASK_LIST_COMMAND: &str = "scheduled_agent_task.list";
pub const SCHEDULED_AGENT_TASK_CANCEL_COMMAND: &str = "scheduled_agent_task.cancel";

pub struct CreateScheduledAgentTaskCommand {
    pub trace: TraceContext,
    pub scope: AutonomyScope,
    pub schedule: ScheduledAgentTaskSchedule,
    pub target_agent: String,
    pub user_prompt: String,
    pub delegated_context: serde_json::Value,
    pub policy: ScheduledAgentTaskPolicy,
    pub metadata: BTreeMap<String, String>,
}
```

Include detailed English comments explaining that `user_prompt` is accepted only at the intent service boundary and must be converted into `AutonomyPayloadRef` before Scheduler receives the job.

- [ ] **Step 3: Add Builder helpers**

Implement a small Builder-style API for optional context and metadata:

```rust
impl CreateScheduledAgentTaskCommand {
    pub fn with_delegated_context(mut self, context: serde_json::Value) -> Self {
        self.delegated_context = context;
        self
    }

    pub fn with_metadata(mut self, metadata: BTreeMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }
}
```

- [ ] **Step 4: Export the module**

Add `pub mod scheduled_agent_task_service;` and `pub use scheduled_agent_task_service::*;` in `macaca-proto/src/lib.rs`.

- [ ] **Step 5: Verify proto contract**

Run: `cd macaca && cargo test -p macaca-proto scheduled_agent_task_service --lib`

Expected: pass.

## Task 2: Payload And Audit Memento

**Files:**
- Create: `macaca/crates/services/macaca-scheduled-agent-task/src/lib.rs`
- Create: `macaca/crates/services/macaca-scheduled-agent-task/src/service_contract.rs`
- Create: `macaca/crates/services/macaca-scheduled-agent-task/src/local_provider.rs`
- Create: `macaca/crates/services/macaca-scheduled-agent-task/src/payload_store.rs`
- Create: `macaca/crates/services/macaca-scheduled-agent-task/src/audit.rs`
- Test: `macaca/crates/services/macaca-scheduled-agent-task/src/local_provider.rs`

- [ ] **Step 1: Add failing provider tests**

Write tests proving create stores raw prompt only in the provider-local payload store, returns a redacted summary, creates an audit id, and registers a Scheduler job with `SchedulerTargetCommand::AgentExecution`.

Run: `cd macaca && cargo test -p macaca-scheduled-agent-task`

Expected: fail until crate and provider exist.

- [ ] **Step 2: Implement provider-local payload memento**

Create a small in-memory store:

```rust
struct StoredScheduledAgentPayload {
    prompt: String,
    delegated_context: serde_json::Value,
    digest: String,
    redacted_summary: String,
    created_at: DateTime<Utc>,
}
```

Add English comments explaining that this is a local provider memento and not a Scheduler payload field.

- [ ] **Step 3: Implement sanitized audit evidence**

Record audit ids such as `audit.scheduled_agent_task.created.N`, including trace id, task id, scheduler job id, payload digest, target agent, and safe reason code. Do not include raw prompt or raw delegated context.

- [ ] **Step 4: Implement service provider create flow**

The create flow must:

1. Validate trace, scope, schedule, prompt, and target agent.
2. Persist payload memento and digest.
3. Build `AutonomyPayloadRef`.
4. Build `SchedulerTargetCommand::AgentExecution`.
5. Register the job through the injected Scheduler client or service boundary.
6. Return `ScheduledAgentTaskCommandResult` with trace id, audit id, scheduler job id, payload digest, and safe metadata.

- [ ] **Step 5: Add structured unavailable provider**

The unavailable provider must return explicit unavailable results and logs, never fake success.

- [ ] **Step 6: Verify service crate**

Run: `cd macaca && cargo test -p macaca-scheduled-agent-task`

Expected: pass with tests covering redaction, audit ids, Scheduler target construction, unavailable behavior, and prompt exclusion from summaries.

## Task 3: Runtime Dispatch

**Files:**
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/autonomy_dispatch.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/lib.rs`
- Modify: runtime-host autonomy bootstrap files that register local services
- Test: `macaca/crates/runtime/macaca-runtime-host/src/autonomy_dispatch.rs`

- [ ] **Step 1: Write failing dispatch test**

Add a test that feeds `SchedulerTargetCommand::AgentExecution` with an `AutonomyPayloadRef`, uses a fake payload resolver, and asserts runtime-host sends one `AgentExecutionCommand` to `service.agent_execution`.

Expected command fields:

```rust
assert_eq!(command.execution_intent, AgentExecutionIntent::TaskWorker);
assert_eq!(command.user_prompt, "Analyze the market and record result.");
assert_eq!(command.metadata["scheduler_run_source"], "service.scheduler");
assert_eq!(command.metadata["payload_digest"], "digest.prompt.123");
```

- [ ] **Step 2: Implement AgentExecution dispatch Strategy**

Add a Strategy object or helper that:

1. Resolves `AutonomyPayloadRef` through the scheduled-agent-task service.
2. Builds `AgentExecutionCommand`.
3. Calls `ServiceRuntime` with `AGENT_EXECUTION_SERVICE_ID`.
4. Returns `AutonomyDispatchOutcome::succeeded`, retryable failure, or skipped unavailable.

Do not parse business intent or inspect application names.

- [ ] **Step 3: Add logs**

Log sanitized nodes:

- `scheduled agent dispatch payload resolved`
- `scheduled agent dispatch invoking agent execution service`
- `scheduled agent dispatch completed`
- `scheduled agent dispatch failed`

Each log includes trace id, scheduler job/run when available, payload digest, target agent, and result code.

- [ ] **Step 4: Verify runtime-host**

Run: `cd macaca && cargo test -p macaca-runtime-host autonomy_dispatch`

Expected: pass.

## Task 4: UI Manual Entry

**Files:**
- Modify: `frontend/lib/autonomy-types.ts`
- Modify: `frontend/lib/autonomy.ts`
- Modify: `frontend/components/autonomy/ScheduleManagerPanel.tsx`
- Create: `frontend/components/autonomy/ScheduledAgentTaskEditorDrawer.tsx`
- Create or modify focused UI files under `frontend/components/autonomy/`
- Modify: `macaca/crates/shells/macaca-web/src/routes.rs`
- Modify: `macaca/crates/shells/macaca-web/src/bootstrap.rs`
- Test: frontend lint/typecheck and Web route tests

- [ ] **Step 1: Add frontend types**

Add a draft type:

```ts
export interface ScheduledAgentTaskDraft {
  name: string;
  target_agent: string;
  task_prompt: string;
  interval_secs: number;
  metadata: Record<string, string>;
}
```

Keep raw prompt only in mutation requests. Do not include raw prompt in list summaries.

- [ ] **Step 2: Add Web route DTOs**

Add app-scoped routes:

- `POST /api/apps/{app_id}/autonomy/scheduled-agent-tasks`
- `GET /api/apps/{app_id}/autonomy/scheduled-agent-tasks`
- `GET /api/apps/{app_id}/autonomy/scheduled-agent-tasks/{task_id}`
- `DELETE /api/apps/{app_id}/autonomy/scheduled-agent-tasks/{task_id}`

Routes only validate HTTP shape, create trace context, and call SDK/focused client commands.

- [ ] **Step 3: Add drawer component**

Create a focused drawer that contains only manual scheduled-agent-task fields. Keep the component under 200 lines; split controls if necessary. The component does not compute scheduler semantics.

- [ ] **Step 4: Add safe summaries**

List views render task id, target agent, schedule summary, lifecycle, payload digest, trace id, audit id, last run, and result status. They must not render raw prompt.

- [ ] **Step 5: Verify UI and Web**

Run:

```bash
cd frontend && npm run lint
cd macaca && cargo check -p macaca-web
```

Expected: pass.

## Task 5: Entry Agent Tool

**Files:**
- Locate and modify the current generic tool/capability projection files for entry agents.
- Add tests beside the existing tool projection tests.

- [ ] **Step 1: Locate current tool catalog boundary**

Run:

```bash
rg -n "tool catalog|ToolCatalog|capability tool|AgentContextSnapshot|visible skills|macaca:agent" macaca/crates -g '!target'
```

Choose the existing serviceized context/tool projection path. Do not add tool semantics inside frontend or Scheduler.

- [ ] **Step 2: Add generic tool declaration**

Expose a generic tool such as `scheduled_agent_task.create` to eligible entry agents. The tool schema accepts:

- `target_agent`
- `task_prompt`
- `schedule`
- `metadata`
- optional bounded `delegated_context`

The tool implementation calls the scheduled-agent-task service through `ServiceRuntime` or SDK client.

- [ ] **Step 3: Add policy gates**

Require application scope, trace, target agent, and capability permission before creating a task. Return denied/unavailable states when the service or permission is absent.

- [ ] **Step 4: Add logs and audit**

Log sanitized nodes:

- `entry agent requested scheduled task creation`
- `entry agent scheduled task command admitted`
- `entry agent scheduled task command denied`
- `entry agent scheduled task command completed`

Never log raw prompt or raw delegated context.

- [ ] **Step 5: Verify agent-created task path**

Add a test where a fake entry agent tool call creates the same service command as the UI route. Assert both paths produce the same DTO shape.

## Task 6: Tests And Boundary Gates

**Files:**
- Modify: `macaca/crates/tests/macaca-integration-tests/tests/serviceization_dependency_gate.rs` or equivalent gate files
- Modify: serviceization escape-hatch tests
- Add integration tests for scheduled agent task lifecycle
- Update docs under `macaca/docs/` if service ownership docs need a new section
- Update `openspec/changes/add-scheduled-agent-task-intents/tasks.md` truthfully during implementation

- [ ] **Step 1: Add dependency-boundary tests**

Gate requirements:

- Scheduler provider does not read raw prompt fields.
- Web/frontend do not call legacy `/api/apps/{app_id}/schedules` for scheduled-agent-task creation.
- Frontend does not encode application-specific templates or workflow names.
- Runtime-host dispatch uses service DTOs and `ServiceRuntime`.
- Service providers do not import presentation shell crates.

- [ ] **Step 2: Add redaction tests**

Search serialized responses, Scheduler summaries, run summaries, logs where practical, and audit summaries for the raw prompt fixture string. Expected: only controlled payload store test helpers contain it.

- [ ] **Step 3: Add integration test**

Test chain:

1. Create scheduled agent task with prompt.
2. Verify safe summary has digest but no prompt.
3. Force or materialize Scheduler due run.
4. Lease and dispatch.
5. Verify `service.agent_execution` received prompt.
6. Verify trace/audit chain includes task id, job id, run id, execution trace, result status.

- [ ] **Step 4: Run validation**

Run:

```bash
cd macaca && cargo fmt
cd macaca && cargo test -p macaca-proto scheduled_agent_task_service --lib
cd macaca && cargo test -p macaca-scheduled-agent-task
cd macaca && cargo test -p macaca-runtime-host autonomy_dispatch
cd macaca && cargo test -p macaca-integration-tests scheduled_agent_task
openspec validate add-scheduled-agent-task-intents --strict
git diff --check
```

Expected: all pass before marking OpenSpec tasks complete.

## Self-Review

- Spec coverage: service contract, payload/audit, runtime dispatch, UI, entry agent tool, tests, and boundary gates each have a dedicated task.
- Placeholder scan: no implementation task depends on undefined business logic or application-specific names.
- Type consistency: `CreateScheduledAgentTaskCommand`, `ScheduledAgentTaskSummary`, `AutonomyPayloadRef`, `SchedulerTargetCommand::AgentExecution`, and `AgentExecutionCommand` are consistently named across tasks.
- Boundary check: Scheduler never owns raw prompt storage or interpretation; frontend and Web are adapters; entry agents use a generic tool; runtime-host owns dispatch strategy; agent execution owns LLM invocation.
