# Skill Governance Curation Service Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the first safe slice of Macaca skill self-evolution: governed usage telemetry, lifecycle metadata, and deterministic curation dry-run reports behind `service.skill`.

**Architecture:** Extend the existing Skill service using Command, Facade, State, Observer, Memento, and Specification patterns. Runtime-host provides a built-in in-memory provider for the first slice; future changes can replace storage and curation strategies without changing kernel or shell ownership.

**Tech Stack:** Rust, `macaca-skill`, `macaca-runtime-host`, `macaca-sdk`, OpenSpec.

---

### Task 1: OpenSpec Contract

**Files:**
- Create: `/Users/quantum/Code/dev/agent/openspec/changes/add-skill-governance-curation-service/proposal.md`
- Create: `/Users/quantum/Code/dev/agent/openspec/changes/add-skill-governance-curation-service/design.md`
- Create: `/Users/quantum/Code/dev/agent/openspec/changes/add-skill-governance-curation-service/tasks.md`
- Create: `/Users/quantum/Code/dev/agent/openspec/changes/add-skill-governance-curation-service/specs/skill-governance-curation/spec.md`

- [ ] **Step 1: Validate the OpenSpec change**

Run:

```bash
openspec validate add-skill-governance-curation-service --strict
```

Expected: validation succeeds.

### Task 2: Service DTOs

**Files:**
- Create: `/Users/quantum/Code/dev/agent/macaca/crates/services/macaca-skill/src/governance.rs`
- Modify: `/Users/quantum/Code/dev/agent/macaca/crates/services/macaca-skill/src/lib.rs`
- Modify: `/Users/quantum/Code/dev/agent/macaca/crates/services/macaca-skill/src/service_contract.rs`
- Modify: `/Users/quantum/Code/dev/agent/macaca/crates/services/macaca-skill/src/service_adapter.rs`

- [ ] **Step 1: Add governance data models and command constants**

Implement lifecycle state, provenance, usage events, curation recommendations, and typed command/result DTOs.

- [ ] **Step 2: Export the models and append descriptor capabilities**

Export through `lib.rs` and add `skill.governance.*` permissions without changing existing command names.

### Task 3: Runtime Provider

**Files:**
- Modify: `/Users/quantum/Code/dev/agent/macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider.rs`

- [ ] **Step 1: Add in-memory governance state**

Use a mutex-protected store keyed by skill id. Keep it provider-local and replaceable.

- [ ] **Step 2: Implement usage record and snapshot commands**

Decode typed commands, require trace, update sanitized counters, and log accepted/completed events.

- [ ] **Step 3: Implement curation dry-run**

Return deterministic recommendations only. Do not touch skill files.

### Task 4: SDK Client

**Files:**
- Modify: `/Users/quantum/Code/dev/agent/macaca/crates/facade/macaca-sdk/src/skill_client.rs`

- [ ] **Step 1: Extend the trait and Null Object**

Add governance snapshot, usage record, and curation dry-run methods with structured unavailable behavior.

- [ ] **Step 2: Extend service-backed calls**

Route the new methods through the existing generic service client.

### Task 5: Verification

**Files:**
- Modify: focused unit tests next to changed modules.

- [ ] **Step 1: Run targeted tests**

Run:

```bash
cd macaca
cargo test -p macaca-skill skill_descriptor_exports_contract_shape
cargo test -p macaca-runtime-host skill_governance
cargo check -p macaca-sdk
```

- [ ] **Step 2: Run formatting and diff checks**

Run:

```bash
git diff --check -- \
  openspec/changes/add-skill-governance-curation-service \
  docs/superpowers/plans/2026-05-23-skill-governance-curation-service.md \
  macaca/crates/services/macaca-skill \
  macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider.rs \
  macaca/crates/facade/macaca-sdk/src/skill_client.rs
```
