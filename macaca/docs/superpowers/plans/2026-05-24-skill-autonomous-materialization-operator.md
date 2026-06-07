# Skill Autonomous Materialization Operator Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a governed Skill service operator that can autonomously drive ready proposal materialization without moving semantics into shells or applications.

**Architecture:** The operator is a Director over existing service-owned Commands and Strategies: proposal processing, package target resolution, proposal materialization, content mutation, and governance promotion. It is trace-required, policy-gated, dry-run capable, batch-limited, body-free, and fully auditable.

**Tech Stack:** Rust, `macaca-skill` DTO contracts, `macaca-runtime-host` Skill provider modules, OpenSpec, focused `cargo test` suites.

---

### Task 1: Contract And OpenSpec

**Files:**
- Create: `openspec/changes/add-skill-autonomous-materialization-operator/`
- Modify: `macaca/crates/services/macaca-skill/src/service_contract.rs`
- Modify: `macaca/crates/services/macaca-skill/src/lib.rs`
- Create: `macaca/crates/services/macaca-skill/src/proposal_materialization_operator.rs`

- [ ] Add OpenSpec proposal, design, tasks, and `skill-governance-curation` delta.
- [x] Add provider-neutral command/result DTOs for autonomous materialization runs.
- [x] Add command validation for trace, dry-run/apply policy refs, package readiness, entitlement readiness, batch limit, and evidence refs.
- [x] Export DTOs and command constants through `macaca-skill`.
- [x] Add DTO tests for apply denial without policy refs and result serialization without generated bodies.

### Task 2: Runtime Host Operator

**Files:**
- Create: `macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider_materialization_operator.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/lib.rs`
- Test: `macaca/crates/runtime/macaca-runtime-host/src/skill_materialization_operator_tests.rs`

- [x] Write a failing test proving dry-run operator runs processing and previews materialization without file mutation.
- [x] Write a failing test proving apply-mode operator materializes a ready proposal and promotes governance only after mutation succeeds.
- [x] Implement a small operator module that composes `process_proposals` and `materialize_ready_proposal`.
- [x] Add structured logs for accepted run, processing summary, selected proposals, materialization result, denial, rollback ref, and final aggregate result.
- [x] Route the new command from the Skill service provider.

### Task 3: Operations Evidence

**Files:**
- Modify: `macaca/crates/shells/macaca-web/src/skill_operations_routes.rs`
- Modify: `macaca/crates/facade/macaca-sdk/src/skill_client.rs`
- Modify: `macaca/crates/facade/macaca-sdk/src/skill_client_service_backed.rs`

- [ ] Expose operator snapshot/result refs through the SDK without adding shell-owned semantics.
- [ ] Extend operations output with body-free operator counts and last-run refs.
- [ ] Add tests proving Web operations remains a thin SDK adapter.

### Task 4: Verification

**Files:**
- Modify: `docs/macaca-agent-self-evolution-live-monitoring-report.md`

- [ ] Run `openspec validate add-skill-autonomous-materialization-operator --strict`.
- [ ] Run focused `macaca-skill`, `macaca-runtime-host`, and `macaca-web` tests.
- [ ] Run `cargo check -p macaca-web`.
- [ ] Run `git diff --check`.
- [ ] Run GitNexus change detection and record affected scope as advisory.
- [ ] Append live verification results and remaining activation/reuse blockers to the monitoring report.
