## 1. OpenSpec

- [x] 1.1 Create proposal, design, tasks, and delta spec.
- [x] 1.2 Validate `add-evolution-release-safety-chain` with `--strict`.

## 2. Service Contract And Release Strategy

- [x] 2.1 Add release action, status, policy, memento, command, finding, and
  result DTOs.
- [x] 2.2 Add executable release safety Strategy with quarantine, canary,
  promotion, monitoring, rollback, supersede, reject, inconclusive, and dry-run
  outcomes.
- [x] 2.3 Add service trait command, unavailable behavior, descriptor
  capability, and in-memory provider support with structured logs.

## 3. SDK And Runtime Host

- [x] 3.1 Add SDK facade method and unavailable release behavior.
- [x] 3.2 Add runtime-host command decoding for release safety without owning
  policy semantics.

## 4. Verification

- [x] 4.1 Add tests for dry-run, canary pass, canary failure, rollback memento,
  high-blast-radius denial, SDK unavailable, and runtime-host decoding.
- [x] 4.2 Run targeted Rust tests.
- [x] 4.3 Run `openspec validate add-evolution-release-safety-chain --strict`.
- [x] 4.4 Run `git diff --check`.
