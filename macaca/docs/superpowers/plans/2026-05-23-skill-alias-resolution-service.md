# Skill Alias Resolution Service Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add traced, service-owned Skill alias resolution so future consolidation can preserve old skill references without rewriting scheduler, task, or context files.

**Architecture:** Extend the existing Skill service command surface with alias upsert, resolve, and snapshot commands. Keep public provider construction stable while moving mutable governance/alias state into a focused runtime-host module.

**Tech Stack:** Rust, `macaca-skill`, `macaca-runtime-host`, `macaca-sdk`, OpenSpec.

---

### Task 1: Spec And Plan

**Files:**
- Create: `/Users/quantum/Code/dev/agent/openspec/changes/add-skill-alias-resolution-service/proposal.md`
- Create: `/Users/quantum/Code/dev/agent/openspec/changes/add-skill-alias-resolution-service/design.md`
- Create: `/Users/quantum/Code/dev/agent/openspec/changes/add-skill-alias-resolution-service/tasks.md`
- Create: `/Users/quantum/Code/dev/agent/openspec/changes/add-skill-alias-resolution-service/specs/skill-governance-curation/spec.md`

- [ ] **Step 1: Validate OpenSpec**

Run:

```bash
openspec validate add-skill-alias-resolution-service --strict
```

Expected: validation succeeds.

### Task 2: Skill Contract DTOs

**Files:**
- Modify: `/Users/quantum/Code/dev/agent/macaca/crates/services/macaca-skill/src/governance.rs`
- Modify: `/Users/quantum/Code/dev/agent/macaca/crates/services/macaca-skill/src/service_contract.rs`
- Modify: `/Users/quantum/Code/dev/agent/macaca/crates/services/macaca-skill/src/service_adapter.rs`

- [ ] **Step 1: Add alias records and command DTOs**

Add alias kinds, records, upsert/resolve/snapshot command DTOs, and sanitized results.

- [ ] **Step 2: Add command constants and descriptor capability**

Add `skill.alias.upsert`, `skill.alias.resolve`, `skill.alias.snapshot`, `capability.skill.alias`, and `skill.alias` permission.

### Task 3: Runtime Host Provider

**Files:**
- Create: `/Users/quantum/Code/dev/agent/macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider_state.rs`
- Modify: `/Users/quantum/Code/dev/agent/macaca/crates/runtime/macaca-runtime-host/src/lib.rs`
- Modify: `/Users/quantum/Code/dev/agent/macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider.rs`

- [ ] **Step 1: Move governance state to helper module**

Create a `SkillProviderGovernanceState` that owns records, aliases, and sorted snapshots.

- [ ] **Step 2: Add alias command branches**

Implement upsert, resolve, and snapshot branches with structured logs and no filesystem mutation.

### Task 4: SDK Facade

**Files:**
- Modify: `/Users/quantum/Code/dev/agent/macaca/crates/facade/macaca-sdk/src/skill_client.rs`

- [ ] **Step 1: Add alias methods to trait**

Expose alias upsert, resolve, and snapshot on `SystemSkillClient`.

- [ ] **Step 2: Add Null Object and service-backed behavior**

Unavailable client must return explicit unavailable or unresolved states; service-backed client routes through generic service calls.

### Task 5: Verification

**Files:**
- Modify tests beside changed provider modules.

- [ ] **Step 1: Run focused tests**

Run:

```bash
cd macaca
cargo test -p macaca-runtime-host skill_alias
cargo test -p macaca-runtime-host skill_governance
cargo test -p macaca-skill skill_descriptor_exports_contract_shape
cargo check -p macaca-sdk
```

- [ ] **Step 2: Run final gates**

Run:

```bash
openspec validate add-skill-alias-resolution-service --strict
git diff --check -- \
  openspec/changes/add-skill-alias-resolution-service \
  docs/superpowers/plans/2026-05-23-skill-alias-resolution-service.md \
  macaca/crates/services/macaca-skill \
  macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider.rs \
  macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider_state.rs \
  macaca/crates/facade/macaca-sdk/src/skill_client.rs
```
