# Self-Evolution Evaluation Harness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a generic, auditable harness that proves Macaca skill self-evolution happened and measures whether evolved skills improve later real tasks.

**Architecture:** Add a provider-neutral evaluation contract close to skill governance, with deterministic scoring helpers before any live service wiring. The harness uses Command, Facade, Strategy, Decorator, State, Observer, Memento, and Specification patterns so metrics stay replaceable, traceable, auditable, and application-agnostic.

**Tech Stack:** Rust workspace under `macaca/`, OpenSpec, Store/EventLog refs, Skill governance service DTOs, SDK/SystemFacade, Web/CLI/frontend thin adapters where needed.

---

## File Structure

- Create `macaca/crates/services/macaca-skill/src/evaluation/mod.rs` for the evaluation module boundary and public exports.
- Create `macaca/crates/services/macaca-skill/src/evaluation/model.rs` for provider-neutral DTOs and lifecycle state.
- Create `macaca/crates/services/macaca-skill/src/evaluation/scoring.rs` for deterministic white-box and black-box scoring.
- Create `macaca/crates/services/macaca-skill/src/evaluation/report.rs` for sanitized JSON and Markdown report summaries.
- Modify `macaca/crates/services/macaca-skill/src/lib.rs` to export the evaluation module.
- Add tests near existing skill governance tests, following the current crate layout after inspection.
- Later tasks may modify SDK, runtime-host, Web, CLI, or frontend only after the model and scoring contract are stable.

## Task 1: Specification Baseline

**Files:**
- Create: `docs/superpowers/specs/2026-05-23-self-evolution-evaluation-harness-design.md`
- Create: `openspec/changes/add-self-evolution-evaluation-harness/proposal.md`
- Create: `openspec/changes/add-self-evolution-evaluation-harness/design.md`
- Create: `openspec/changes/add-self-evolution-evaluation-harness/tasks.md`
- Create: `openspec/changes/add-self-evolution-evaluation-harness/specs/skill-governance-curation/spec.md`
- Create: `docs/superpowers/plans/2026-05-23-self-evolution-evaluation-harness.md`

- [ ] **Step 1: Validate the OpenSpec change**

Run: `openspec validate add-self-evolution-evaluation-harness --strict`

Expected: validation passes with no malformed requirement or scenario errors.

- [ ] **Step 2: Check documentation whitespace**

Run: `git diff --check`

Expected: no trailing whitespace or conflict marker output.

- [ ] **Step 3: Commit the specification baseline**

```bash
git add docs/superpowers/specs/2026-05-23-self-evolution-evaluation-harness-design.md docs/superpowers/plans/2026-05-23-self-evolution-evaluation-harness.md openspec/changes/add-self-evolution-evaluation-harness
git commit -m "spec: add self-evolution evaluation harness"
```

Expected: commit succeeds and includes only the new design, plan, and OpenSpec files.

## Task 2: Evaluation Model

**Files:**
- Create: `macaca/crates/services/macaca-skill/src/evaluation/mod.rs`
- Create: `macaca/crates/services/macaca-skill/src/evaluation/model.rs`
- Modify: `macaca/crates/services/macaca-skill/src/lib.rs`
- Test: existing `macaca-skill` unit test target or a new focused test module under the crate's current test layout.

- [ ] **Step 1: Inspect current crate layout**

Run: `find macaca/crates/services/macaca-skill/src -maxdepth 2 -type f | sort`

Expected: identify the current module export style and test placement before editing.

- [ ] **Step 2: Write failing model tests**

Add tests that construct an evaluation record with:

```rust
let record = SelfEvolutionEvaluationRecord {
    evaluation_id: "eval-1".into(),
    trace_id: "trace-1".into(),
    task_family_id: "spec_change_loop".into(),
    lifecycle: SelfEvolutionEvaluationLifecycle::Prepared,
    white_box: SelfEvolutionWhiteBoxEvidence::default(),
    baseline: SelfEvolutionRunMetrics::default(),
    evolved: SelfEvolutionRunMetrics::default(),
    report_refs: SelfEvolutionReportRefs::default(),
};
assert_eq!(record.lifecycle, SelfEvolutionEvaluationLifecycle::Prepared);
```

Expected: test fails until DTOs exist.

- [ ] **Step 3: Implement model DTOs**

Implement:

```rust
pub enum SelfEvolutionEvaluationLifecycle {
    Prepared,
    BaselineRecorded,
    EvolvedRecorded,
    Scored,
    Passed,
    Failed,
    Inconclusive,
}

pub struct SelfEvolutionEvaluationRecord { ... }
pub struct SelfEvolutionWhiteBoxEvidence { ... }
pub struct SelfEvolutionRunMetrics { ... }
pub struct SelfEvolutionReportRefs { ... }
pub struct SelfEvolutionScore { ... }
```

Every non-obvious type needs English comments explaining ownership, trace/audit behavior, and why fields store refs rather than raw payloads.

- [ ] **Step 4: Run focused tests**

Run from `macaca/`: `cargo test -p macaca-skill self_evolution_evaluation -- --nocapture`

Expected: new model tests pass.

- [ ] **Step 5: Commit**

```bash
git add macaca/crates/services/macaca-skill/src/lib.rs macaca/crates/services/macaca-skill/src/evaluation
git commit -m "feat: add self-evolution evaluation model"
```

## Task 3: Deterministic Scoring

**Files:**
- Create: `macaca/crates/services/macaca-skill/src/evaluation/scoring.rs`
- Modify: `macaca/crates/services/macaca-skill/src/evaluation/mod.rs`
- Test: focused scoring tests in the current crate test layout.

- [ ] **Step 1: Write failing white-box scoring tests**

Test complete evidence passes and missing proposal, curation, promotion/catalog, or activation evidence fails or becomes inconclusive with bounded reason codes.

- [ ] **Step 2: Write failing black-box scoring tests**

Test pass, no activation failure, completion regression failure, policy regression failure, and inconclusive missing baseline/evolved metrics.

- [ ] **Step 3: Implement scoring Strategy**

Add a deterministic scorer that returns `SelfEvolutionScore` with lifecycle, pass/fail state, reason codes, and bounded diagnostic text. Do not inspect application names or raw task output.

- [ ] **Step 4: Run focused scoring tests**

Run from `macaca/`: `cargo test -p macaca-skill self_evolution_evaluation_scoring -- --nocapture`

Expected: all scoring tests pass.

- [ ] **Step 5: Commit**

```bash
git add macaca/crates/services/macaca-skill/src/evaluation
git commit -m "feat: score self-evolution evaluation runs"
```

## Task 4: Sanitized Reports

**Files:**
- Create: `macaca/crates/services/macaca-skill/src/evaluation/report.rs`
- Modify: `macaca/crates/services/macaca-skill/src/evaluation/mod.rs`
- Test: focused report tests in the current crate test layout.

- [ ] **Step 1: Write failing report tests**

Test JSON and Markdown summaries include evaluation id, trace id, task family id, metric counts, checkpoint refs, score state, and bounded reasons. Include a sensitive fixture string and verify it is omitted.

- [ ] **Step 2: Implement report builders**

Add JSON-compatible and Markdown summary builders that only render bounded refs, counts, states, and reason codes. Comments must explain why reports intentionally avoid raw prompts, provider payloads, manifests, package bytes, and full skill bodies.

- [ ] **Step 3: Add checkpoint logs**

Use the crate's existing logging style to record report creation and scoring decisions with evaluation id, trace id, phase, count fields, and result state.

- [ ] **Step 4: Run report tests**

Run from `macaca/`: `cargo test -p macaca-skill self_evolution_evaluation_report -- --nocapture`

Expected: report tests pass and sensitive fixture text is absent.

- [ ] **Step 5: Commit**

```bash
git add macaca/crates/services/macaca-skill/src/evaluation
git commit -m "feat: report self-evolution evaluation results"
```

## Task 5: Service/SDK Wiring Plan Checkpoint

**Files:**
- Inspect before editing: SDK skill client, runtime-host skill provider, Web/CLI operation routes, and existing Skill governance command DTOs.
- Modify only after impact analysis: exact files determined by current symbol ownership.

- [ ] **Step 1: Run GitNexus impact analysis for each symbol before editing**

Run the appropriate GitNexus impact command for the specific symbol selected after inspection. HIGH/CRITICAL output is recorded as routing input and does not block implementation per operator instruction.

- [ ] **Step 2: Write failing SDK/provider tests**

Add tests proving unavailable behavior, typed command forwarding, and no shell-owned scoring semantics.

- [ ] **Step 3: Implement minimal command/facade wiring**

Expose evaluation commands through the existing service/facade pattern without provider construction in SDK or shell layers.

- [ ] **Step 4: Run targeted service and SDK checks**

Run the focused cargo tests/checks identified by the touched crates.

- [ ] **Step 5: Commit**

```bash
git add <touched service/sdk/runtime-host files>
git commit -m "feat: wire self-evolution evaluation facade"
```

## Task 6: Final Verification

**Files:**
- Modify: `openspec/changes/add-self-evolution-evaluation-harness/tasks.md`
- Potentially modify: runbook docs if created during implementation.

- [ ] **Step 1: Update OpenSpec task checklist truthfully**

Mark only completed tasks in `openspec/changes/add-self-evolution-evaluation-harness/tasks.md`.

- [ ] **Step 2: Run validation commands**

Run:

```bash
openspec validate add-self-evolution-evaluation-harness --strict
git diff --check
```

Run targeted cargo checks/tests for every touched Rust crate. Run frontend lint/build only if frontend files were touched.

- [ ] **Step 3: Run GitNexus detect changes**

Run: `npx gitnexus detect_changes`

Expected: affected symbols and flows match the evaluation harness scope. HIGH/CRITICAL warnings are recorded but do not block per operator instruction.

- [ ] **Step 4: Commit verification updates**

```bash
git add openspec/changes/add-self-evolution-evaluation-harness/tasks.md <any runbook docs>
git commit -m "docs: finalize self-evolution evaluation harness tasks"
```

## Self-Review

- Spec coverage: the OpenSpec delta covers white-box completion, incomplete-chain rejection, black-box improvement, sanitized reports, and shell ownership. Tasks cover spec, model, scoring, reports, service/SDK wiring, and verification.
- Placeholder scan: implementation-specific file paths are identified where the codebase must be inspected first; Task 5 requires impact analysis before exact touched files are finalized.
- Type consistency: all planned DTO names share the `SelfEvolution...` prefix and stay inside the `macaca-skill` evaluation module until facade wiring is planned from current symbols.
