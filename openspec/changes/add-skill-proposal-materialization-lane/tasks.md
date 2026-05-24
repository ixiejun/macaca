## 1. Specification

- [x] 1.1 Create OpenSpec proposal, design, tasks, and delta spec.
- [x] 1.2 Validate with `openspec validate add-skill-proposal-materialization-lane --strict`.

## 2. Service Contract

- [x] 2.1 Add provider-neutral proposal materialization DTOs, status, command,
  result, validation, and command constant.
- [x] 2.2 Export materialization DTOs through `macaca-skill`.
- [x] 2.3 Extend the Skill service descriptor with a materialization capability
  and sanitized permission hint.

## 3. Runtime Host Provider

- [x] 3.1 Add a local materialization Strategy module.
- [x] 3.2 Add a Builder that converts proposal metadata into bounded
  AgentSkills-compatible `SKILL.md` bytes.
- [x] 3.3 Reject materialization unless a proposal has a
  `ReadyForMaterialization` processing record.
- [x] 3.4 In dry-run mode, return digest and planned bytes without file or
  governance mutation.
- [x] 3.5 In apply mode, delegate `SKILL.md` writes to the existing content
  mutation Strategy and promote the proposal only after a successful mutation.
- [x] 3.6 Add structured logs for materialization acceptance, denial, preview,
  apply, mutation result, and promotion.

## 4. Verification

- [x] 4.1 Add TDD coverage for command validation and body-free result
  serialization.
- [x] 4.2 Add runtime tests for non-ready denial, dry-run immutability,
  apply-mode `SKILL.md` creation, proposal promotion, and governance snapshot.
- [x] 4.3 Run focused Rust tests for `macaca-skill` and
  `macaca-runtime-host`.
- [x] 4.4 Run `openspec validate add-skill-proposal-materialization-lane --strict`.
- [x] 4.5 Run `git diff --check` and GitNexus change detection before
  completion.

## 5. Monitoring Report

- [x] 5.1 Update `docs/macaca-agent-self-evolution-live-monitoring-report.md`
  with materialization lane status, verification commands, and remaining
  activation/reuse blocker.
