# Skill Evolution Proposal Snapshot Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a read-only Skill evolution proposal snapshot command so draft proposals can be audited without mutating active skill state.

**Architecture:** Extend `service.skill` with typed snapshot DTOs, provider-local proposal listing, and SDK facade methods. Keep the slice deterministic and non-destructive so later promotion, rejection, LLM review, and Store-backed providers can build on a stable read model.

**Tech Stack:** Rust, `macaca-skill`, `macaca-runtime-host`, `macaca-sdk`, OpenSpec.

---

### Task 1: OpenSpec Contract

**Files:**
- Create: `openspec/changes/add-skill-evolution-proposal-snapshot/proposal.md`
- Create: `openspec/changes/add-skill-evolution-proposal-snapshot/design.md`
- Create: `openspec/changes/add-skill-evolution-proposal-snapshot/tasks.md`
- Create: `openspec/changes/add-skill-evolution-proposal-snapshot/specs/skill-governance-curation/spec.md`

- [ ] **Step 1: Validate OpenSpec**

Run:

```bash
openspec validate add-skill-evolution-proposal-snapshot --strict
```

Expected: validation succeeds.

### Task 2: Contract DTOs

**Files:**
- Modify: `macaca/crates/services/macaca-skill/src/evolution.rs`
- Modify: `macaca/crates/services/macaca-skill/src/service_contract.rs`
- Modify: `macaca/crates/services/macaca-skill/src/service_adapter.rs`

- [ ] **Step 1: Add snapshot DTOs and command name**

Add `SkillExperienceProposalSnapshotCommand`, `SkillExperienceProposalSnapshotResult`, and `SKILL_EVOLUTION_SNAPSHOT_COMMAND`.

- [ ] **Step 2: Keep descriptor append-only**

Keep the existing `capability.skill.evolution` and `skill.evolution` permission, and update descriptor tests only for command coverage if needed.

### Task 3: Provider And Tests

**Files:**
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider_state.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider_tests.rs`

- [ ] **Step 1: Write failing provider test**

Add a test that creates two proposals, calls `skill.evolution.snapshot`, and asserts the returned proposals are sorted, non-mutating, and sanitized.

- [ ] **Step 2: Implement provider state and command branch**

Return sorted proposal records from the in-memory strategy and log trace id plus proposal count.

### Task 4: SDK Facade

**Files:**
- Modify: `macaca/crates/facade/macaca-sdk/src/skill_client.rs`

- [ ] **Step 1: Extend trait and clients**

Add `skill_experience_snapshot` with empty unavailable behavior and service-backed routing.

### Task 5: Verification

**Files:**
- No additional files.

- [ ] **Step 1: Run focused checks**

Run:

```bash
openspec validate add-skill-evolution-proposal-snapshot --strict
cd macaca && cargo test -p macaca-runtime-host skill_experience
cd macaca && cargo test -p macaca-skill skill_descriptor_exports_contract_shape
cd macaca && cargo check -p macaca-sdk
git diff --check -- openspec/changes/add-skill-evolution-proposal-snapshot docs/superpowers macaca/crates/services/macaca-skill macaca/crates/runtime/macaca-runtime-host macaca/crates/facade/macaca-sdk/src/skill_client.rs
```

Expected: all commands exit successfully.
