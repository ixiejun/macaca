# Skill Proposal Materialization Lane Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a governed Skill service command that turns a `ReadyForMaterialization` proposal into a bounded `SKILL.md` draft through existing mutation and rollback policy.

**Architecture:** `macaca-skill` owns provider-neutral Command DTOs and validation. `macaca-runtime-host` owns the built-in materialization Strategy and a Builder that produces AgentSkills-compatible content, then delegates writes to the existing content-mutation Strategy. Shells remain adapters and no application-specific branches are introduced.

**Tech Stack:** Rust workspace under `macaca/`, `macaca-skill`, `macaca-runtime-host`, OpenSpec, GitNexus.

---

## File Structure

- Create `openspec/changes/add-skill-proposal-materialization-lane/`.
- Create `macaca/crates/services/macaca-skill/src/proposal_materialization.rs` for DTOs, validation, and bounded result metadata.
- Modify `macaca/crates/services/macaca-skill/src/lib.rs` and `service_contract.rs` to export command types and constants.
- Create `macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider_proposal_materialization.rs` for the local Strategy and Builder orchestration.
- Modify `macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider.rs` and `lib.rs` to route the new command.
- Create `macaca/crates/runtime/macaca-runtime-host/src/skill_proposal_materialization_tests.rs` for TDD coverage.

## Task 1: OpenSpec

- [ ] Add proposal, design, tasks, and delta spec for `add-skill-proposal-materialization-lane`.
- [ ] Validate with `openspec validate add-skill-proposal-materialization-lane --strict`.

## Task 2: Contract DTOs

- [ ] Run GitNexus impact for edited exported symbols.
- [ ] Write failing tests for materialization command validation and result serialization.
- [ ] Add `SkillProposalMaterializationCommand`, status, and result DTOs with detailed English comments.
- [ ] Export DTOs and command constant through `macaca-skill`.
- [ ] Run `cargo test -p macaca-skill proposal_materialization`.

## Task 3: Runtime Strategy

- [ ] Write failing runtime tests for non-ready denial, dry-run immutability, and apply-mode `SKILL.md` creation.
- [ ] Add a Builder that converts a proposal into bounded AgentSkills-compatible markdown.
- [ ] Add a Strategy that checks processing state, delegates to `SkillContentMutationCommand`, promotes the draft only after successful mutation, and logs sanitized checkpoints.
- [ ] Route `skill.evolution.materialization.apply` through `SkillSystemServiceProvider`.
- [ ] Run `cargo test -p macaca-runtime-host skill_proposal_materialization`.

## Task 4: Verification And Docs

- [ ] Run `cargo test -p macaca-runtime-host skill_proposal_processing`.
- [ ] Run `cargo test -p macaca-runtime-host skill_content_mutation`.
- [ ] Run `cargo check -p macaca-web` if service command exports affect Web build.
- [ ] Run `openspec validate add-skill-proposal-materialization-lane --strict`.
- [ ] Run `git diff --check`.
- [ ] Run GitNexus `detect_changes`.
- [ ] Update `docs/macaca-agent-self-evolution-live-monitoring-report.md` with the new materialization status and any remaining blocker.
