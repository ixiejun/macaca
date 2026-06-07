# Migrate macaca-kernel Consumers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move upper-crate consumers off deprecated `macaca-kernel` construction APIs and onto `KernelBuilder` / kernel-owned primitives without changing runtime behavior.

**Architecture:** This is a consumer migration on top of the completed `refactor-macaca-kernel-patterns` change. Production upper crates use `KernelBuilder`; deprecated `Kernel::new` and direct `SimpleScheduler` remain callable only for kernel-internal compatibility coverage and factory bridge code.

**Tech Stack:** Rust workspace crates under `macaca/crates`, OpenSpec, GitNexus, cargo test/check, ripgrep verification.

---

## Current Context

The previous kernel refactor added:

- `macaca_kernel::KernelBuilder`
- `macaca_kernel::SchedulerFactory`
- `macaca_kernel::SchedulerKind`
- `macaca_kernel::executor::ExecutorEventFactory`
- `macaca_kernel::AgentStatusTransitionPolicy`

Deprecated but retained:

- `Kernel::new`
- `SimpleScheduler`

Observed direct upper-consumer `Kernel::new` calls:

- `macaca/crates/macaca-web/src/lib.rs:82`
- `macaca/crates/macaca-app/src/runtime.rs:286`
- `macaca/crates/macaca-app/src/workflow.rs:442`
- `macaca/crates/macaca-sdk/src/registry_api.rs:71`
- `macaca/crates/macaca-cli/src/commands.rs:65`
- `macaca/crates/macaca-cli/src/commands.rs:98`
- `macaca/crates/macaca-cli/src/commands.rs:123`
- `macaca/crates/macaca-cli/src/commands.rs:159`
- `macaca/crates/macaca-integration-tests/tests/app_declarative.rs:58`
- `macaca/crates/macaca-integration-tests/tests/fullstack_autodev.rs:58`
- `macaca/crates/macaca-integration-tests/tests/live_fullstack_autodev.rs:42`
- `macaca/crates/macaca-integration-tests/tests/kernel_lifecycle.rs:59`
- `macaca/crates/macaca-kernel/tests/e2e_auto_programming.rs:119`

Allowed remaining deprecated locations after this plan:

- `macaca/crates/macaca-kernel/src/kernel.rs`: deprecated compatibility wrapper definition.
- `macaca/crates/macaca-kernel/src/scheduler_factory.rs`: factory bridge to `SimpleScheduler`.
- `macaca/crates/macaca-kernel/src/scheduler.rs`: `SimpleScheduler` definition and compatibility tests.
- `macaca/crates/macaca-kernel/src/kernel.rs` tests or `macaca-kernel/tests/*` if explicitly named as compatibility coverage and locally allow deprecated.

## Superpowers Brainstorm

### Option A: Production-only migration

Replace `Kernel::new` in production crates and leave tests unchanged.

Benefits:

- Smallest diff.
- Removes runtime production deprecation warnings.

Risks:

- Tests keep showing old construction pattern.
- Future contributors may copy legacy test helpers into production code.
- Does not fully express the consumer migration contract.

### Option B: Production + upper tests migration, kernel compatibility tests retained

Replace `Kernel::new` in production crates and regular upper tests. Keep deprecated compatibility coverage only inside `macaca-kernel`.

Benefits:

- Clean production surface.
- Keeps deprecated APIs tested for migration-period compatibility.
- Makes grep verification meaningful.
- Matches the project’s additive-first migration policy.

Risks:

- Touches more crates than Option A.
- Requires careful grep exclusions for kernel compatibility code.

Recommendation: use Option B.

### Option C: Remove all deprecated calls everywhere

Replace every `Kernel::new` / `SimpleScheduler` call, including kernel compatibility tests.

Benefits:

- Cleanest grep output.

Risks:

- Deprecated APIs stay defined but untested.
- Conflicts with the project requirement to keep deprecated APIs callable until migration is complete.

## File Map

### OpenSpec

- Create: `openspec/changes/migrate-kernel-consumers-to-builder/proposal.md`
- Create: `openspec/changes/migrate-kernel-consumers-to-builder/design.md`
- Create: `openspec/changes/migrate-kernel-consumers-to-builder/tasks.md`
- Create: `openspec/changes/migrate-kernel-consumers-to-builder/specs/macaca-kernel-consumer-migration/spec.md`

### Production Rust files

- Modify: `macaca/crates/macaca-web/src/lib.rs`
  - Replace `Kernel::new` with `KernelBuilder::new(...).build()`.
- Modify: `macaca/crates/macaca-cli/src/commands.rs`
  - Add one local helper that uses `KernelBuilder`.
  - Route `run_kernel`, `list_agents`, `show_status`, and `create_kernel` through that helper.
- Modify: `macaca/crates/macaca-app/src/runtime.rs`
  - Replace test/helper `Kernel::new` with `KernelBuilder`.
- Modify: `macaca/crates/macaca-app/src/workflow.rs`
  - Replace test/helper `Kernel::new` with `KernelBuilder`.
- Modify: `macaca/crates/macaca-sdk/src/registry_api.rs`
  - Replace test/helper `Kernel::new` with `KernelBuilder`.

### Integration and kernel tests

- Modify: `macaca/crates/macaca-integration-tests/tests/app_declarative.rs`
- Modify: `macaca/crates/macaca-integration-tests/tests/fullstack_autodev.rs`
- Modify: `macaca/crates/macaca-integration-tests/tests/live_fullstack_autodev.rs`
- Modify: `macaca/crates/macaca-integration-tests/tests/kernel_lifecycle.rs`
- Modify: `macaca/crates/macaca-kernel/tests/e2e_auto_programming.rs`
  - Prefer `KernelBuilder` for normal e2e helpers unless the test is explicitly compatibility coverage.
- Modify only if needed: `macaca/crates/macaca-kernel/src/kernel.rs`
  - Keep at least one local compatibility test for deprecated `Kernel::new`.

## Task 1: Create OpenSpec Change

**Files:**

- Create: `openspec/changes/migrate-kernel-consumers-to-builder/proposal.md`
- Create: `openspec/changes/migrate-kernel-consumers-to-builder/design.md`
- Create: `openspec/changes/migrate-kernel-consumers-to-builder/tasks.md`
- Create: `openspec/changes/migrate-kernel-consumers-to-builder/specs/macaca-kernel-consumer-migration/spec.md`

- [ ] **Step 1: Review OpenSpec context**

Run:

```bash
cd /Users/quantum/Code/dev/agent
openspec list
openspec list --specs
```

Expected:

```text
refactor-macaca-kernel-patterns is complete or active.
migrate-kernel-consumers-to-builder does not already exist.
```

- [ ] **Step 2: Write proposal**

Create `openspec/changes/migrate-kernel-consumers-to-builder/proposal.md`:

```markdown
# Change: Migrate kernel consumers to KernelBuilder

## Why

`macaca-kernel` now exposes design-pattern primitives such as `KernelBuilder`, `SchedulerFactory`, and `ExecutorEventFactory`. Upper crates should stop constructing kernels through deprecated compatibility APIs so future kernel refactors can rely on one canonical construction path.

## What Changes

- Replace upper-crate production `Kernel::new` calls with `KernelBuilder`.
- Replace regular upper-crate test helpers with `KernelBuilder`.
- Keep deprecated `Kernel::new` and `SimpleScheduler` callable inside `macaca-kernel` compatibility coverage.
- Add verification that upper production crates do not call deprecated kernel construction APIs.

## Impact

- Affected specs: `macaca-kernel-consumer-migration`
- Affected code: `macaca-web`, `macaca-cli`, `macaca-app`, `macaca-sdk`, `macaca-integration-tests`, selected `macaca-kernel` tests
- Non-impact: no runtime behavior change; no scheduler behavior change; no trace, EventLog, SSE, planner, worker, coordinator, driver, skill, or MCP behavior changes.
```

- [ ] **Step 3: Write design**

Create `openspec/changes/migrate-kernel-consumers-to-builder/design.md`:

```markdown
## Context

The previous `refactor-macaca-kernel-patterns` change introduced `KernelBuilder`, `SchedulerFactory`, `ExecutorEventFactory`, and status transition primitives. `Kernel::new` and `SimpleScheduler` remain callable but deprecated.

Upper crates still call `Kernel::new` directly. This keeps production code coupled to the deprecated compatibility entry and causes deprecation warnings during workspace checks.

## Goals

- Keep behavior 1:1 compatible.
- Make `KernelBuilder` the canonical upper-crate construction entry.
- Keep deprecated kernel APIs callable for migration-period compatibility.
- Keep compatibility tests inside `macaca-kernel`.
- Prevent new upper production usage of deprecated kernel construction APIs.

## Non-Goals

- Do not remove `Kernel::new`.
- Do not remove `SimpleScheduler`.
- Do not change `SchedulerFactory` behavior.
- Do not change `ExecutorEvent` or `TaskResult` payloads.
- Do not change web session, trace, EventLog, SSE, task board, planner, worker, coordinator, driver, skill, or MCP behavior.
- Do not introduce app-specific or workflow-specific code.

## Decisions

- Use `KernelBuilder::new(config.clone(), llm, tools).build()` when the caller owns only `&KernelConfig`.
- Use `KernelBuilder::new(config, llm, tools).build()` when the caller owns the config.
- Add a small helper in `macaca-cli` to avoid repeating builder construction across commands.
- Treat `macaca-kernel` internal factory bridge and explicitly named compatibility tests as the only valid deprecated-call locations.
```

- [ ] **Step 4: Write tasks**

Create `openspec/changes/migrate-kernel-consumers-to-builder/tasks.md`:

```markdown
## 1. Preparation

- [ ] 1.1 Run GitNexus impact for `Kernel::new` upstream.
- [ ] 1.2 Run GitNexus impact for `run_kernel` upstream before editing CLI startup.
- [ ] 1.3 Run GitNexus impact for `init_state` or the web kernel startup function before editing `macaca-web/src/lib.rs`.
- [ ] 1.4 Confirm current deprecated kernel consumer calls with grep.

## 2. OpenSpec validation

- [ ] 2.1 Run `openspec validate migrate-kernel-consumers-to-builder --strict`.

## 3. Production consumer migration

- [ ] 3.1 Migrate `macaca-web/src/lib.rs` to `KernelBuilder`.
- [ ] 3.2 Migrate `macaca-cli/src/commands.rs` to a local builder-backed helper.
- [ ] 3.3 Migrate `macaca-app` helper construction to `KernelBuilder`.
- [ ] 3.4 Migrate `macaca-sdk` helper construction to `KernelBuilder`.

## 4. Test consumer migration and compatibility coverage

- [ ] 4.1 Migrate normal integration test kernel helpers to `KernelBuilder`.
- [ ] 4.2 Migrate normal kernel e2e helpers to `KernelBuilder`.
- [ ] 4.3 Ensure `macaca-kernel` retains explicit deprecated compatibility coverage for `Kernel::new`.
- [ ] 4.4 Keep `SimpleScheduler` compatibility usage confined to kernel scheduler tests and scheduler factory bridge.

## 5. Deprecated-call containment checks

- [ ] 5.1 Verify production upper crates have no `Kernel::new` calls.
- [ ] 5.2 Verify production upper crates have no direct `SimpleScheduler` calls.
- [ ] 5.3 Verify remaining deprecated calls are only in allowed kernel compatibility locations.

## 6. Verification

- [ ] 6.1 Run `cargo fmt`.
- [ ] 6.2 Run `cargo test -p macaca-kernel -- --nocapture`.
- [ ] 6.3 Run `cargo test -p macaca-integration-tests kernel -- --nocapture`.
- [ ] 6.4 Run `cargo check -p macaca-kernel -p macaca-web -p macaca-app -p macaca-sdk -p macaca-cli`.
- [ ] 6.5 Run `gitnexus_detect_changes(scope: "all")`.
```

- [ ] **Step 5: Write delta spec**

Create `openspec/changes/migrate-kernel-consumers-to-builder/specs/macaca-kernel-consumer-migration/spec.md`:

```markdown
## ADDED Requirements

### Requirement: Upper crates use KernelBuilder for kernel construction

Upper production crates SHALL construct kernels through `KernelBuilder` instead of deprecated `Kernel::new`.

#### Scenario: Web startup constructs kernel

- **WHEN** the web application initializes kernel state
- **THEN** it constructs the kernel through `KernelBuilder`
- **AND** LLM provider, tools, app registry, session, trace, and task behavior remain unchanged.

#### Scenario: CLI commands construct kernel

- **WHEN** CLI commands need a kernel
- **THEN** they construct the kernel through a builder-backed helper
- **AND** command output and startup behavior remain unchanged.

### Requirement: Deprecated kernel APIs remain compatibility-only

Deprecated kernel construction APIs SHALL remain callable but SHALL NOT be used by upper production crates.

#### Scenario: Compatibility coverage remains inside kernel

- **WHEN** compatibility tests exercise `Kernel::new` or direct `SimpleScheduler`
- **THEN** those calls are confined to `macaca-kernel` compatibility coverage
- **AND** upper production crates do not call those deprecated APIs.

### Requirement: Scheduler behavior remains unchanged

Migrating upper consumers to `KernelBuilder` SHALL NOT change default scheduler behavior.

#### Scenario: Builder default scheduler

- **WHEN** an upper crate constructs a kernel with `KernelBuilder::new(...).build()`
- **THEN** the default scheduler remains equivalent to the previous `Kernel::new` default.
```

## Task 2: Migrate Production Consumer Calls

**Files:**

- Modify: `macaca/crates/macaca-web/src/lib.rs`
- Modify: `macaca/crates/macaca-cli/src/commands.rs`
- Modify: `macaca/crates/macaca-app/src/runtime.rs`
- Modify: `macaca/crates/macaca-app/src/workflow.rs`
- Modify: `macaca/crates/macaca-sdk/src/registry_api.rs`

- [ ] **Step 1: Run GitNexus impact before editing symbols**

Run:

```text
gitnexus_impact({ target: "Kernel", direction: "upstream", repo: "agent" })
gitnexus_impact({ target: "run_kernel", direction: "upstream", repo: "agent" })
```

Expected:

- Risk may be high because web/CLI startup touches user-visible paths.
- If HIGH or CRITICAL, report the blast radius before editing and keep code changes limited to constructor replacement.

- [ ] **Step 2: Migrate web startup**

Change imports in `macaca/crates/macaca-web/src/lib.rs` from:

```rust
use macaca_kernel::Kernel;
```

to:

```rust
use macaca_kernel::{Kernel, KernelBuilder};
```

Replace:

```rust
let kernel = Arc::new(Kernel::new(
    &kernel_config,
    Arc::clone(&llm),
    Box::new(DefaultToolSet::new()),
));
```

with:

```rust
let kernel = Arc::new(
    KernelBuilder::new(
        kernel_config,
        Arc::clone(&llm),
        Box::new(DefaultToolSet::new()),
    )
    .build(),
);
```

- [ ] **Step 3: Migrate CLI helper construction**

Change imports in `macaca/crates/macaca-cli/src/commands.rs` from:

```rust
use macaca_kernel::Kernel;
```

to:

```rust
use macaca_kernel::{Kernel, KernelBuilder};
```

Add this helper near `create_kernel`:

```rust
fn build_kernel(
    config: KernelConfig,
    llm: Arc<dyn LlmProvider>,
    tools: Box<dyn macaca_tools::ToolCatalog>,
) -> Kernel {
    KernelBuilder::new(config, llm, tools).build()
}
```

Replace each command-local construction:

```rust
let kernel = Kernel::new(&config.kernel, llm, tools);
```

with:

```rust
let kernel = build_kernel(config.kernel.clone(), llm, tools);
```

Replace `create_kernel` body with:

```rust
pub fn create_kernel(config: &KernelConfig) -> Kernel {
    let llm: Arc<dyn LlmProvider> = Arc::new(StubLlmProvider);
    let tools = Box::new(DefaultToolSet::new());
    build_kernel(config.clone(), llm, tools)
}
```

- [ ] **Step 4: Migrate app test helpers**

In `macaca/crates/macaca-app/src/runtime.rs` and `macaca/crates/macaca-app/src/workflow.rs`, import `KernelBuilder` and replace:

```rust
Kernel::new(&config, llm, Box::new(DefaultToolSet::new()))
```

with:

```rust
KernelBuilder::new(config, llm, Box::new(DefaultToolSet::new())).build()
```

For inline config expressions currently passed by reference, bind them to a local `config` value first.

- [ ] **Step 5: Migrate SDK test helper**

In `macaca/crates/macaca-sdk/src/registry_api.rs`, import `KernelBuilder` and replace:

```rust
Kernel::new(&config, llm, Box::new(DefaultToolSet::new()))
```

with:

```rust
KernelBuilder::new(config, llm, Box::new(DefaultToolSet::new())).build()
```

## Task 3: Migrate Integration Tests and Keep Compatibility Coverage

**Files:**

- Modify: `macaca/crates/macaca-integration-tests/tests/app_declarative.rs`
- Modify: `macaca/crates/macaca-integration-tests/tests/fullstack_autodev.rs`
- Modify: `macaca/crates/macaca-integration-tests/tests/live_fullstack_autodev.rs`
- Modify: `macaca/crates/macaca-integration-tests/tests/kernel_lifecycle.rs`
- Modify: `macaca/crates/macaca-kernel/tests/e2e_auto_programming.rs`
- Modify if needed: `macaca/crates/macaca-kernel/src/kernel.rs`

- [ ] **Step 1: Migrate integration helper imports**

For each integration test file, change:

```rust
use macaca_kernel::Kernel;
```

to:

```rust
use macaca_kernel::{Kernel, KernelBuilder};
```

- [ ] **Step 2: Replace test helper construction**

Replace:

```rust
Kernel::new(&config, llm, Box::new(DefaultToolSet::new()))
```

with:

```rust
KernelBuilder::new(config, llm, Box::new(DefaultToolSet::new())).build()
```

- [ ] **Step 3: Keep one explicit compatibility test for `Kernel::new`**

If `macaca/crates/macaca-kernel/src/kernel.rs` does not already contain an explicit deprecated constructor compatibility test, add:

```rust
#[test]
#[allow(deprecated)]
fn deprecated_kernel_new_remains_callable() {
    let config = KernelConfig {
        max_agents: 4,
        heartbeat_interval_ms: 1000,
        agent_timeout_ms: 1000,
    };
    let llm: Arc<dyn LlmProvider> = Arc::new(MockLlm);
    let kernel = Kernel::new(&config, llm, Box::new(DefaultToolSet::new()));
    let _ = kernel;
}
```

If an equivalent compatibility test already exists, rename it to make the compatibility intent explicit instead of adding a duplicate.

## Task 4: Verification and Deprecated Containment

**Files:**

- No new source files expected.

- [ ] **Step 1: Run formatting**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo fmt
```

Expected:

```text
No formatting errors.
```

- [ ] **Step 2: Run kernel tests**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-kernel -- --nocapture
```

Expected:

```text
test result: ok
```

- [ ] **Step 3: Run integration kernel tests**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-integration-tests kernel -- --nocapture
```

Expected:

```text
No failures.
```

- [ ] **Step 4: Run upper crate checks**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo check -p macaca-kernel -p macaca-web -p macaca-app -p macaca-sdk -p macaca-cli
```

Expected:

```text
Finished `dev` profile
```

- [ ] **Step 5: Run deprecated production grep**

Run:

```bash
cd /Users/quantum/Code/dev/agent
rg -n "Kernel::new|SimpleScheduler\\b" \
  macaca/crates/macaca-web \
  macaca/crates/macaca-app \
  macaca/crates/macaca-sdk \
  macaca/crates/macaca-cli \
  --glob '*.rs'
```

Expected:

```text
No matches.
```

- [ ] **Step 6: Run remaining deprecated call audit**

Run:

```bash
cd /Users/quantum/Code/dev/agent
rg -n "Kernel::new|SimpleScheduler\\b" macaca/crates --glob '*.rs'
```

Expected remaining matches are limited to:

```text
macaca/crates/macaca-kernel/src/kernel.rs
macaca/crates/macaca-kernel/src/scheduler.rs
macaca/crates/macaca-kernel/src/scheduler_factory.rs
macaca/crates/macaca-kernel/tests/e2e_auto_programming.rs
```

If integration tests still appear, migrate them unless they are explicitly compatibility tests.

- [ ] **Step 7: Validate OpenSpec**

Run:

```bash
cd /Users/quantum/Code/dev/agent
openspec validate migrate-kernel-consumers-to-builder --strict
```

Expected:

```text
Change 'migrate-kernel-consumers-to-builder' is valid
```

- [ ] **Step 8: Run GitNexus detect changes**

Run:

```text
gitnexus_detect_changes({ scope: "all", repo: "agent" })
```

Expected:

- Changed symbols are limited to the OpenSpec files, documented plan files, and kernel consumer construction helpers.
- If risk is HIGH or CRITICAL, report affected flows before committing.

## Self-Review

- Spec coverage: The plan creates OpenSpec proposal/design/tasks/spec, migrates every observed upper production `Kernel::new` call, migrates regular upper tests, and preserves kernel compatibility coverage.
- Placeholder scan: No unresolved placeholders or unspecified implementation steps remain.
- Type consistency: The plan consistently uses `KernelBuilder::new(config, llm, tools).build()` and preserves `Kernel` return types.
- Scope check: This plan does not alter scheduler semantics, trace/EventLog/SSE behavior, task loop behavior, driver/skill/MCP integration, or app-specific logic.
