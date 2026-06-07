# Skill Experience Evolution Service Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a draft-only Skill experience evolution command that turns verified task evidence into sanitized skill proposal records without mutating active skill files.

**Architecture:** Extend `service.skill` with typed evolution DTOs, provider-local proposal state, and SDK facade methods. Keep the first slice deterministic and non-destructive so later policy, approval, memento, and Store-backed providers can replace the in-memory strategy.

**Tech Stack:** Rust, `macaca-skill`, `macaca-runtime-host`, `macaca-sdk`, OpenSpec.

---

### Task 1: OpenSpec Contract

**Files:**
- Create: `openspec/changes/add-skill-experience-evolution-service/proposal.md`
- Create: `openspec/changes/add-skill-experience-evolution-service/design.md`
- Create: `openspec/changes/add-skill-experience-evolution-service/tasks.md`
- Create: `openspec/changes/add-skill-experience-evolution-service/specs/skill-governance-curation/spec.md`

- [ ] **Step 1: Validate OpenSpec**

Run:

```bash
openspec validate add-skill-experience-evolution-service --strict
```

Expected: validation succeeds.

### Task 2: Contract DTOs

**Files:**
- Modify: `macaca/crates/services/macaca-skill/src/governance.rs`
- Modify: `macaca/crates/services/macaca-skill/src/service_contract.rs`
- Modify: `macaca/crates/services/macaca-skill/src/service_adapter.rs`

- [ ] **Step 1: Add evolution proposal DTOs**

Add task evidence input, candidate classification, recommended action, proposal record, command, and result types.

- [ ] **Step 2: Add command and descriptor capability**

Add `skill.evolution.propose_from_task`, `capability.skill.evolution`, and `skill.evolution` permission without changing existing command names.

### Task 3: Provider And Tests

**Files:**
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider_state.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider_tests.rs`

- [ ] **Step 1: Write failing tests**

Add tests for accepted verified reusable task evidence, rejected missing evidence, and non-mutating snapshots.

- [ ] **Step 2: Implement provider state and command branch**

Store proposal records in memory, return sorted snapshots through existing governance surfaces, and log key execution points.

### Task 4: SDK Facade

**Files:**
- Modify: `macaca/crates/facade/macaca-sdk/src/skill_client.rs`

- [ ] **Step 1: Extend trait and clients**

Add `propose_skill_experience` with structured unavailable behavior and service-backed routing.

### Task 5: Verification

**Files:**
- No additional files.

- [ ] **Step 1: Run focused checks**

Run:

```bash
openspec validate add-skill-experience-evolution-service --strict
cd macaca && cargo test -p macaca-runtime-host skill_experience
cd macaca && cargo test -p macaca-runtime-host skill_governance
cd macaca && cargo test -p macaca-runtime-host skill_alias
cd macaca && cargo test -p macaca-skill skill_descriptor_exports_contract_shape
cd macaca && cargo check -p macaca-sdk
git diff --check -- openspec/changes/add-skill-experience-evolution-service docs/superpowers macaca/crates/services/macaca-skill macaca/crates/runtime/macaca-runtime-host macaca/crates/facade/macaca-sdk/src/skill_client.rs
```

Expected: all commands exit successfully.
