## 1. OpenSpec

- [x] 1.1 Create proposal, design, tasks, and delta spec.
- [x] 1.2 Validate `add-evolution-admission-quality-gates` with `--strict`.

## 2. Service Contract And Specifications

- [x] 2.1 Add admission DTOs and decision states to `macaca-autonomy-evolution`.
- [x] 2.2 Add executable Specification gates for naming, triggers, body focus,
  resources, validation refs, forward-test refs, duplicates, and stale metadata.
- [x] 2.3 Add service trait command, unavailable behavior, and in-memory provider
  support with structured logs.

## 3. SDK And Runtime Host

- [x] 3.1 Add SDK facade method for admission and unavailable client behavior.
- [x] 3.2 Add runtime-host command decoding for admission without owning
  semantics.

## 4. Verification

- [x] 4.1 Add tests for meaningless `skill-exp-*` names, good semantic names,
  missing trigger quality, duplicate candidates, stale metadata, and sanitized
  denial reasons.
- [x] 4.2 Run targeted Rust tests.
- [x] 4.3 Run `openspec validate add-evolution-admission-quality-gates --strict`.
- [x] 4.4 Run `git diff --check`.
