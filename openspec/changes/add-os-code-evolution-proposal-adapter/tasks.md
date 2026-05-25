## 1. OpenSpec

- [x] 1.1 Create proposal, design, tasks, and delta spec.
- [x] 1.2 Validate `add-os-code-evolution-proposal-adapter` with `--strict`.

## 2. Adapter Contract

- [x] 2.1 Add provider-neutral OS-code proposal command, input, finding,
  decision, and result DTOs.
- [x] 2.2 Add replaceable `OsCodeEvolutionProposalAdapter` Strategy.
- [x] 2.3 Add default non-mutating Strategy with OpenSpec, Superpowers,
  GitNexus, tests, release gate, rollback, and blast-radius checks.

## 3. Verification

- [x] 3.1 Add tests for ready proposal, missing evidence, high blast-radius
  quarantine, and source mutation denial.
- [x] 3.2 Run targeted Rust tests.
- [x] 3.3 Run `openspec validate add-os-code-evolution-proposal-adapter --strict`.
- [x] 3.4 Run `git diff --check`.
