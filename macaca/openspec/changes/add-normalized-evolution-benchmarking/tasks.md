## 1. OpenSpec

- [x] 1.1 Create proposal, design, tasks, and delta spec.
- [x] 1.2 Validate `add-normalized-evolution-benchmarking` with `--strict`.

## 2. Service Contract And Scoring Strategy

- [x] 2.1 Add benchmark metric DTOs, command, result, and decision states.
- [x] 2.2 Add executable comparability checks and default scoring Strategy.
- [x] 2.3 Add service trait command, unavailable behavior, and in-memory provider
  support with structured logs.

## 3. SDK And Runtime Host

- [x] 3.1 Add SDK facade method for paired benchmark scoring and unavailable
  behavior.
- [x] 3.2 Add runtime-host command decoding for benchmarking without owning
  scoring semantics.

## 4. Verification

- [x] 4.1 Add tests for pass, quality regression failure, non-comparable task
  family inconclusive, missing metrics inconclusive, regression reason failure,
  SDK unavailable, and runtime-host decoding.
- [x] 4.2 Run targeted Rust tests.
- [x] 4.3 Run `openspec validate add-normalized-evolution-benchmarking --strict`.
- [x] 4.4 Run `git diff --check`.
