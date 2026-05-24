# Skill Proposal Processing Lane Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a service-owned processing lane that scores, groups, suppresses, and snapshots Skill evolution proposals before any future materialization gate.

**Architecture:** Skill service owns provider-neutral Command DTOs, processing State, deterministic Specification checks, and sanitized snapshots. Runtime-host provides the built-in Strategy over current proposal records; SDK/Web/CLI stay thin Facade/Adapter layers and never own curation semantics.

**Tech Stack:** Rust workspace under `macaca/`, `macaca-skill`, `macaca-runtime-host`, `macaca-sdk`, `macaca-web`, OpenSpec, GitNexus.

---

## File Structure

- Modify `macaca/crates/services/macaca-skill/src/proposal_processing.rs`: new DTOs, validation, quality score, duplicate signature helpers.
- Modify `macaca/crates/services/macaca-skill/src/lib.rs`: export proposal-processing DTOs.
- Modify `macaca/crates/services/macaca-skill/src/service_contract.rs`: add command constants and descriptor command entries.
- Modify `macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider_state.rs`: store processing records and snapshot helper.
- Create `macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider_proposal_processing.rs`: built-in deterministic Strategy and command handlers.
- Modify `macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider.rs`: route new commands.
- Modify `macaca/crates/runtime/macaca-runtime-host/src/lib.rs`: register the new provider module.
- Modify `macaca/crates/facade/macaca-sdk/src/skill_client.rs`: trait and unavailable client methods.
- Modify `macaca/crates/facade/macaca-sdk/src/skill_client_service_backed.rs`: service-backed command forwarding.
- Modify `macaca/crates/shells/macaca-web/src/skill_operations_routes.rs`: include processing snapshot in operations output and add a thin processing run route only if route registration is already straightforward.
- Test files: focused tests in existing `macaca-runtime-host` and `macaca-sdk` test modules; add `macaca-skill` unit tests beside DTOs.

### Task 1: OpenSpec Proposal

**Files:**
- Create: `openspec/changes/add-skill-proposal-processing-lane/proposal.md`
- Create: `openspec/changes/add-skill-proposal-processing-lane/design.md`
- Create: `openspec/changes/add-skill-proposal-processing-lane/tasks.md`
- Create: `openspec/changes/add-skill-proposal-processing-lane/specs/skill-governance-curation/spec.md`

- [ ] **Step 1: Write the proposal and spec delta**

Use the exact OpenSpec files created in this plan. The delta must add requirements for service-owned proposal processing, duplicate suppression, quality scoring, processing snapshots, and thin shell boundaries.

- [ ] **Step 2: Validate the change**

Run: `openspec validate add-skill-proposal-processing-lane --strict`

Expected: validation succeeds with no errors.

### Task 2: Skill Contract DTOs

**Files:**
- Modify: `macaca/crates/services/macaca-skill/src/proposal_processing.rs`
- Modify: `macaca/crates/services/macaca-skill/src/lib.rs`
- Modify: `macaca/crates/services/macaca-skill/src/service_contract.rs`

- [ ] **Step 1: Run impact analysis before editing exported Skill service symbols**

Run GitNexus impact for `SkillExperienceProposalRecord`, `skill_service_descriptor`, and `SystemSkillClient` before edits.

- [ ] **Step 2: Write failing DTO tests**

Add tests that assert:

```rust
#[test]
fn processing_signature_is_bounded_and_generic() {
    let signature = SkillProposalDuplicateSignature::from_parts(
        "Verified terminal task completion observed through service.agent_execution; output_chars=10, artifact_count=0, token_total=unavailable.",
        &SkillEvolutionCandidateClassification::ReusableProcedure,
        &SkillExperienceCandidateDestination::NewSkillDraft,
        &SkillEvolutionProposalAction::CreateDraft,
        None,
    );
    assert!(signature.value.len() <= 160);
    assert!(!signature.value.contains("provider_payload"));
}
```

- [ ] **Step 3: Verify RED**

Run: `cargo test -p macaca-skill processing_signature_is_bounded_and_generic`

Expected: failure because `proposal_processing` DTOs do not exist yet.

- [ ] **Step 4: Implement DTOs and validation**

Add enums and structs for `SkillProposalProcessingState`, `SkillProposalQualityScore`, `SkillProposalProcessingRecord`, `SkillProposalProcessingRunCommand`, `SkillProposalProcessingRunResult`, `SkillProposalProcessingSnapshotCommand`, and `SkillProposalProcessingSnapshotResult`. Include English comments explaining ownership, state semantics, sanitization, and non-materialization.

- [ ] **Step 5: Verify GREEN**

Run: `cargo test -p macaca-skill processing_signature_is_bounded_and_generic`

Expected: pass.

### Task 3: Runtime-Host Processing Strategy

**Files:**
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider_state.rs`
- Create: `macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider_proposal_processing.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/lib.rs`

- [ ] **Step 1: Run impact analysis before editing provider state and command routing**

Run GitNexus impact for `SkillProviderGovernanceState` and `SkillSystemServiceProvider`.

- [ ] **Step 2: Write failing provider tests**

Add tests proving dry-run is immutable, apply marks one duplicate group ready and suppresses later duplicates, missing policy refs reject apply, and snapshot returns state counts.

- [ ] **Step 3: Verify RED**

Run: `cargo test -p macaca-runtime-host skill_proposal_processing`

Expected: failure because processing command routing is missing.

- [ ] **Step 4: Implement deterministic Strategy**

Implement processing over existing proposal records. The Strategy must log run start, candidate count, duplicate group count, ready count, suppressed count, rejected count, policy ref count, evidence ref count, and mutation status. Apply mode must mutate only processing state, never proposal lifecycle or Skill package files.

- [ ] **Step 5: Verify GREEN**

Run: `cargo test -p macaca-runtime-host skill_proposal_processing`

Expected: all focused processing tests pass.

### Task 4: SDK Facade

**Files:**
- Modify: `macaca/crates/facade/macaca-sdk/src/skill_client.rs`
- Modify: `macaca/crates/facade/macaca-sdk/src/skill_client_service_backed.rs`

- [ ] **Step 1: Run impact analysis before editing SDK trait symbols**

Run GitNexus impact for `SystemSkillClient`.

- [ ] **Step 2: Write failing SDK tests**

Add tests for unavailable behavior and service-backed forwarding for the two new processing commands.

- [ ] **Step 3: Verify RED**

Run: `cargo test -p macaca-sdk skill_proposal_processing`

Expected: failure because SDK methods do not exist.

- [ ] **Step 4: Implement facade methods**

Add `process_skill_proposals` and `skill_proposal_processing_snapshot` to the trait, Null Object client, and service-backed client. Each method must forward typed commands and preserve structured unavailable errors.

- [ ] **Step 5: Verify GREEN**

Run: `cargo test -p macaca-sdk skill_proposal_processing`

Expected: focused SDK tests pass.

### Task 5: Web Operations Adapter

**Files:**
- Modify: `macaca/crates/shells/macaca-web/src/skill_operations_routes.rs`

- [ ] **Step 1: Run impact analysis before editing Web route symbols**

Run GitNexus impact for `get_skill_operations` and any route registration symbol touched.

- [ ] **Step 2: Write or update route tests if local patterns exist**

The test should prove the route includes a `processing` field from the Skill service snapshot and does not compute quality or duplicate status in Web.

- [ ] **Step 3: Implement thin adapter changes**

Call the SDK processing snapshot command from `get_skill_operations` and include the returned DTO under `"processing"`. If adding a POST route is necessary, it must only convert transport fields to `SkillProposalProcessingRunCommand` and forward through SDK.

- [ ] **Step 4: Verify focused Web tests or compile check**

Run the narrowest available Web test. If no focused test exists, run `cargo check -p macaca-web`.

Expected: command exits with status 0.

### Task 6: Full Verification And OpenSpec Task Closure

**Files:**
- Modify: `openspec/changes/add-skill-proposal-processing-lane/tasks.md`

- [ ] **Step 1: Run targeted tests**

Run:

```bash
cd macaca
cargo test -p macaca-skill proposal_processing
cargo test -p macaca-runtime-host skill_proposal_processing
cargo test -p macaca-sdk skill_proposal_processing
cargo check -p macaca-web
```

- [ ] **Step 2: Run boundary and spec checks**

Run:

```bash
openspec validate add-skill-proposal-processing-lane --strict
cd macaca
cargo test -p macaca-integration-tests route_c_dependency_boundaries
cargo test -p macaca-integration-tests route_c_baseline
```

- [ ] **Step 3: Run formatting and change detection**

Run:

```bash
git diff --check
```

Then run GitNexus `detect_changes` for the repo before committing.

- [ ] **Step 4: Update task checkboxes only after verification**

Mark OpenSpec tasks complete only for commands that actually passed. Leave any unverified item unchecked and document the blocker.
