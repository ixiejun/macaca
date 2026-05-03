# Migrate macaca-agent Consumers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move upper-crate consumers off deprecated `macaca-agent` construction helpers and onto the new service/capability/lifecycle primitives without changing runtime behavior.

**Architecture:** Treat the current `macaca-agent` primitive-boundary refactor as the baseline. This migration is a narrow consumer cleanup: upper crates must call the additive primitive APIs directly, while larger traced construction consolidation remains owned by the existing `migrate-agent-construction-to-framework-primitives` change.

**Tech Stack:** Rust workspace crates under `macaca/crates`, OpenSpec, GitNexus, cargo test/check.

---

## Current Context

The local tree already contains the completed-but-uncommitted `macaca-agent` primitive-boundary refactor:

- `macaca/crates/macaca-agent/src/services.rs` adds `AgentServices::builder()` and marks `AgentServices::empty()` deprecated.
- `macaca/crates/macaca-agent/src/capability.rs` adds `AgentCapabilitySet`, `AgentCapabilityNode`, and `CapabilitySource`.
- `macaca/crates/macaca-agent/src/lifecycle.rs` adds lifecycle policy/transition primitives.
- `macaca/crates/macaca-agent/src/basic.rs` and `macaca/crates/macaca-agent/src/state_machine.rs` keep compatibility but mark direct legacy construction paths deprecated.

Upper-crate scan currently shows only three direct deprecated `AgentServices::empty()` consumers:

- `macaca/crates/macaca-kernel/src/kernel.rs:71`
- `macaca/crates/macaca-sdk/src/builder.rs:292`
- `macaca/crates/macaca-sdk/src/builder.rs:307`

Upper crates already partially consume the new primitives:

- `macaca/crates/macaca-framework/src/construction.rs` uses `AgentServices`, `AgentCapabilitySet`, and `AgentLifecyclePolicy` inside `AgentBuildRequest`.
- `macaca/crates/macaca-web/src/framework_runner.rs` builds traced requests with `AgentServices::default()` and `AgentCapabilitySet`.

There is also a larger active change:

- `openspec/changes/migrate-agent-construction-to-framework-primitives/`

This plan must not duplicate that broader migration. It only removes deprecated upper-crate direct calls and locks the consumer contract around the additive primitive entry points.

## Superpowers Brainstorm

### Option A: Minimal deprecated-call cleanup only

Replace the three `AgentServices::empty()` calls with `AgentServices::builder().build()` and run checks.

Benefits:

- Lowest risk.
- Removes the immediate deprecation warnings in upper crates.
- Keeps behavior exactly the same because both paths construct no-op service bundles.

Risks:

- Does not add an OpenSpec contract proving upper crates must avoid deprecated primitive APIs.
- Future contributors may reintroduce deprecated calls.

### Option B: Minimal cleanup plus OpenSpec consumer contract

Create a focused OpenSpec change for `macaca-agent` consumer migration, replace deprecated upper calls, and add grep/check tasks that prevent old APIs in upper crates.

Benefits:

- Still small and reversible.
- Aligns with project rule that behavior/interface migrations need OpenSpec first.
- Produces an enforceable contract: upper crates must use additive primitive entries.
- Avoids overlapping with the broader framework construction migration.

Risks:

- Adds a small amount of spec/process overhead for a small code change.
- Requires careful wording so it does not conflict with `migrate-agent-construction-to-framework-primitives`.

### Option C: Fold this into the broader framework construction migration

Extend `migrate-agent-construction-to-framework-primitives` to include all remaining upper-crate consumer cleanup.

Benefits:

- Keeps all “agent construction migration” work under one umbrella.
- Can coordinate future framework/web/task changes in one proposal.

Risks:

- Too broad for the current requested small migration.
- Makes a simple deprecated-call cleanup dependent on an already large 28-task change.
- Increases regression surface across trace/session/task construction paths.

### Recommendation

Use Option B. It satisfies the AGENTS.md workflow, keeps the migration small, and gives the project a clear consumer rule without broadening into traced construction internals.

## Design Pattern Fit

From `macaca/docs/design_patterns.md`, this slice should use the existing patterns already introduced by `macaca-agent`:

- **Builder:** `AgentServices::builder().build()` is the canonical construction entry and replaces deprecated ad-hoc empty construction.
- **Composite:** `AgentCapabilitySet` remains the capability container exposed to framework/web consumers; no new parallel capability shape is introduced in this slice.
- **Strategy:** `AgentLifecyclePolicy` remains the lifecycle extension point; this slice does not add new lifecycle policy behavior.
- **Adapter / Facade:** `AgentServices` is the only upper-crate service binding facade. Upper crates must not assemble or depend on the internal optional fields directly.

Do not introduce new patterns in this migration. The existing primitives are sufficient.

## Scope

In scope:

- Add a focused OpenSpec change for upper-crate migration to `macaca-agent` primitive APIs.
- Replace upper-crate `AgentServices::empty()` calls with `AgentServices::builder().build()`.
- Verify no upper crate calls deprecated `AgentServices::empty()`, `BasicAgent::new()`, `BasicAgent::with_id()`, `AgentStateMachine::new()`, or `AgentStateMachine::with_policy()`.
- Keep `macaca-agent` legacy functions available and deprecated for compatibility and grepability.

Out of scope:

- Do not migrate `macaca-app::AppCapabilitySet` into `macaca-agent::AgentCapabilitySet` in this slice. That affects manifest semantics and should be a separate app/model migration.
- Do not continue the full framework/web/task traced construction migration here. That belongs to `openspec/changes/migrate-agent-construction-to-framework-primitives/`.
- Do not make `AgentServices` fields private.
- Do not change trace, EventLog, SSE, planner, worker, coordinator, driver, skill, MCP, or task behavior.

## Files

- Create: `openspec/changes/migrate-macaca-agent-consumers-to-primitive-boundaries/proposal.md`
- Create: `openspec/changes/migrate-macaca-agent-consumers-to-primitive-boundaries/design.md`
- Create: `openspec/changes/migrate-macaca-agent-consumers-to-primitive-boundaries/tasks.md`
- Create: `openspec/changes/migrate-macaca-agent-consumers-to-primitive-boundaries/specs/macaca-agent-consumer-migration/spec.md`
- Modify: `macaca/crates/macaca-kernel/src/kernel.rs`
- Modify: `macaca/crates/macaca-sdk/src/builder.rs`

## Task 1: Create OpenSpec Change

**Files:**

- Create: `openspec/changes/migrate-macaca-agent-consumers-to-primitive-boundaries/proposal.md`
- Create: `openspec/changes/migrate-macaca-agent-consumers-to-primitive-boundaries/design.md`
- Create: `openspec/changes/migrate-macaca-agent-consumers-to-primitive-boundaries/tasks.md`
- Create: `openspec/changes/migrate-macaca-agent-consumers-to-primitive-boundaries/specs/macaca-agent-consumer-migration/spec.md`

- [ ] **Step 1: Review current OpenSpec context**

Run:

```bash
cd /Users/quantum/Code/dev/agent
openspec list
openspec list --specs
```

Expected:

- Active changes include the existing `refactor-macaca-agent-primitive-boundaries`.
- The new change id `migrate-macaca-agent-consumers-to-primitive-boundaries` does not exist.

- [ ] **Step 2: Create proposal**

Create `openspec/changes/migrate-macaca-agent-consumers-to-primitive-boundaries/proposal.md`:

```markdown
# Change: Migrate macaca-agent consumers to primitive boundaries

## Why

`macaca-agent` now exposes additive service, capability, and lifecycle primitives. Upper crates should stop calling deprecated construction helpers so future refactors can rely on one canonical primitive surface.

## What Changes

- Replace upper-crate `AgentServices::empty()` calls with `AgentServices::builder().build()`.
- Require upper crates to consume `macaca-agent` primitives through additive public entries instead of deprecated compatibility helpers.
- Add verification that deprecated direct construction helpers are not used outside `macaca-agent`.

## Impact

- Affected specs: `macaca-agent-consumer-migration`
- Affected code: `macaca-kernel`, `macaca-sdk`
- Non-impact: no runtime behavior change; no trace, task, planner, worker, coordinator, driver, skill, or MCP behavior changes.
```

- [ ] **Step 3: Create design**

Create `openspec/changes/migrate-macaca-agent-consumers-to-primitive-boundaries/design.md`:

```markdown
## Context

The previous `macaca-agent` primitive-boundary refactor introduced `AgentServices::builder()`, `AgentCapabilitySet`, and lifecycle policy primitives while keeping legacy helpers deprecated for compatibility.

The only upper-crate deprecated service construction calls currently found are in `macaca-kernel` and `macaca-sdk` tests. Framework/web already consume `AgentServices` and `AgentCapabilitySet`, while larger traced construction cleanup remains covered by `migrate-agent-construction-to-framework-primitives`.

## Goals

- Keep behavior 1:1 compatible.
- Remove upper-crate usage of deprecated `macaca-agent` helper constructors.
- Preserve deprecated helper definitions inside `macaca-agent` for temporary compatibility and grepability.
- Keep this migration independent from the broader framework construction migration.

## Non-Goals

- Do not remove deprecated helper definitions from `macaca-agent`.
- Do not migrate app manifest capability modeling in this change.
- Do not change traced agent construction, session behavior, EventLog behavior, task scheduling, or planner/worker/coordinator behavior.

## Decisions

- Use `AgentServices::builder().build()` instead of `AgentServices::default()` at migrated call sites because it explicitly exercises the new builder pattern.
- Treat `AgentServices::default()` as acceptable inside framework request defaults because it delegates to the builder and is not deprecated.
- Add a verification grep that excludes definitions inside `macaca-agent` and checks upper crates only.
```

- [ ] **Step 4: Create tasks**

Create `openspec/changes/migrate-macaca-agent-consumers-to-primitive-boundaries/tasks.md`:

```markdown
## 1. Preparation

- [ ] 1.1 Run GitNexus impact for `execute_agent` upstream.
- [ ] 1.2 Run GitNexus impact for `DeclarativeAgent` upstream.
- [ ] 1.3 Confirm current deprecated upper-crate call sites with grep.

## 2. Consumer migration

- [ ] 2.1 Replace `AgentServices::empty()` in `macaca-kernel` with `AgentServices::builder().build()`.
- [ ] 2.2 Replace `AgentServices::empty()` in `macaca-sdk` tests with `AgentServices::builder().build()`.

## 3. Verification

- [ ] 3.1 Run `cargo fmt`.
- [ ] 3.2 Run `cargo test -p macaca-sdk declarative_agent -- --nocapture`.
- [ ] 3.3 Run `cargo test -p macaca-kernel -- --nocapture`.
- [ ] 3.4 Run `cargo check -p macaca-agent -p macaca-framework -p macaca-sdk -p macaca-kernel -p macaca-web`.
- [ ] 3.5 Run deprecated-call grep for upper crates.
- [ ] 3.6 Run `openspec validate migrate-macaca-agent-consumers-to-primitive-boundaries --strict`.
- [ ] 3.7 Run `gitnexus_detect_changes(scope: "all")`.
```

- [ ] **Step 5: Create delta spec**

Create `openspec/changes/migrate-macaca-agent-consumers-to-primitive-boundaries/specs/macaca-agent-consumer-migration/spec.md`:

```markdown
## ADDED Requirements

### Requirement: Upper crates use additive macaca-agent primitives

Upper crates SHALL construct agent service bundles through additive `macaca-agent` primitive APIs rather than deprecated compatibility helpers.

#### Scenario: Kernel builds empty services

- **GIVEN** the kernel executes a registered agent
- **WHEN** it needs an empty service bundle
- **THEN** it constructs services with `AgentServices::builder().build()`
- **AND** the no-op memory, IPC, and persistence fallback behavior remains unchanged.

#### Scenario: SDK tests build empty services

- **GIVEN** SDK declarative-agent tests need a service bundle
- **WHEN** they run an agent with no injected services
- **THEN** they construct services with `AgentServices::builder().build()`
- **AND** existing test assertions remain unchanged.
```

- [ ] **Step 6: Validate OpenSpec**

Run:

```bash
cd /Users/quantum/Code/dev/agent
openspec validate migrate-macaca-agent-consumers-to-primitive-boundaries --strict
```

Expected:

```text
Change 'migrate-macaca-agent-consumers-to-primitive-boundaries' is valid
```

## Task 2: Run Impact Analysis Before Editing

**Files:**

- No file changes in this task.

- [ ] **Step 1: Check impact for kernel execution path**

Run GitNexus:

```text
gitnexus_impact({
  "repo": "agent",
  "target": "execute_agent",
  "direction": "upstream",
  "maxDepth": 3
})
```

Expected:

- Direct callers are listed.
- Risk is not ignored. If risk is HIGH or CRITICAL, stop and report the blast radius before editing.

- [ ] **Step 2: Check impact for SDK declarative agent path**

Run GitNexus:

```text
gitnexus_impact({
  "repo": "agent",
  "target": "DeclarativeAgent",
  "direction": "upstream",
  "maxDepth": 3
})
```

Expected:

- SDK test and builder consumers are listed.
- Risk is not ignored. If risk is HIGH or CRITICAL, stop and report before editing.

- [ ] **Step 3: Confirm deprecated upper-crate call sites**

Run:

```bash
cd /Users/quantum/Code/dev/agent
rg -n "AgentServices::empty\\(|BasicAgent::new\\(|BasicAgent::with_id\\(|AgentStateMachine::new\\(|AgentStateMachine::with_policy\\(" macaca/crates --glob '*.rs'
```

Expected before migration:

```text
macaca/crates/macaca-agent/src/services.rs:... pub fn empty() -> Self {
macaca/crates/macaca-kernel/src/kernel.rs:... let services = AgentServices::empty();
macaca/crates/macaca-sdk/src/builder.rs:... let services = AgentServices::empty();
macaca/crates/macaca-sdk/src/builder.rs:... let services = AgentServices::empty();
```

Additional deprecated definitions inside `macaca-agent` may appear. Those definitions stay in place.

## Task 3: Migrate macaca-kernel Consumer

**Files:**

- Modify: `macaca/crates/macaca-kernel/src/kernel.rs`

- [ ] **Step 1: Replace service construction in `Kernel::execute_agent`**

Change:

```rust
let services = AgentServices::empty();
```

To:

```rust
let services = AgentServices::builder().build();
```

Expected behavior:

- The kernel still injects an empty service bundle.
- No-op memory, IPC, and persistence fallbacks remain unchanged.

- [ ] **Step 2: Run targeted kernel test/check**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-kernel -- --nocapture
```

Expected:

- Tests pass.
- No new `AgentServices::empty()` deprecation warning from `macaca-kernel`.

## Task 4: Migrate macaca-sdk Test Consumers

**Files:**

- Modify: `macaca/crates/macaca-sdk/src/builder.rs`

- [ ] **Step 1: Replace service construction in `declarative_agent_run_calls_llm`**

Change:

```rust
let services = AgentServices::empty();
```

To:

```rust
let services = AgentServices::builder().build();
```

- [ ] **Step 2: Replace service construction in `declarative_agent_empty_prompt_errors`**

Change:

```rust
let services = AgentServices::empty();
```

To:

```rust
let services = AgentServices::builder().build();
```

- [ ] **Step 3: Run targeted SDK tests**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-sdk declarative_agent -- --nocapture
```

Expected:

- `declarative_agent_run_calls_llm` passes.
- `declarative_agent_empty_prompt_errors` passes.
- No new `AgentServices::empty()` deprecation warning from `macaca-sdk`.

## Task 5: Workspace Verification

**Files:**

- No source edits beyond Tasks 3 and 4.

- [ ] **Step 1: Format**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo fmt
```

Expected:

- Command exits successfully.

- [ ] **Step 2: Run cross-crate check**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo check -p macaca-agent -p macaca-framework -p macaca-sdk -p macaca-kernel -p macaca-web
```

Expected:

- Command exits successfully.
- Existing unrelated warnings may remain.
- There are no deprecation warnings caused by upper-crate `AgentServices::empty()` usage.

- [ ] **Step 3: Verify deprecated helper calls are contained**

Run:

```bash
cd /Users/quantum/Code/dev/agent
rg -n "AgentServices::empty\\(|BasicAgent::new\\(|BasicAgent::with_id\\(|AgentStateMachine::new\\(|AgentStateMachine::with_policy\\(" macaca/crates --glob '*.rs'
```

Expected:

- Deprecated helper definitions or tests inside `macaca-agent` may remain.
- No matches in `macaca-kernel`, `macaca-sdk`, `macaca-framework`, `macaca-web`, `macaca-task`, or other upper crates.

- [ ] **Step 4: Validate OpenSpec**

Run:

```bash
cd /Users/quantum/Code/dev/agent
openspec validate migrate-macaca-agent-consumers-to-primitive-boundaries --strict
```

Expected:

```text
Change 'migrate-macaca-agent-consumers-to-primitive-boundaries' is valid
```

- [ ] **Step 5: Run GitNexus detect changes**

Run GitNexus:

```text
gitnexus_detect_changes({
  "repo": "agent",
  "scope": "all"
})
```

Expected:

- Changed symbols match the planned migration.
- Affected processes are limited to expected kernel/sdk service construction paths and OpenSpec/docs.
- If unexpected trace, task, planner, worker, coordinator, driver, skill, or MCP flows appear, stop and inspect before committing.

## Task 6: Commit

**Files:**

- Stage the OpenSpec change and the two source files modified by this plan.

- [ ] **Step 1: Review diff**

Run:

```bash
cd /Users/quantum/Code/dev/agent
git diff -- openspec/changes/migrate-macaca-agent-consumers-to-primitive-boundaries macaca/crates/macaca-kernel/src/kernel.rs macaca/crates/macaca-sdk/src/builder.rs
```

Expected:

- Diff only contains the new OpenSpec change and `AgentServices::empty()` call-site replacements.

- [ ] **Step 2: Commit**

Run:

```bash
cd /Users/quantum/Code/dev/agent
git add openspec/changes/migrate-macaca-agent-consumers-to-primitive-boundaries macaca/crates/macaca-kernel/src/kernel.rs macaca/crates/macaca-sdk/src/builder.rs
git commit -m "refactor: migrate macaca-agent consumer primitives"
```

Expected:

- Commit succeeds.
- If the repository hook rebuilds GitNexus, let it finish.

## Self-Review

- Spec coverage: The plan creates an OpenSpec change, migrates every currently observed upper-crate deprecated service construction call, and validates containment.
- Placeholder scan: The plan contains concrete commands, file paths, expected outputs, and exact replacement code.
- Scope control: This plan explicitly excludes the larger traced construction migration and app capability model unification.
- Risk control: GitNexus impact is required before edits, and `detect_changes` is required before commit.
