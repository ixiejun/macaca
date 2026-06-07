# macaca-agent Primitive Boundary Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 对 `macaca-agent` 做第二轮渐进式设计模式重构，把已落地的 services / capability / lifecycle 抽象收敛成稳定 primitive boundary，并保持现有行为 1:1 不变。

**Architecture:** 本轮只在 `macaca-agent` crate 内做 additive-first 边界整理：新增模块化 primitives、builder/conversion/facade 方法和行为锁定测试，旧 public re-export 与旧构造路径保持兼容。后续 `macaca-framework`、`macaca-sdk`、`macaca-web`、`macaca-kernel` 的消费方迁移单独提案处理。

**Tech Stack:** Rust, Tokio, async-trait, macaca-proto, macaca-llm, macaca-tools, OpenSpec, GitNexus, cargo test/check.

---

## Context

`macaca-agent` 已完成第一轮 `refactor-macaca-agent-patterns`：

- `AgentServices` 已有 no-op fallback facade：`memory_service()` / `ipc_service()` / `persist_service()`。
- `BasicAgentBuilder` 已存在，`BasicAgent::new` / `with_id` 已委托 builder。
- `AgentLifecyclePolicy` / `DefaultAgentLifecyclePolicy` 已存在，`AgentStateMachine` 已委托 policy。
- `AgentCapabilitySet` / `AgentCapabilityNode` / `CapabilitySource` 已存在，但仍放在 `basic.rs`。

当前继续重构的核心问题不是“再发明新行为”，而是把这些 primitive 从 demo-level 实现推进到可被上层长期稳定消费的 crate boundary：

- `agent.rs` 同时定义 Agent trait、service traits、noop services、AgentServices bundle，职责偏多。
- `basic.rs` 同时定义 BasicAgent、BasicAgentBuilder、capability composite，capability primitive 不应绑定 BasicAgent 文件。
- `state_machine.rs` 已有 policy，但 transition reason 推导仍是私有函数，上层无法做只读预检或审计。
- `AgentServices` 仍主要靠 public Option 字段构造，缺少 builder-style canonical construction。
- `macaca-app` 中已有平行的 `AppCapabilitySet`，`macaca-framework` 也直接消费 `AgentCapabilitySet`，后续需要一个更稳的 `macaca-agent` capability API 作为迁移基座。

## Superpowers Brainstorm

### Option A: 不再改 `macaca-agent`，直接迁移上层消费方

只基于当前 `AgentServices` / `AgentCapabilitySet` / `AgentLifecyclePolicy` 继续推进 `macaca-framework`、`macaca-web`、`macaca-sdk` 迁移。

Trade-offs:

- 优点：短期改动少，不碰底层 crate。
- 缺点：当前 primitive 仍散在 `agent.rs` / `basic.rs` / `state_machine.rs`，上层迁移会继续依赖不够清晰的内部命名和不完整 facade。
- 风险：消费方迁移过程中容易再次在 framework/web/app 里复制 capability / services / lifecycle 辅助结构。
- 结论：不推荐作为下一步。它会把底层边界债务推给更高风险的上层 crate。

### Option B: 第二轮 additive-first primitive boundary 收口

在 `macaca-agent` 内新增/整理 `services.rs`、`capability.rs`、`lifecycle.rs` 等边界，让现有类型迁移到更聚焦的模块；增加 `AgentServicesBuilder`、capability source 访问/转换、lifecycle transition preflight 等 additive API；旧 re-export 保持不变。

Trade-offs:

- 优点：小步、低风险、行为不变；能为后续 framework/web/sdk 迁移提供更清晰的基础 API。
- 缺点：会产生一些模块移动和 re-export，需要谨慎控制 public API 兼容。
- 风险：如果一次性把字段私有化会破坏调用方，所以本轮只新增 canonical API，不删除旧字段。
- 结论：推荐。本轮目标是“收口边界，不改变语义”。

### Option C: 一次性把 `macaca-agent` 升级为完整 AgentSpec / RuntimeAgent contract

在 `macaca-agent` 中新增 `AgentSpec`、`AgentExecutionContext`、`TracePolicy`、`ToolPolicy`，并让 framework/web/sdk 立即迁移。

Trade-offs:

- 优点：最终形态更完整，能快速减少 framework/web glue。
- 缺点：跨 `macaca-agent`、`macaca-framework`、`macaca-sdk`、`macaca-web`、`macaca-kernel`，很容易破坏 trace、tool visibility、session resume。
- 风险：和现有 `migrate-agent-construction-to-framework-primitives` OpenSpec 重叠，职责边界会混乱。
- 结论：本轮不采用。AgentSpec / traced construction 属于后续消费方迁移，不应塞进本次底层 crate 重构。

## Recommended Design

采用 Option B：第二轮 additive-first primitive boundary 收口。

本轮只做 `macaca-agent` 内部边界重构，不迁移上层 crate 行为：

- `AgentServices` 保留旧 public 字段，新增 `AgentServicesBuilder` 和 `with_*` 构造方法。
- no-op service 行为保持：不写 memory、不发 IPC、不写 persist、不产生 trace/event。
- `AgentCapabilitySet` 从 `basic.rs` 收到 `capability.rs`，新增只读 API：`is_empty()`、`len()`、`nodes()`、`sources()`、`from_source()`。
- `BasicAgent` 继续通过 flatten 后的 legacy `Vec<Capability>` 对外展示 capability。
- `AgentLifecyclePolicy` 保持现有行为，新增 `AgentLifecycleTransition` 值对象和 `AgentStateMachine::can_transition_to()` 只读预检。
- `lib.rs` 继续 re-export 所有旧类型，调用方不需要改代码。
- 不引入新依赖，不硬编码 application/workflow/driver/agent name。

## Design Pattern Mapping

- `Facade`: `AgentServices` 继续作为服务访问门面，新增 builder/canonical constructor 降低 public Option 字段依赖。
- `Null Object`: `NoopMemoryService` / `NoopIpcService` / `NoopPersistService` 保持缺省安全行为。
- `Builder`: `AgentServicesBuilder` 收敛服务 bundle 构造，和现有 `BasicAgentBuilder` 对齐。
- `Composite`: `AgentCapabilitySet` / `AgentCapabilityNode` 从 BasicAgent 文件中独立出来，成为 agent-level capability graph primitive。
- `State + Strategy`: `AgentLifecycleTransition` + `AgentLifecyclePolicy` 让状态迁移可预检、可审计、可替换。
- `Adapter`: 本轮不直接迁移 `macaca-kernel::services`，但为后续 service adapter 迁移提供稳定 builder 入口。

## Files

- Modify: `macaca/crates/macaca-agent/src/lib.rs`
- Modify: `macaca/crates/macaca-agent/src/agent.rs`
- Modify: `macaca/crates/macaca-agent/src/basic.rs`
- Modify: `macaca/crates/macaca-agent/src/state_machine.rs`
- Create: `macaca/crates/macaca-agent/src/services.rs`
- Create: `macaca/crates/macaca-agent/src/capability.rs`
- Create: `macaca/crates/macaca-agent/src/lifecycle.rs`
- Create: `openspec/changes/refactor-macaca-agent-primitive-boundaries/proposal.md`
- Create: `openspec/changes/refactor-macaca-agent-primitive-boundaries/design.md`
- Create: `openspec/changes/refactor-macaca-agent-primitive-boundaries/tasks.md`
- Create: `openspec/changes/refactor-macaca-agent-primitive-boundaries/specs/macaca-agent-core/spec.md`

## Task 1: OpenSpec proposal and delta spec

**Files:**
- Create: `openspec/changes/refactor-macaca-agent-primitive-boundaries/proposal.md`
- Create: `openspec/changes/refactor-macaca-agent-primitive-boundaries/design.md`
- Create: `openspec/changes/refactor-macaca-agent-primitive-boundaries/tasks.md`
- Create: `openspec/changes/refactor-macaca-agent-primitive-boundaries/specs/macaca-agent-core/spec.md`

- [ ] **Step 1: Review active changes**

Run:

```bash
cd /Users/quantum/Code/dev/agent
openspec list
openspec list --specs
```

Expected:

```text
Changes:
  refactor-macaca-agent-patterns  ✓ Complete
  migrate-agent-construction-to-framework-primitives  0/28 tasks
```

The exact list may include other active changes. Confirm there is no existing `refactor-macaca-agent-primitive-boundaries` change.

- [ ] **Step 2: Create proposal**

Create `openspec/changes/refactor-macaca-agent-primitive-boundaries/proposal.md`:

```markdown
# Change: Refactor macaca-agent primitive boundaries

## Why
`macaca-agent` has already introduced service facade/no-op fallbacks, BasicAgent builder, lifecycle policy, and capability composite primitives. These primitives are still co-located with concrete agent files and lack a small set of canonical construction and inspection APIs that upper crates can safely depend on.

## What Changes
- Add module boundaries for services, capability, and lifecycle primitives.
- Add `AgentServicesBuilder` as the canonical additive constructor for service bundles while preserving existing fields and behavior.
- Move capability composite types behind an agent-level capability module and add read-only inspection/conversion helpers.
- Add lifecycle transition value/preflight helpers without changing current transition semantics.
- Keep all existing public re-exports and runtime behavior compatible.

## Impact
- Affected specs: macaca-agent-core
- Affected code: `macaca/crates/macaca-agent/src/**`
- Follow-up consumers: `macaca-framework`, `macaca-sdk`, `macaca-web`, `macaca-kernel` in separate changes only.
```

- [ ] **Step 3: Create design**

Create `openspec/changes/refactor-macaca-agent-primitive-boundaries/design.md`:

```markdown
## Context

The first `macaca-agent` refactor is complete. The next risk is that upper crates consume these primitives while they are still shaped like implementation details. This change makes the primitives explicit module boundaries without changing behavior.

## Goals

- Preserve `Agent` trait behavior and `BasicAgent` behavior 1:1.
- Preserve no-op service side effects exactly: no memory writes, no IPC side effects, no persist writes.
- Preserve lifecycle transition matrix exactly.
- Preserve legacy flattened capability output exactly.
- Provide canonical additive APIs for future consumer migration.

## Non-Goals

- Do not migrate framework/web/sdk/kernel consumers in this change.
- Do not introduce AgentSpec or traced construction contracts.
- Do not make `AgentServices` fields private yet.
- Do not change trace, EventLog, SSE, task, planner, worker, or coordinator behavior.

## Decisions

- Module extraction is allowed only if `lib.rs` re-exports keep existing imports compiling.
- `AgentServicesBuilder` is additive; existing direct struct construction remains possible during migration.
- Capability source inspection is read-only; mutation still happens through explicit constructors and `push_group`.
- Lifecycle preflight returns boolean/result based on the same `transition_reason` semantics as `transition`.
```

- [ ] **Step 4: Create tasks checklist**

Create `openspec/changes/refactor-macaca-agent-primitive-boundaries/tasks.md`:

```markdown
## 1. Preparation
- [ ] 1.1 Run GitNexus impact for `AgentServices` upstream.
- [ ] 1.2 Run GitNexus impact for `AgentCapabilitySet` upstream.
- [ ] 1.3 Run GitNexus impact for `AgentStateMachine` upstream.
- [ ] 1.4 Confirm current `cargo test -p macaca-agent` baseline passes.

## 2. Services primitive boundary
- [ ] 2.1 Add `services.rs` with service traits, no-op implementations, `AgentServices`, and `AgentServicesBuilder`.
- [ ] 2.2 Keep public re-exports compatible from `agent.rs` and `lib.rs`.
- [ ] 2.3 Add tests for builder-provided memory/ipc/persist services and no-op defaults.

## 3. Capability primitive boundary
- [ ] 3.1 Add `capability.rs` and move `CapabilitySource`, `AgentCapabilityNode`, and `AgentCapabilitySet` there.
- [ ] 3.2 Add read-only helpers: `is_empty`, `len`, `nodes`, `sources`, `from_source`.
- [ ] 3.3 Update `basic.rs` to consume capability primitives from the new module.
- [ ] 3.4 Add tests proving flattened legacy capability output is unchanged.

## 4. Lifecycle primitive boundary
- [ ] 4.1 Add `lifecycle.rs` with `AgentTransitionReason`, `AgentLifecyclePolicy`, `DefaultAgentLifecyclePolicy`, and `AgentLifecycleTransition`.
- [ ] 4.2 Keep `AgentStateMachine` API compatible and delegate to lifecycle primitives.
- [ ] 4.3 Add `AgentStateMachine::can_transition_to` as additive preflight.
- [ ] 4.4 Add transition table tests for preflight and transition behavior equivalence.

## 5. Verification
- [ ] 5.1 Run `cargo fmt`.
- [ ] 5.2 Run `cargo test -p macaca-agent -- --nocapture`.
- [ ] 5.3 Run `cargo check -p macaca-agent -p macaca-framework -p macaca-sdk -p macaca-kernel -p macaca-web`.
- [ ] 5.4 Run deprecated/API containment grep for old imports and confirm compatibility.
- [ ] 5.5 Run `openspec validate refactor-macaca-agent-primitive-boundaries --strict`.
- [ ] 5.6 Run `gitnexus_detect_changes(scope: "all")` before commit.
```

- [ ] **Step 5: Create delta spec**

Create `openspec/changes/refactor-macaca-agent-primitive-boundaries/specs/macaca-agent-core/spec.md`:

```markdown
## ADDED Requirements

### Requirement: Agent service construction SHALL expose a canonical builder

The system SHALL provide an additive builder-style API for constructing `AgentServices` while preserving the existing direct field compatibility and no-op fallback behavior.

#### Scenario: Empty services preserve no-op behavior
- **GIVEN** an `AgentServices` value is constructed without concrete services
- **WHEN** callers access memory, IPC, or persistence through facade methods
- **THEN** no-op services SHALL be returned
- **AND** those no-op services SHALL NOT write memory, send IPC, write persistence, or alter agent output

#### Scenario: Builder services are effective
- **GIVEN** concrete memory, IPC, or persistence services are provided through the builder
- **WHEN** callers access them through facade methods
- **THEN** the effective services SHALL be the provided implementations

### Requirement: Agent capabilities SHALL have a stable primitive boundary

The system SHALL expose agent capability composite types through a dedicated primitive boundary while preserving existing flattened legacy output.

#### Scenario: Flattened capability output is unchanged
- **GIVEN** a `BasicAgent` is built with legacy capabilities
- **WHEN** capabilities are flattened for the legacy `Agent::capabilities` API
- **THEN** the visible capability list SHALL match the previous behavior

#### Scenario: Capability sources are inspectable
- **GIVEN** capabilities are grouped by source
- **WHEN** callers inspect the capability set
- **THEN** source metadata SHALL be available through read-only APIs
- **AND** callers SHALL NOT need to parse flattened capability names to infer source

### Requirement: Agent lifecycle transitions SHALL be preflightable

The system SHALL expose read-only lifecycle transition preflight using the same semantics as state mutation.

#### Scenario: Valid transition preflight matches mutation
- **GIVEN** a valid transition in the current lifecycle matrix
- **WHEN** the transition is checked through preflight
- **THEN** it SHALL be accepted
- **AND** executing the same transition SHALL still succeed

#### Scenario: Invalid transition preflight matches mutation
- **GIVEN** an invalid transition in the current lifecycle matrix
- **WHEN** the transition is checked through preflight
- **THEN** it SHALL be rejected
- **AND** executing the same transition SHALL still fail without changing state

## MODIFIED Requirements

### Requirement: macaca-agent refactor remains additive and behavior-compatible

The system SHALL keep `macaca-agent` primitive boundary refactors additive and SHALL NOT change existing agent execution, service fallback, lifecycle, capability, trace, session, task, planner, worker, coordinator, or application behavior.

#### Scenario: Existing imports remain compatible
- **GIVEN** upper crates import `AgentServices`, `AgentCapabilitySet`, `BasicAgentBuilder`, or `AgentLifecyclePolicy` from `macaca_agent`
- **WHEN** primitive modules are introduced
- **THEN** existing imports SHALL continue to compile through public re-exports
```

- [ ] **Step 6: Validate OpenSpec**

Run:

```bash
cd /Users/quantum/Code/dev/agent
openspec validate refactor-macaca-agent-primitive-boundaries --strict
```

Expected:

```text
Change 'refactor-macaca-agent-primitive-boundaries' is valid
```

## Task 2: Services primitive boundary

**Files:**
- Modify: `macaca/crates/macaca-agent/src/agent.rs`
- Create: `macaca/crates/macaca-agent/src/services.rs`
- Modify: `macaca/crates/macaca-agent/src/lib.rs`

- [ ] **Step 1: Run impact analysis before editing**

Run GitNexus:

```text
gitnexus_impact({ target: "AgentServices", direction: "upstream", repo: "agent" })
```

Expected:

```text
Risk level is reviewed before editing. Direct callers in macaca-agent/framework/sdk/kernel/web are known.
```

If risk is HIGH or CRITICAL, report it before editing.

- [ ] **Step 2: Write behavior tests first**

Add tests in `macaca/crates/macaca-agent/src/services.rs` after moving code. Required test names:

```rust
#[tokio::test]
async fn builder_with_memory_service_uses_provided_service() { /* use a test MemoryService that records calls */ }

#[tokio::test]
async fn builder_with_ipc_service_uses_provided_service() { /* use a test IpcService that records calls */ }

#[tokio::test]
async fn builder_with_persist_service_uses_provided_service() { /* use a test PersistService that records calls */ }
```

- [ ] **Step 3: Create services module**

Move service traits/no-op implementations/`AgentServices` from `agent.rs` into `services.rs`. Add this builder:

```rust
pub struct AgentServicesBuilder {
    memory: Option<Box<dyn MemoryService>>,
    ipc: Option<Box<dyn IpcService>>,
    persist: Option<Box<dyn PersistService>>,
}

impl AgentServicesBuilder {
    pub fn new() -> Self {
        Self {
            memory: None,
            ipc: None,
            persist: None,
        }
    }

    pub fn memory(mut self, memory: Box<dyn MemoryService>) -> Self {
        self.memory = Some(memory);
        self
    }

    pub fn ipc(mut self, ipc: Box<dyn IpcService>) -> Self {
        self.ipc = Some(ipc);
        self
    }

    pub fn persist(mut self, persist: Box<dyn PersistService>) -> Self {
        self.persist = Some(persist);
        self
    }

    pub fn build(self) -> AgentServices {
        AgentServices {
            memory: self.memory,
            ipc: self.ipc,
            persist: self.persist,
        }
    }
}

impl Default for AgentServicesBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentServices {
    pub fn builder() -> AgentServicesBuilder {
        AgentServicesBuilder::new()
    }
}
```

Do not make existing `AgentServices` fields private in this task.

- [ ] **Step 4: Keep agent.rs focused on Agent trait**

`agent.rs` should import `AgentServices` from `crate::services` and keep only the `Agent` trait plus trait-specific docs.

- [ ] **Step 5: Update lib exports**

In `lib.rs`, add:

```rust
pub mod services;

pub use services::{
    AgentServices, AgentServicesBuilder, IpcService, MemoryService, NoopIpcService,
    NoopMemoryService, NoopPersistService, PersistService,
};
```

Keep `pub mod agent;` and `pub use agent::Agent;` compatible.

- [ ] **Step 6: Verify services task**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-agent services -- --nocapture
cargo test -p macaca-agent agent -- --nocapture
```

Expected: all targeted tests pass.

## Task 3: Capability primitive boundary

**Files:**
- Create: `macaca/crates/macaca-agent/src/capability.rs`
- Modify: `macaca/crates/macaca-agent/src/basic.rs`
- Modify: `macaca/crates/macaca-agent/src/lib.rs`

- [ ] **Step 1: Run impact analysis before editing**

Run GitNexus:

```text
gitnexus_impact({ target: "AgentCapabilitySet", direction: "upstream", repo: "agent" })
```

Expected: risk reviewed before editing.

- [ ] **Step 2: Move capability types**

Move `CapabilitySource`, `AgentCapabilityNode`, and `AgentCapabilitySet` from `basic.rs` to `capability.rs` without changing existing behavior.

- [ ] **Step 3: Add read-only helpers**

Add these methods:

```rust
impl AgentCapabilitySet {
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.flatten_for_legacy_api().len()
    }

    pub fn nodes(&self) -> &[AgentCapabilityNode] {
        &self.nodes
    }

    pub fn from_source(source: CapabilitySource, capabilities: Vec<Capability>) -> Self {
        let mut set = Self::default();
        set.push_group(source, capabilities);
        set
    }

    pub fn sources(&self) -> Vec<CapabilitySource> {
        self.nodes
            .iter()
            .filter_map(|node| match node {
                AgentCapabilityNode::Leaf(_) => Some(CapabilitySource::Legacy),
                AgentCapabilityNode::Group { source, .. } => Some(source.clone()),
            })
            .collect()
    }
}
```

If cloning sources is undesirable, use `CapabilitySource: Copy` instead because variants are simple.

- [ ] **Step 4: Update BasicAgent imports**

In `basic.rs`, remove local capability definitions and import:

```rust
use crate::capability::{AgentCapabilitySet, CapabilitySource};
```

Keep `BasicAgent::capability_set()` unchanged.

- [ ] **Step 5: Update lib exports**

In `lib.rs`, add:

```rust
pub mod capability;

pub use capability::{AgentCapabilityNode, AgentCapabilitySet, CapabilitySource};
```

Remove capability exports from the `basic` re-export list.

- [ ] **Step 6: Add capability tests**

Add tests in `capability.rs`:

```rust
#[test]
fn capability_set_reports_sources_without_changing_flattened_output() { /* source metadata + flatten */ }

#[test]
fn capability_set_from_source_matches_push_group() { /* compare flattened output */ }
```

- [ ] **Step 7: Verify capability task**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-agent capability -- --nocapture
cargo test -p macaca-agent basic -- --nocapture
```

Expected: all targeted tests pass.

## Task 4: Lifecycle primitive boundary

**Files:**
- Create: `macaca/crates/macaca-agent/src/lifecycle.rs`
- Modify: `macaca/crates/macaca-agent/src/state_machine.rs`
- Modify: `macaca/crates/macaca-agent/src/lib.rs`

- [ ] **Step 1: Run impact analysis before editing**

Run GitNexus:

```text
gitnexus_impact({ target: "AgentStateMachine", direction: "upstream", repo: "agent" })
```

Expected: risk reviewed before editing.

- [ ] **Step 2: Move lifecycle policy types**

Move `AgentTransitionReason`, `AgentLifecyclePolicy`, and `DefaultAgentLifecyclePolicy` from `state_machine.rs` to `lifecycle.rs`.

- [ ] **Step 3: Add transition value object**

In `lifecycle.rs`, add:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentLifecycleTransition {
    pub from: AgentState,
    pub to: AgentState,
    pub reason: AgentTransitionReason,
}

impl AgentLifecycleTransition {
    pub fn new(from: AgentState, to: AgentState) -> MacacaResult<Self> {
        Ok(Self {
            from,
            to,
            reason: transition_reason(from, to)?,
        })
    }
}

pub fn transition_reason(from: AgentState, to: AgentState) -> MacacaResult<AgentTransitionReason> {
    match (from, to) {
        (AgentState::Created, AgentState::Running) => Ok(AgentTransitionReason::Start),
        (AgentState::Running, AgentState::Suspended) => Ok(AgentTransitionReason::Suspend),
        (AgentState::Suspended, AgentState::Running) => Ok(AgentTransitionReason::Resume),
        (AgentState::Running, AgentState::Terminated)
        | (AgentState::Suspended, AgentState::Terminated) => Ok(AgentTransitionReason::Terminate),
        _ => Err(MacacaError::Agent(format!(
            "invalid state transition: {:?} -> {:?}",
            from, to
        ))),
    }
}
```

- [ ] **Step 4: Add preflight API**

In `AgentStateMachine`, add:

```rust
pub fn can_transition_to(&self, next: AgentState) -> bool {
    AgentLifecycleTransition::new(self.state, next)
        .map(|transition| {
            self.policy
                .can_transition(transition.from, transition.to, transition.reason)
        })
        .unwrap_or(false)
}
```

Update `transition` to use `AgentLifecycleTransition::new` and preserve the same error string style.

- [ ] **Step 5: Update lib exports**

In `lib.rs`, add:

```rust
pub mod lifecycle;

pub use lifecycle::{
    AgentLifecyclePolicy, AgentLifecycleTransition, AgentTransitionReason,
    DefaultAgentLifecyclePolicy,
};
```

Keep `AgentStateMachine` exported from `state_machine`.

- [ ] **Step 6: Add lifecycle tests**

Add tests:

```rust
#[test]
fn can_transition_to_matches_transition_success_matrix() { /* use same table as existing test */ }

#[test]
fn lifecycle_transition_rejects_invalid_pair() { /* Created -> Suspended is error */ }
```

- [ ] **Step 7: Verify lifecycle task**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-agent lifecycle -- --nocapture
cargo test -p macaca-agent state_machine -- --nocapture
```

Expected: all targeted tests pass.

## Task 5: Full verification and commit

**Files:**
- Modify: all files touched in Tasks 1-4

- [ ] **Step 1: Format**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo fmt
```

Expected: completes without formatting errors.

- [ ] **Step 2: Run macaca-agent tests**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-agent -- --nocapture
```

Expected: all `macaca-agent` tests pass.

- [ ] **Step 3: Run dependent compile check**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo check -p macaca-agent -p macaca-framework -p macaca-sdk -p macaca-kernel -p macaca-web
```

Expected: compilation succeeds. Existing warnings are acceptable if unrelated.

- [ ] **Step 4: Validate OpenSpec**

Run:

```bash
cd /Users/quantum/Code/dev/agent
openspec validate refactor-macaca-agent-primitive-boundaries --strict
```

Expected:

```text
Change 'refactor-macaca-agent-primitive-boundaries' is valid
```

- [ ] **Step 5: Check public compatibility grep**

Run:

```bash
cd /Users/quantum/Code/dev/agent
rg -n "use macaca_agent::\{.*AgentServices|AgentCapabilitySet|AgentLifecyclePolicy|AgentStateMachine" macaca/crates --glob '*.rs'
```

Expected: existing imports still compile without requiring call-site rewrites.

- [ ] **Step 6: GitNexus detect changes**

Run GitNexus:

```text
gitnexus_detect_changes({ scope: "all", repo: "agent" })
```

Expected: affected scope is limited to `macaca-agent` and compile-only dependent crates unless OpenSpec/docs are included.

- [ ] **Step 7: Commit**

Run:

```bash
cd /Users/quantum/Code/dev/agent
git status --short
git add openspec/changes/refactor-macaca-agent-primitive-boundaries macaca/crates/macaca-agent
git commit -m "refactor agent primitive boundaries"
```

Expected: commit succeeds and working tree contains no unexpected unstaged changes from this refactor.

## Self-Review

- Spec coverage: Plan includes OpenSpec proposal/design/tasks/spec, services builder/facade, capability module, lifecycle module/preflight, verification, GitNexus, and commit.
- Placeholder scan: No unfinished placeholder markers remain; code snippets are concrete enough for implementation.
- Type consistency: All new names are consistent: `AgentServicesBuilder`, `AgentCapabilitySet`, `AgentLifecycleTransition`, `can_transition_to`.
- Scope check: This is intentionally `macaca-agent`-only. Consumer migration is explicitly deferred to separate OpenSpec changes to keep behavior stable.
