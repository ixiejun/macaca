## 1. Specification
- [x] 1.1 Add OpenSpec proposal, design, tasks, and delta spec.

## 2. Implementation
- [x] 2.1 Add a focused Web adapter module for governed Skill task outcome telemetry.
- [x] 2.2 Add tests proving successful task commands are generated only for active governed Skills visible in the cached snapshot.
- [x] 2.3 Call the adapter from the Agent Execution completion observer without changing task success semantics.

## 3. Verification
- [x] 3.1 Run focused `macaca-web` tests for Skill telemetry and self-evolution observer paths.
- [x] 3.2 Run `cargo check -p macaca-web`.
- [x] 3.3 Validate the OpenSpec change with `openspec validate record-skill-task-outcome-telemetry --strict`.
- [x] 3.4 Re-run live `/api/chat/v2` follow-up and verify `successful_task_count` increments.
