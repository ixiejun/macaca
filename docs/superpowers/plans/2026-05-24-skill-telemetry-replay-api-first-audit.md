# Skill Telemetry Replay API-First Audit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix durable governed Skill usage telemetry replay and provide canonical API-first self-evolution audit/trigger verification.

**Architecture:** Persist sanitized Skill governance events through a local append-only journal Strategy and replay them into the provider read model before package recovery. Add a thin Web diagnostic Adapter that aggregates canonical operations, registry/load-path, and session observer evidence without owning Skill semantics.

**Tech Stack:** Rust, Tokio, Axum, serde JSONL, OpenSpec, Macaca Skill service DTOs.

---

### Task 1: OpenSpec Contract

**Files:**
- Create: `openspec/changes/fix-skill-telemetry-replay-api-first-audit/proposal.md`
- Create: `openspec/changes/fix-skill-telemetry-replay-api-first-audit/design.md`
- Create: `openspec/changes/fix-skill-telemetry-replay-api-first-audit/tasks.md`
- Create: `openspec/changes/fix-skill-telemetry-replay-api-first-audit/specs/skill-governance-curation/spec.md`

- [ ] Add requirements for durable usage telemetry replay.
- [ ] Add requirements for API-first audit/trigger verification.
- [ ] Run `openspec validate fix-skill-telemetry-replay-api-first-audit --strict`.

### Task 2: Durable Governance Journal

**Files:**
- Create: `macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider_event_journal.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/lib.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider_state.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/skill_service_governance_store.rs`
- Test: `macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider_tests.rs`

- [ ] Write a failing restart replay test.
- [ ] Add provider configuration for a generic governance event journal path.
- [ ] Persist sanitized events as JSONL after in-memory append.
- [ ] Replay JSONL events into the local read model during provider start.
- [ ] Keep materialized package recovery after replay so package identity fills gaps without resetting counters.

### Task 3: API-First Audit Adapter

**Files:**
- Create: `macaca/crates/shells/macaca-web/src/skill_self_evolution_audit.rs`
- Modify: `macaca/crates/shells/macaca-web/src/lib.rs`
- Modify: `macaca/crates/shells/macaca-web/src/bootstrap.rs`
- Test: `macaca/crates/shells/macaca-web/src/skill_self_evolution_audit.rs`

- [ ] Write DTO/helper tests for passed and failed canonical evidence states.
- [ ] Add a route that returns operations, registry visibility, and observer evidence status.
- [ ] Ensure the route logs key counts and missing evidence reasons.

### Task 4: Verification And Evidence

**Files:**
- Modify: `docs/macaca-agent-self-evolution-live-monitoring-report.md`

- [ ] Run targeted cargo tests.
- [ ] Run `cargo check -p macaca-runtime-host -p macaca-web`.
- [ ] Run `openspec validate fix-skill-telemetry-replay-api-first-audit --strict`.
- [ ] Run `git diff --check`.
- [ ] Update the live monitoring report with verified results and remaining gaps.
