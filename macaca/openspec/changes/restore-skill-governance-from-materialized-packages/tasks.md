## 1. Specification
- [x] 1.1 Add OpenSpec proposal, design, tasks, and delta spec.

## 2. Implementation
- [x] 2.1 Add a focused failing test for recovering an `Active` governance record from a materialized package after a new provider state is created.
- [x] 2.2 Implement bounded package provenance parsing and recovery in the Skill service provider state.
- [x] 2.3 Wire recovery into the provider startup path without moving semantics into Web or CLI.

## 3. Verification
- [x] 3.1 Run the focused runtime-host recovery test.
- [x] 3.2 Run relevant Skill provider tests.
- [x] 3.3 Run `cargo check -p macaca-runtime-host` and `cargo check -p macaca-web`.
- [x] 3.4 Validate OpenSpec with `openspec validate restore-skill-governance-from-materialized-packages --strict`.
- [x] 3.5 Restart the backend and prove `/skills/operations` restores the materialized Skill governance record.
