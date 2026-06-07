## 1. Specification

- [x] 1.1 Create OpenSpec proposal, design, tasks, and delta spec.
- [x] 1.2 Validate with `openspec validate add-skill-autonomous-materialization-operator --strict`.

## 2. Skill Service Contract

- [x] 2.1 Add provider-neutral autonomous materialization command/result DTOs.
- [x] 2.2 Add command constants and descriptor capability/permission hints.
- [x] 2.3 Add validation and body-free serialization tests.

## 3. Runtime Host Operator

- [x] 3.1 Add a service-owned operator module that composes processing and materialization Strategies.
- [x] 3.2 Add a provider-neutral package target resolver Strategy.
- [x] 3.3 Support dry-run mode without file or governance mutation.
- [x] 3.4 Support apply mode only for ready proposals with policy/evidence/audit refs.
- [x] 3.5 Emit structured logs for processing, selection, materialization, denial, rollback, and aggregate result.

## 4. Operations And SDK Evidence

- [x] 4.1 Add SDK methods for operator run and snapshot/result refs.
- [x] 4.2 Expose body-free operator evidence through Skill operations.
- [x] 4.3 Keep Web routes as thin SDK adapters.

## 5. Verification

- [x] 5.1 Run focused Rust tests for `macaca-skill`, `macaca-runtime-host`, and `macaca-web`.
- [x] 5.2 Run `cargo check -p macaca-web`.
- [x] 5.3 Run `git diff --check` and GitNexus change detection.
- [ ] 5.4 Append live verification and remaining blockers to the monitoring report.
