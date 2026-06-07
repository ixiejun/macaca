# Materialized Skill Semantic Identity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Materialized Skill packages should expose semantic, triggerable names and descriptions instead of proposal-id-derived names whenever sanitized semantic evidence exists.

**Architecture:** Keep identity derivation inside the existing runtime-host materialization Builder. The Builder creates bounded model-facing metadata, while mutation, policy, rollback, promotion, and audit remain in existing service-owned commands and strategies.

**Tech Stack:** Rust, `macaca-runtime-host`, `macaca-skill`, OpenSpec.

---

### Task 1: Semantic Identity Test

**Files:**
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/skill_proposal_materialization_tests.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider_proposal_materialization.rs`

- [ ] **Step 1: Write the failing test**

Add a test named `semantic_materialized_skill_identity_prefers_reusable_procedure_over_proposal_id`. It should create a ready proposal with no `target_skill_name`, a reusable procedure such as `Verify materialization registry load path and usage telemetry`, run dry-run materialization, and assert that the resulting `skill_id` contains semantic words such as `materialization`, `registry`, and `telemetry` while not containing the raw task UUID.

- [ ] **Step 2: Run test to verify it fails**

Run:
`cargo test -p macaca-runtime-host semantic_materialized_skill_identity --manifest-path macaca/Cargo.toml`

Expected: FAIL because `proposal_skill_name()` currently falls back to the proposal id.

- [ ] **Step 3: Implement semantic identity derivation**

Add a small `MaterializedSkillIdentity` helper owned by the Builder. It should:
- prefer `target_skill_name`;
- otherwise extract bounded meaningful ASCII tokens from `reusable_procedure`;
- otherwise extract from `bounded_summary`;
- otherwise fall back to a bounded proposal-id slug;
- cap names at the current slug length.

- [ ] **Step 4: Improve generated trigger text**

Use the derived identity to render a `When To Use` section with bounded context from the proposal rather than the generic one-line template.

- [ ] **Step 5: Add key-node logs**

Log the derived name, derivation source, proposal id, trace id, and whether a fallback was used. Do not log generated bodies or raw proposal text.

- [ ] **Step 6: Verify**

Run:
`cargo test -p macaca-runtime-host semantic_materialized_skill_identity --manifest-path macaca/Cargo.toml`

Expected: PASS.

### Task 2: Contract And Build Verification

**Files:**
- Modify: `openspec/changes/improve-materialized-skill-semantic-identity/*`
- Modify: `docs/superpowers/specs/2026-05-24-materialized-skill-semantic-identity-design.md`
- Modify: `docs/superpowers/plans/2026-05-24-materialized-skill-semantic-identity.md`

- [ ] **Step 1: Validate OpenSpec**

Run:
`openspec validate improve-materialized-skill-semantic-identity --strict`

Expected: PASS.

- [ ] **Step 2: Run broader materialization tests**

Run:
`cargo test -p macaca-runtime-host proposal_materialization --manifest-path macaca/Cargo.toml`

Expected: PASS.

- [ ] **Step 3: Run crate check**

Run:
`cargo check -p macaca-runtime-host --manifest-path macaca/Cargo.toml`

Expected: PASS.

- [ ] **Step 4: Run whitespace check**

Run:
`git diff --check`

Expected: no output and exit code 0.
