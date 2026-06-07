# macaca-kernel Design Pattern Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 对 `macaca-kernel` 做五个小切片的设计模式渐进式重构，先收敛 executor event、scheduler、status、executor payload 和 kernel facade 边界，保持现有行为 1:1。

**Architecture:** Additive-first。先新增 kernel-owned helper/factory/policy/builder，再迁移直接调用点；旧入口保留兼容并可标记 deprecated，避免一次性改动 web/app/sdk/cli。`ExecutorEvent`、SSE/EventLog、task execution、agent status 和 scheduler 行为不得改变。

**Tech Stack:** Rust, Tokio, `async-trait`, `macaca-kernel`, `macaca-web`, `macaca-app`, `macaca-sdk`, `macaca-cli`, OpenSpec, GitNexus, cargo test/check.

---

## Current Context

`macaca-kernel` 当前核心文件：

- `macaca/crates/macaca-kernel/src/kernel.rs`：`Kernel` facade，固定构造 `SimpleScheduler`。
- `macaca/crates/macaca-kernel/src/scheduler.rs`：`Scheduler` trait + `SimpleScheduler` strategy。
- `macaca/crates/macaca-kernel/src/status.rs`：`AgentStatusTracker`。
- `macaca/crates/macaca-kernel/src/executor/mod.rs`：executor public contract，含 `ExecutorEvent`、`TaskResult`、`AgentRunner`。
- `macaca/crates/macaca-kernel/src/executor/app_executor.rs`：`ApplicationExecutor`，当前 1268 行。
- `macaca/crates/macaca-kernel/src/executor/worker.rs`：legacy `TaskExecutor`，也构造 executor/system events。
- `macaca/crates/macaca-web/src/loop_manager.rs`：当前有 web-local `executor_task_started` / `executor_task_completed` / `executor_task_failed` helper，应迁移到 kernel。

主要消费方：

- `macaca-web`：`Kernel`、`ApplicationExecutorRegistry`、`ApplicationExecutor`、`ExecutorEvent`。
- `macaca-app`：`Kernel::new`。
- `macaca-sdk`：`Kernel::new`。
- `macaca-cli`：`Kernel::new`。
- `macaca-integration-tests`：kernel lifecycle 与 app tests。

Known hotspots:

- `app_executor.rs` 超过 500 行，但不应第一步直接拆。
- `fork_manager.rs` 超过 500 行，但本轮不触碰，避免扩大范围。
- `ExecutorEvent` 是 UI trace、SSE、EventLog、session restore 的关键 contract，必须保持 payload shape。

## Superpowers Brainstorm Summary

推荐方案：先做低风险 helper/factory 切片，再逐步迁移调用方。

不推荐直接拆 `ApplicationExecutor`，因为它是 worker 执行、fork resume、broadcast、queue result 的热路径。也不推荐优先大改 `Kernel::new`，因为消费点多，风险高。

五个切片顺序：

1. `ExecutorEvent` lifecycle helper。
2. `SchedulerFactory`。
3. `AgentStatusTransitionPolicy`。
4. Executor event publisher / payload boundary。
5. `KernelBuilder` / facade 收口。

## File Map

### OpenSpec

- Create: `openspec/changes/refactor-macaca-kernel-patterns/proposal.md`
- Create: `openspec/changes/refactor-macaca-kernel-patterns/design.md`
- Create: `openspec/changes/refactor-macaca-kernel-patterns/tasks.md`
- Create: `openspec/changes/refactor-macaca-kernel-patterns/specs/macaca-kernel-patterns/spec.md`

### Kernel files

- Create: `macaca/crates/macaca-kernel/src/executor/event_factory.rs`
  - Responsibility: canonical `ExecutorEvent` and `TaskResult` lifecycle construction.
- Modify: `macaca/crates/macaca-kernel/src/executor/mod.rs`
  - Export event factory and keep existing event/result types.
- Modify: `macaca/crates/macaca-kernel/src/executor/worker.rs`
  - Replace direct lifecycle event construction with factory in a narrow slice.
- Modify: `macaca/crates/macaca-kernel/src/executor/app_executor.rs`
  - Replace direct lifecycle event/result construction with factory/publisher in later slices.
- Create: `macaca/crates/macaca-kernel/src/scheduler_factory.rs`
  - Responsibility: construct scheduler strategies from a small additive config enum.
- Modify: `macaca/crates/macaca-kernel/src/scheduler.rs`
  - Keep `Scheduler` and `SimpleScheduler`; optionally add `SchedulerKind`.
- Create: `macaca/crates/macaca-kernel/src/status_transition.rs`
  - Responsibility: explicit status/activity transition policy.
- Modify: `macaca/crates/macaca-kernel/src/status.rs`
  - Delegate convenience helpers to transition policy without changing behavior.
- Create: `macaca/crates/macaca-kernel/src/kernel_builder.rs`
  - Responsibility: additive `KernelBuilder` that preserves `Kernel::new` behavior.
- Modify: `macaca/crates/macaca-kernel/src/kernel.rs`
  - Add internal constructor path if needed; keep public `Kernel::new` compatible.
- Modify: `macaca/crates/macaca-kernel/src/lib.rs`
  - Export new additive primitives.

### Upper consumer files

- Modify: `macaca/crates/macaca-web/src/loop_manager.rs`
  - Replace web-local executor event helpers with `macaca-kernel` helper calls.
- Later proposal only: migrate `macaca-web`, `macaca-app`, `macaca-sdk`, `macaca-cli` to `KernelBuilder`.

## Task 1: Create OpenSpec Change

**Files:**

- Create: `openspec/changes/refactor-macaca-kernel-patterns/proposal.md`
- Create: `openspec/changes/refactor-macaca-kernel-patterns/design.md`
- Create: `openspec/changes/refactor-macaca-kernel-patterns/tasks.md`
- Create: `openspec/changes/refactor-macaca-kernel-patterns/specs/macaca-kernel-patterns/spec.md`

- [ ] **Step 1: Review OpenSpec context**

Run:

```bash
cd /Users/quantum/Code/dev/agent
openspec list
openspec list --specs
```

Expected:

```text
No existing refactor-macaca-kernel-patterns change
Related task/agent/framework/runtime changes may be active
```

- [ ] **Step 2: Create proposal**

Create `openspec/changes/refactor-macaca-kernel-patterns/proposal.md`:

```markdown
# Change: Refactor macaca-kernel with design pattern primitives

## Why

`macaca-kernel` is the Agent OS coordination center. Executor event construction, scheduler selection, agent status transitions, and kernel construction are currently scattered across kernel and web call sites, which makes trace/EventLog correctness and future scheduler/executor extensions harder to maintain.

## What Changes

- Add canonical executor lifecycle event/result factory helpers.
- Add scheduler factory primitives while preserving `SimpleScheduler` behavior.
- Add explicit agent status transition policy helpers.
- Move executor payload construction toward kernel-owned primitives.
- Add an additive kernel builder/facade entry while keeping `Kernel::new` compatible.

## Impact

- Affected specs: `macaca-kernel-patterns`
- Affected code: `macaca-kernel`, `macaca-web` loop manager helper usage
- Non-impact: no application-specific logic, no scheduler behavior change, no `ExecutorEvent` payload shape change, no EventLog/SSE behavior change.
```

- [ ] **Step 3: Create design**

Create `openspec/changes/refactor-macaca-kernel-patterns/design.md`:

```markdown
## Context

`macaca-kernel` owns the system coordination surface: kernel facade, registry, scheduler, status tracker, executor, event bus, and orchestration. It is consumed by web, app, sdk, cli, and integration tests. `ApplicationExecutor` and `fork_manager` are large files, but this change prioritizes low-risk primitive extraction before file splitting.

## Goals

- Keep behavior 1:1 compatible.
- Preserve `ExecutorEvent` and `TaskResult` payload shape.
- Move lifecycle event construction into kernel-owned helpers.
- Make scheduler construction extensible without changing the selected scheduler.
- Make status transitions explicit without introducing new lifecycle semantics.
- Add kernel builder/facade primitives without removing `Kernel::new`.

## Non-Goals

- Do not split `ApplicationExecutor` or `ForkManager` into many files in this change.
- Do not change task scheduling order.
- Do not change worker supervision, fork resume, queue semantics, SSE, EventLog, session restore, or web trace behavior.
- Do not hardcode application, workflow, driver, or agent names.
- Do not introduce third-party dependencies.

## Decisions

- Use Factory Method/Builder for executor lifecycle payloads.
- Use Strategy + Factory for scheduler construction, initially returning only `SimpleScheduler`.
- Use State policy helpers for existing agent activity/status transitions.
- Keep `Kernel::new` as compatibility facade and add `KernelBuilder` as an additive entry.

## Risks / Mitigations

- `ExecutorEvent` regressions can break UI trace and history restore. Mitigation: snapshot-style unit tests for every factory method and existing SSE/EventLog checks.
- `ApplicationExecutor` edits can affect live worker execution. Mitigation: only replace payload construction first; do not move supervisor loop logic.
- `Kernel::new` has many consumers. Mitigation: keep it unchanged externally and migrate consumers in a later proposal.
```

- [ ] **Step 4: Create tasks**

Create `openspec/changes/refactor-macaca-kernel-patterns/tasks.md`:

```markdown
## 1. Preparation

- [ ] 1.1 Run GitNexus impact for `ExecutorEvent`, `TaskResult`, `TaskExecutor::execute_task`, `ApplicationExecutor`, `SimpleScheduler`, `AgentStatusTracker`, and `Kernel::new` before editing related symbols.
- [ ] 1.2 Run baseline kernel tests.
- [ ] 1.3 Run baseline web/kernel targeted check.

## 2. Executor lifecycle helper

- [ ] 2.1 Add `executor/event_factory.rs`.
- [ ] 2.2 Add factory unit tests for started/completed/failed/result payloads.
- [ ] 2.3 Export factory from `executor/mod.rs`.
- [ ] 2.4 Migrate web-local helper usage to kernel helper.
- [ ] 2.5 Migrate legacy `TaskExecutor::execute_task` direct lifecycle construction.

## 3. Scheduler factory

- [ ] 3.1 Add `SchedulerKind` and `SchedulerFactory`.
- [ ] 3.2 Keep default factory output equivalent to `SimpleScheduler`.
- [ ] 3.3 Add selection parity tests.

## 4. Agent status transition policy

- [ ] 4.1 Add transition policy tests for current behavior.
- [ ] 4.2 Add `AgentStatusTransitionPolicy`.
- [ ] 4.3 Route status convenience helpers through the policy.

## 5. Executor payload boundary

- [ ] 5.1 Replace selected `ApplicationExecutor` event/result construction with factory calls.
- [ ] 5.2 Keep broadcast/event_tx behavior unchanged.
- [ ] 5.3 Add regression tests for emitted event fields.

## 6. Kernel builder/facade

- [ ] 6.1 Add `KernelBuilder`.
- [ ] 6.2 Make `KernelBuilder::build` match `Kernel::new` defaults.
- [ ] 6.3 Keep `Kernel::new` compatible by delegating internally if safe.
- [ ] 6.4 Add builder tests.

## 7. Verification

- [ ] 7.1 Run `cargo fmt`.
- [ ] 7.2 Run `cargo test -p macaca-kernel -- --nocapture`.
- [ ] 7.3 Run `cargo test -p macaca-integration-tests kernel -- --nocapture`.
- [ ] 7.4 Run `cargo check -p macaca-kernel -p macaca-web -p macaca-app -p macaca-sdk -p macaca-cli`.
- [ ] 7.5 Run `openspec validate refactor-macaca-kernel-patterns --strict`.
- [ ] 7.6 Run `gitnexus_detect_changes(scope: "all")`.
```

- [ ] **Step 5: Create delta spec**

Create `openspec/changes/refactor-macaca-kernel-patterns/specs/macaca-kernel-patterns/spec.md`:

```markdown
## ADDED Requirements

### Requirement: Canonical Executor Lifecycle Event Construction

Kernel SHALL provide canonical helpers for constructing executor lifecycle events and task results without changing `ExecutorEvent` or `TaskResult` payload shape.

#### Scenario: Completed event preserves fields

- **WHEN** a completed executor event is created through the kernel helper
- **THEN** it contains the original task id, agent name, success flag, output, empty error, artifacts, completion timestamp, and token usage fields compatible with existing consumers.

### Requirement: Scheduler Factory Preserves Default Scheduling

Kernel SHALL provide a scheduler factory that preserves current `SimpleScheduler` selection behavior by default.

#### Scenario: Default scheduler matches simple scheduler

- **WHEN** the default scheduler factory is used
- **THEN** it selects the same agent as `SimpleScheduler` for the same registry and task fixture.

### Requirement: Explicit Agent Status Transition Policy

Kernel SHALL expose explicit helpers for existing agent activity/status transitions without introducing new lifecycle semantics.

#### Scenario: Thinking and idle transitions remain compatible

- **WHEN** an agent is marked thinking and then idle
- **THEN** its activity and current task fields match the current `AgentStatusTracker` behavior.

### Requirement: Kernel Builder Is Additive

Kernel SHOULD provide an additive builder/facade construction entry while keeping `Kernel::new` compatible.

#### Scenario: Builder matches Kernel::new defaults

- **WHEN** a kernel is built through the new builder with the same config, llm, and tools
- **THEN** registry capacity, scheduler behavior, and initial status behavior match `Kernel::new`.
```

- [ ] **Step 6: Validate OpenSpec**

Run:

```bash
openspec validate refactor-macaca-kernel-patterns --strict
```

Expected:

```text
Change 'refactor-macaca-kernel-patterns' is valid
```

## Task 2: Baseline and Impact Analysis

**Files:**

- Read-only: `macaca/crates/macaca-kernel/src/**/*`
- Read-only: `macaca/crates/macaca-web/src/loop_manager.rs`

- [ ] **Step 1: Run baseline tests**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-kernel -- --nocapture
```

Expected:

```text
all macaca-kernel tests pass
```

- [ ] **Step 2: Run targeted check**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo check -p macaca-kernel -p macaca-web -p macaca-app -p macaca-sdk -p macaca-cli
```

Expected:

```text
command exits 0
existing unrelated warnings may remain
```

- [ ] **Step 3: Run GitNexus impact before code edits**

Run impact for symbols before editing each related slice:

```text
impact target: ExecutorEvent, direction: upstream
impact target: TaskResult, direction: upstream
impact target: execute_task, direction: upstream
impact target: ApplicationExecutor, direction: upstream
impact target: SimpleScheduler, direction: upstream
impact target: AgentStatusTracker, direction: upstream
impact target: Kernel, direction: upstream
```

Expected:

- Warn and pause if any result is HIGH or CRITICAL.
- `Kernel` and `ApplicationExecutor` are likely high blast-radius symbols.

- [ ] **Step 4: Record file sizes**

Run:

```bash
cd /Users/quantum/Code/dev/agent
wc -l macaca/crates/macaca-kernel/src/*.rs macaca/crates/macaca-kernel/src/**/*.rs 2>/dev/null | sort -nr | head -20
```

Expected:

```text
app_executor.rs and fork_manager.rs are over 500 lines; this change records the issue but only performs low-risk primitive extraction first.
```

## Task 3: ExecutorEvent Lifecycle Helper

**Files:**

- Create: `macaca/crates/macaca-kernel/src/executor/event_factory.rs`
- Modify: `macaca/crates/macaca-kernel/src/executor/mod.rs`
- Modify: `macaca/crates/macaca-web/src/loop_manager.rs`
- Modify: `macaca/crates/macaca-kernel/src/executor/worker.rs`

- [ ] **Step 1: Create event factory tests first**

Create `macaca/crates/macaca-kernel/src/executor/event_factory.rs` with tests and minimal failing references:

```rust
use chrono::Utc;

use super::{ExecutorEvent, TaskResult, TokenUsage};
use crate::executor::TaskId;

#[derive(Debug, Clone)]
pub struct ExecutorEventFactory {
    task_id: TaskId,
    agent: String,
}

impl ExecutorEventFactory {
    pub fn new(task_id: TaskId, agent: impl Into<String>) -> Self {
        Self {
            task_id,
            agent: agent.into(),
        }
    }

    pub fn started(&self) -> ExecutorEvent {
        ExecutorEvent::TaskStarted {
            task_id: self.task_id,
            agent: self.agent.clone(),
        }
    }

    pub fn success_result(&self, output: impl Into<String>) -> TaskResult {
        TaskResult {
            task_id: self.task_id,
            success: true,
            output: output.into(),
            error: None,
            artifacts: vec![],
            completed_at: Utc::now(),
            tokens_used: None,
        }
    }

    pub fn failed_result(&self, error: impl Into<String>) -> TaskResult {
        TaskResult {
            task_id: self.task_id,
            success: false,
            output: String::new(),
            error: Some(error.into()),
            artifacts: vec![],
            completed_at: Utc::now(),
            tokens_used: None,
        }
    }

    pub fn completed(&self, output: impl Into<String>) -> ExecutorEvent {
        self.completed_with_result(self.success_result(output))
    }

    pub fn completed_with_result(&self, mut result: TaskResult) -> ExecutorEvent {
        result.task_id = self.task_id;
        ExecutorEvent::TaskCompleted {
            task_id: self.task_id,
            agent: self.agent.clone(),
            result,
        }
    }

    pub fn failed(&self, error: impl Into<String>) -> ExecutorEvent {
        ExecutorEvent::TaskFailed {
            task_id: self.task_id,
            agent: self.agent.clone(),
            error: error.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn started_preserves_task_and_agent() {
        let task_id = TaskId::new();
        let event = ExecutorEventFactory::new(task_id, "planner").started();

        match event {
            ExecutorEvent::TaskStarted { task_id: got, agent } => {
                assert_eq!(got, task_id);
                assert_eq!(agent, "planner");
            }
            other => panic!("expected TaskStarted, got {other:?}"),
        }
    }

    #[test]
    fn completed_preserves_result_fields() {
        let task_id = TaskId::new();
        let event = ExecutorEventFactory::new(task_id, "backend").completed("done");

        match event {
            ExecutorEvent::TaskCompleted {
                task_id: got,
                agent,
                result,
            } => {
                assert_eq!(got, task_id);
                assert_eq!(agent, "backend");
                assert_eq!(result.task_id, task_id);
                assert!(result.success);
                assert_eq!(result.output, "done");
                assert_eq!(result.error, None);
                assert!(result.artifacts.is_empty());
                assert!(result.tokens_used.is_none());
            }
            other => panic!("expected TaskCompleted, got {other:?}"),
        }
    }

    #[test]
    fn completed_with_result_overwrites_task_id() {
        let task_id = TaskId::new();
        let wrong_task_id = TaskId::new();
        let result = TaskResult {
            task_id: wrong_task_id,
            success: true,
            output: "done".into(),
            error: None,
            artifacts: vec!["artifact.txt".into()],
            completed_at: Utc::now(),
            tokens_used: Some(TokenUsage {
                prompt_tokens: 1,
                completion_tokens: 2,
                total_tokens: 3,
            }),
        };

        let event = ExecutorEventFactory::new(task_id, "frontend").completed_with_result(result);

        match event {
            ExecutorEvent::TaskCompleted { result, .. } => {
                assert_eq!(result.task_id, task_id);
                assert_eq!(result.artifacts, vec!["artifact.txt"]);
                assert_eq!(result.tokens_used.unwrap().total_tokens, 3);
            }
            other => panic!("expected TaskCompleted, got {other:?}"),
        }
    }

    #[test]
    fn failed_preserves_error() {
        let task_id = TaskId::new();
        let event = ExecutorEventFactory::new(task_id, "frontend").failed("boom");

        match event {
            ExecutorEvent::TaskFailed {
                task_id: got,
                agent,
                error,
            } => {
                assert_eq!(got, task_id);
                assert_eq!(agent, "frontend");
                assert_eq!(error, "boom");
            }
            other => panic!("expected TaskFailed, got {other:?}"),
        }
    }
}
```

- [ ] **Step 2: Export the factory**

Modify `macaca/crates/macaca-kernel/src/executor/mod.rs`:

```rust
pub mod event_factory;
```

and re-export:

```rust
pub use event_factory::ExecutorEventFactory;
```

- [ ] **Step 3: Run factory tests**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-kernel executor::event_factory -- --nocapture
```

Expected:

```text
4 tests pass
```

- [ ] **Step 4: Replace web-local helpers**

In `macaca/crates/macaca-web/src/loop_manager.rs`, replace helper bodies with kernel helper calls:

```rust
fn executor_task_started(task_id: macaca_proto::TaskId, agent: &str) -> ExecutorEvent {
    macaca_kernel::executor::ExecutorEventFactory::new(task_id, agent).started()
}

fn executor_task_completed(
    task_id: macaca_proto::TaskId,
    agent: &str,
    output: impl Into<String>,
) -> ExecutorEvent {
    macaca_kernel::executor::ExecutorEventFactory::new(task_id, agent).completed(output)
}

fn executor_task_failed(
    task_id: macaca_proto::TaskId,
    agent: &str,
    error: impl Into<String>,
) -> ExecutorEvent {
    macaca_kernel::executor::ExecutorEventFactory::new(task_id, agent).failed(error)
}
```

- [ ] **Step 5: Replace legacy worker direct construction**

In `macaca/crates/macaca-kernel/src/executor/worker.rs`, inside `execute_task`, create one factory:

```rust
let events = crate::executor::ExecutorEventFactory::new(task_id, to_agent.clone());
```

Use:

```rust
self.event_tx.send(events.started()).await
```

and:

```rust
self.event_tx.send(events.completed_with_result(task_result))
```

and:

```rust
self.event_tx.send(events.failed(error))
```

Do not change `SystemEvent` emission in this step.

- [ ] **Step 6: Run targeted tests**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-kernel executor -- --nocapture
cargo check -p macaca-web
```

Expected:

```text
tests pass and macaca-web compiles
```

## Task 4: SchedulerFactory

**Files:**

- Create: `macaca/crates/macaca-kernel/src/scheduler_factory.rs`
- Modify: `macaca/crates/macaca-kernel/src/scheduler.rs`
- Modify: `macaca/crates/macaca-kernel/src/kernel.rs`
- Modify: `macaca/crates/macaca-kernel/src/lib.rs`

- [ ] **Step 1: Add scheduler kind and factory**

Create `macaca/crates/macaca-kernel/src/scheduler_factory.rs`:

```rust
use crate::scheduler::{Scheduler, SimpleScheduler};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerKind {
    Simple,
}

impl Default for SchedulerKind {
    fn default() -> Self {
        Self::Simple
    }
}

pub struct SchedulerFactory;

impl SchedulerFactory {
    pub fn build(kind: SchedulerKind) -> Box<dyn Scheduler> {
        match kind {
            SchedulerKind::Simple => Box::new(SimpleScheduler),
        }
    }
}
```

- [ ] **Step 2: Export factory**

Modify `macaca/crates/macaca-kernel/src/lib.rs`:

```rust
pub mod scheduler_factory;
pub use scheduler_factory::{SchedulerFactory, SchedulerKind};
```

- [ ] **Step 3: Use factory in Kernel::new**

In `macaca/crates/macaca-kernel/src/kernel.rs`, replace:

```rust
scheduler: Box::new(SimpleScheduler),
```

with:

```rust
scheduler: crate::scheduler_factory::SchedulerFactory::build(Default::default()),
```

Remove the direct `SimpleScheduler` import if it becomes unused.

- [ ] **Step 4: Add parity test**

In `scheduler_factory.rs`, add:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::AgentRegistry;
    use chrono::Utc;
    use macaca_proto::{
        AgentId, AgentManifest, AgentState, Capability, Permission, PermissionLevel, Task,
        TaskId, TaskPriority, TaskStatus,
    };

    fn make_task(description: &str) -> Task {
        Task {
            id: TaskId::new(),
            description: description.into(),
            status: TaskStatus::Pending,
            priority: TaskPriority::Normal,
            assigned_agent: None,
            subtasks: vec![],
            parent: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn default_factory_returns_simple_scheduler_behavior() {
        let registry = AgentRegistry::new(10);
        let task = make_task("anything");
        let scheduler = SchedulerFactory::build(SchedulerKind::default());

        let selected = scheduler.select_agent(&registry, &task).await.unwrap();
        assert_eq!(selected, None);
    }
}
```

If imports are unused after compilation, trim them.

- [ ] **Step 5: Run scheduler tests**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-kernel scheduler -- --nocapture
```

Expected:

```text
scheduler and scheduler_factory tests pass
```

## Task 5: Agent Status Transition Policy

**Files:**

- Create: `macaca/crates/macaca-kernel/src/status_transition.rs`
- Modify: `macaca/crates/macaca-kernel/src/status.rs`
- Modify: `macaca/crates/macaca-kernel/src/lib.rs`

- [ ] **Step 1: Add policy type**

Create `macaca/crates/macaca-kernel/src/status_transition.rs`:

```rust
use macaca_proto::{AgentActivity, AgentRuntimeStatus, AgentState};

pub struct AgentStatusTransitionPolicy;

impl AgentStatusTransitionPolicy {
    pub fn apply_state(status: &mut AgentRuntimeStatus, state: AgentState) {
        status.state = state;
        status.updated_at = chrono::Utc::now();
    }

    pub fn apply_activity(status: &mut AgentRuntimeStatus, activity: AgentActivity) {
        status.activity = activity;
        status.updated_at = chrono::Utc::now();
    }

    pub fn apply_idle(status: &mut AgentRuntimeStatus) {
        status.activity = AgentActivity::Idle;
        status.current_task = None;
        status.updated_at = chrono::Utc::now();
    }
}
```

- [ ] **Step 2: Export policy**

Modify `macaca/crates/macaca-kernel/src/lib.rs`:

```rust
pub mod status_transition;
pub use status_transition::AgentStatusTransitionPolicy;
```

- [ ] **Step 3: Route status tracker through policy**

In `macaca/crates/macaca-kernel/src/status.rs`, replace direct assignments in `update_state`:

```rust
status.state = state;
status.updated_at = Utc::now();
```

with:

```rust
crate::status_transition::AgentStatusTransitionPolicy::apply_state(status, state);
```

Replace direct assignments in `update_activity`:

```rust
status.activity = activity;
status.updated_at = Utc::now();
```

with:

```rust
crate::status_transition::AgentStatusTransitionPolicy::apply_activity(status, activity);
```

Update `set_idle` to update both activity and current task in one write lock if practical:

```rust
pub async fn set_idle(&self, agent_id: &AgentId) {
    let mut statuses = self.statuses.write().await;
    if let Some(status) = statuses.get_mut(agent_id) {
        crate::status_transition::AgentStatusTransitionPolicy::apply_idle(status);
    }
}
```

- [ ] **Step 4: Add transition tests**

Add tests in `status_transition.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use macaca_proto::{AgentId, AgentRuntimeStatus};

    fn status() -> AgentRuntimeStatus {
        AgentRuntimeStatus {
            agent_id: AgentId::new(),
            name: "agent".into(),
            state: AgentState::Created,
            activity: AgentActivity::Idle,
            updated_at: Utc::now(),
            current_task: Some("task".into()),
        }
    }

    #[test]
    fn idle_clears_current_task() {
        let mut status = status();
        AgentStatusTransitionPolicy::apply_idle(&mut status);
        assert!(matches!(status.activity, AgentActivity::Idle));
        assert_eq!(status.current_task, None);
    }

    #[test]
    fn activity_transition_preserves_current_task() {
        let mut status = status();
        AgentStatusTransitionPolicy::apply_activity(
            &mut status,
            AgentActivity::Working {
                context: "run".into(),
            },
        );
        assert_eq!(status.current_task, Some("task".into()));
        assert!(matches!(status.activity, AgentActivity::Working { .. }));
    }
}
```

- [ ] **Step 5: Run status tests**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-kernel status -- --nocapture
```

Expected:

```text
status and status_transition tests pass
```

## Task 6: Executor Payload Boundary

**Files:**

- Modify: `macaca/crates/macaca-kernel/src/executor/app_executor.rs`
- Optional Create: `macaca/crates/macaca-kernel/src/executor/event_publisher.rs`
- Modify: `macaca/crates/macaca-kernel/src/executor/mod.rs`

- [ ] **Step 1: Start with factory-only replacement**

Inside `ApplicationExecutor` worker command handling, replace direct construction:

```rust
let start_event = ExecutorEvent::TaskStarted {
    task_id,
    agent: agent_name.clone(),
};
```

with:

```rust
let events = super::ExecutorEventFactory::new(task_id, agent_name.clone());
let start_event = events.started();
```

- [ ] **Step 2: Replace completed event construction**

Replace:

```rust
let completed_event = ExecutorEvent::TaskCompleted {
    task_id,
    agent: agent_name.clone(),
    result: task_result,
};
```

with:

```rust
let completed_event = events.completed_with_result(task_result);
```

- [ ] **Step 3: Replace failed result and event construction**

Replace manual `TaskResult` construction:

```rust
let error_result = TaskResult {
    task_id,
    success: false,
    output: String::new(),
    error: Some(e.clone()),
    artifacts: vec![],
    completed_at: chrono::Utc::now(),
    tokens_used: None,
};
```

with:

```rust
let error_result = events.failed_result(e.clone());
```

Replace failed event:

```rust
let failed_event = events.failed(e);
```

- [ ] **Step 4: Keep send/broadcast sequence unchanged**

The send sequence must remain:

```rust
let _ = event_tx.send(event.clone()).await;
let _ = event_broadcast.send(event);
```

Do not introduce a publisher wrapper until factory replacement passes tests.

- [ ] **Step 5: Run executor tests and check web**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-kernel executor -- --nocapture
cargo check -p macaca-web
```

Expected:

```text
tests pass and web compiles
```

## Task 7: KernelBuilder / Facade Additive Entry

**Files:**

- Create: `macaca/crates/macaca-kernel/src/kernel_builder.rs`
- Modify: `macaca/crates/macaca-kernel/src/kernel.rs`
- Modify: `macaca/crates/macaca-kernel/src/lib.rs`

- [ ] **Step 1: Add builder**

Create `macaca/crates/macaca-kernel/src/kernel_builder.rs`:

```rust
use std::sync::Arc;

use macaca_llm::LlmProvider;
use macaca_proto::config::KernelConfig;
use macaca_tools::ToolCatalog;

use crate::{Kernel, SchedulerFactory, SchedulerKind};

pub struct KernelBuilder {
    config: KernelConfig,
    llm: Arc<dyn LlmProvider>,
    tools: Box<dyn ToolCatalog>,
    scheduler_kind: SchedulerKind,
}

impl KernelBuilder {
    pub fn new(
        config: KernelConfig,
        llm: Arc<dyn LlmProvider>,
        tools: Box<dyn ToolCatalog>,
    ) -> Self {
        Self {
            config,
            llm,
            tools,
            scheduler_kind: SchedulerKind::default(),
        }
    }

    pub fn scheduler_kind(mut self, scheduler_kind: SchedulerKind) -> Self {
        self.scheduler_kind = scheduler_kind;
        self
    }

    pub fn build(self) -> Kernel {
        Kernel::from_parts(
            self.config,
            self.llm,
            self.tools,
            SchedulerFactory::build(self.scheduler_kind),
        )
    }
}
```

- [ ] **Step 2: Add internal constructor**

In `macaca/crates/macaca-kernel/src/kernel.rs`, add:

```rust
pub(crate) fn from_parts(
    config: KernelConfig,
    llm: Arc<dyn LlmProvider>,
    tools: Box<dyn ToolCatalog>,
    scheduler: Box<dyn Scheduler>,
) -> Self {
    Self {
        registry: AgentRegistry::new(config.max_agents),
        scheduler,
        status_tracker: AgentStatusTracker::new(),
        llm,
        tools: Arc::from(tools),
    }
}
```

Then make `Kernel::new` delegate:

```rust
pub fn new(
    config: &KernelConfig,
    llm: Arc<dyn LlmProvider>,
    tools: Box<dyn ToolCatalog>,
) -> Self {
    crate::kernel_builder::KernelBuilder::new(config.clone(), llm, tools).build()
}
```

- [ ] **Step 3: Export builder**

Modify `macaca/crates/macaca-kernel/src/lib.rs`:

```rust
pub mod kernel_builder;
pub use kernel_builder::KernelBuilder;
```

- [ ] **Step 4: Add builder parity test**

Add a unit test in `kernel_builder.rs` using existing kernel test patterns:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use macaca_proto::{LlmMessage, LlmOptions, LlmResponse, MacacaResult, TokenUsage};
    use macaca_tools::DefaultToolSet;

    struct MockLlm;

    #[async_trait]
    impl LlmProvider for MockLlm {
        fn name(&self) -> &str {
            "mock"
        }

        async fn chat(
            &self,
            _messages: Vec<LlmMessage>,
            _options: &LlmOptions,
        ) -> MacacaResult<LlmResponse> {
            Ok(LlmResponse {
                content: "ok".into(),
                reasoning_content: None,
                model: "mock".into(),
                usage: TokenUsage {
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    total_tokens: 0,
                },
                finish_reason: "stop".into(),
                tool_calls: None,
            })
        }
    }

    #[tokio::test]
    async fn builder_matches_kernel_new_empty_registry() {
        let config = KernelConfig {
            max_agents: 16,
            heartbeat_interval_ms: 5000,
            agent_timeout_ms: 30000,
        };
        let llm: Arc<dyn LlmProvider> = Arc::new(MockLlm);
        let kernel = KernelBuilder::new(config, llm, Box::new(DefaultToolSet::new())).build();
        assert_eq!(kernel.agent_count().await, 0);
    }
}
```

- [ ] **Step 5: Do not migrate all consumers in this change**

Keep these existing calls unchanged in this refactor unless the OpenSpec explicitly adds consumer migration:

```text
macaca-web/src/lib.rs
macaca-app/src/runtime.rs
macaca-app/src/workflow.rs
macaca-sdk/src/registry_api.rs
macaca-cli/src/commands.rs
macaca-integration-tests/*
```

Consumer migration should be a separate proposal after the builder is stable.

## Task 8: Verification

**Files:**

- All changed files.

- [ ] **Step 1: Format**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo fmt
```

Expected:

```text
command exits 0
```

- [ ] **Step 2: Kernel tests**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-kernel -- --nocapture
```

Expected:

```text
all macaca-kernel tests pass
```

- [ ] **Step 3: Kernel integration tests**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-integration-tests kernel -- --nocapture
```

Expected:

```text
kernel-related integration tests pass
```

- [ ] **Step 4: Targeted workspace check**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo check -p macaca-kernel -p macaca-web -p macaca-app -p macaca-sdk -p macaca-cli
```

Expected:

```text
command exits 0
existing unrelated warnings may remain
```

- [ ] **Step 5: OpenSpec validation**

Run:

```bash
cd /Users/quantum/Code/dev/agent
openspec validate refactor-macaca-kernel-patterns --strict
```

Expected:

```text
Change 'refactor-macaca-kernel-patterns' is valid
```

- [ ] **Step 6: File-size audit**

Run:

```bash
cd /Users/quantum/Code/dev/agent
wc -l macaca/crates/macaca-kernel/src/*.rs macaca/crates/macaca-kernel/src/**/*.rs 2>/dev/null | sort -nr | head -20
```

Expected:

```text
new files are under 500 lines
existing app_executor.rs and fork_manager.rs may still exceed 500 lines and must be handled by follow-up split proposals
```

- [ ] **Step 7: GitNexus detect changes**

Run:

```text
gitnexus_detect_changes(scope: "all")
```

Expected:

- Changed symbols match the five planned slices.
- Any HIGH/CRITICAL process impact must be reported before commit.

## Self-Review

- Spec coverage: The plan covers event lifecycle factory, scheduler factory, status transition policy, executor payload boundary, kernel builder/facade, and verification.
- Placeholder scan: No placeholder markers remain.
- Type consistency: The plan uses existing kernel types: `ExecutorEvent`, `TaskResult`, `TokenUsage`, `Scheduler`, `SimpleScheduler`, `AgentStatusTracker`, `Kernel`, `KernelConfig`, `LlmProvider`, and `ToolCatalog`.
- Scope control: The plan does not split `ApplicationExecutor` or `ForkManager`, does not migrate all `Kernel::new` consumers, and does not alter EventLog/SSE/session behavior.
