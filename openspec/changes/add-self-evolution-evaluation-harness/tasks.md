## 1. Specification

- [x] 1.1 Create OpenSpec proposal, design, tasks, and delta spec.
- [x] 1.2 Validate the OpenSpec change with strict mode.

## 2. Evaluation Contract

- [x] 2.1 Add provider-neutral evaluation DTOs for records, checkpoints,
  metrics, scoring output, report refs, and lifecycle state.
- [x] 2.2 Add deterministic scoring helpers for white-box and black-box gates.
- [x] 2.3 Add sanitized JSON and Markdown report builders.
- [ ] 2.4 Add structured logs for checkpoint append, scoring, report creation,
  and sanitized failure decisions.

## 3. Service And SDK Boundary

- [ ] 3.1 Add typed evaluation commands and results behind the Skill service or
  adjacent evaluation provider boundary.
- [ ] 3.2 Add SDK/SystemFacade methods and Null Object unavailable behavior.
- [ ] 3.3 Wire runtime-host provider construction without kernel, shell, or
  application-specific dependencies.

## 4. Evidence Integration

- [ ] 4.1 Observe verified task completion, ExperienceCandidate,
  classification, proposal, curation, promotion/apply, catalog visibility,
  activation, rejection, and rollback evidence through service boundaries.
- [ ] 4.2 Ensure evaluation checkpoints store refs and bounded counts only.
- [ ] 4.3 Add rollback and before/after snapshot refs to evaluation reports.

## 5. Shell Adapters And Runbook

- [ ] 5.1 Add Web/CLI route or command adapters that display evaluation reports
  without owning scoring semantics.
- [ ] 5.2 Add frontend report display only if the existing operations surface
  needs visibility into evaluation status.
- [ ] 5.3 Document how to run baseline/evolved task-family evaluations.

## 6. Verification

- [ ] 6.1 Test white-box gate success and each required missing-checkpoint
  failure.
- [ ] 6.2 Test black-box pass, no-activation failure, completion regression
  failure, policy regression failure, and inconclusive scoring.
- [ ] 6.3 Test report sanitization and bounded diagnostics.
- [ ] 6.4 Run OpenSpec strict validation, targeted Rust checks/tests, frontend
  lint/build when touched, `git diff --check`, and GitNexus detect changes.
