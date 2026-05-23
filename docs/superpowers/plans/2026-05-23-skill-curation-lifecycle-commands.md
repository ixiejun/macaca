# Skill Curation Lifecycle Commands Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add traced, service-owned lifecycle mutation commands for governed Skill curation.

**Architecture:** Extend the existing Skill service boundary with metadata-only commands for pin, unpin, archive, and restore. Runtime-host owns the built-in provider strategy, SDK owns the facade, and shells remain read-only consumers until a later approval UI change.

**Tech Stack:** Rust, Tokio, serde DTOs, `SystemService`, `SystemSkillClient`, OpenSpec.

---

### Task 1: OpenSpec Contract

**Files:**
- Create: `openspec/changes/add-skill-curation-lifecycle-commands/proposal.md`
- Create: `openspec/changes/add-skill-curation-lifecycle-commands/design.md`
- Create: `openspec/changes/add-skill-curation-lifecycle-commands/tasks.md`
- Create: `openspec/changes/add-skill-curation-lifecycle-commands/specs/skill-governance-curation/spec.md`

- [x] **Step 1: Define the additive behavior**

Create an OpenSpec change that states lifecycle mutation is service-owned,
metadata-only, trace-required, and protection-aware.

- [x] **Step 2: Validate the change**

Run: `openspec validate add-skill-curation-lifecycle-commands --strict`
Expected: validation succeeds.

### Task 2: Service DTOs

**Files:**
- Modify: `macaca/crates/services/macaca-skill/src/governance.rs`
- Modify: `macaca/crates/services/macaca-skill/src/service_contract.rs`

- [x] **Step 1: Add lifecycle command/result DTOs**

Add an action enum, command struct, and result struct with trace, scope, target,
reason, evidence ids, policy hints, mutation flag, and captured timestamp.

- [x] **Step 2: Add command constants**

Add `skill.curation.pin`, `skill.curation.unpin`, `skill.curation.archive`,
and `skill.curation.restore`.

### Task 3: Runtime Provider

**Files:**
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider_state.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider.rs`

- [x] **Step 1: Add state transition helper**

Implement metadata-only transitions that create a governance record when needed,
update lifecycle/pinned state, retain evidence ids, and deny archive of pinned
records.

- [x] **Step 2: Wire service commands**

Decode each new command, call the state helper, log trace/action/result, and
return typed results.

### Task 4: SDK Facade

**Files:**
- Modify: `macaca/crates/facade/macaca-sdk/src/skill_client.rs`

- [x] **Step 1: Extend `SystemSkillClient`**

Add one typed lifecycle facade method that maps the lifecycle action to the
provider command name.

- [x] **Step 2: Implement unavailable and service-backed behavior**

Unavailable client returns structured config errors; service-backed client sends
typed service calls with trace.

### Task 5: Tests And Verification

**Files:**
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider_tests.rs`

- [x] **Step 1: Add provider tests**

Cover pin/unpin, archive/restore, pinned archive denial, snapshot filtering, and
missing evidence validation.

- [x] **Step 2: Run targeted checks**

Run:
- `cargo test -p macaca-runtime-host skill_service_provider_tests -- --nocapture`
- `cargo check -p macaca-skill -p macaca-sdk -p macaca-runtime-host`
- `openspec validate add-skill-curation-lifecycle-commands --strict`
- `git diff --check`
- GitNexus `detect_changes`
