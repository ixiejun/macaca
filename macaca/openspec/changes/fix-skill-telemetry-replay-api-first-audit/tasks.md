## 1. Implementation

- [x] 1.1 Add OpenSpec requirements for durable usage telemetry replay and API-first audit verification.
- [x] 1.2 Add a failing runtime-host test proving `Activated` and `SuccessfulTask` counters replay after provider restart.
- [x] 1.3 Implement the local Skill governance event journal memento and provider startup replay.
- [x] 1.4 Wire the generic workspace journal path from the Web composition root.
- [x] 1.5 Add a failing web adapter test proving API-first audit reports missing operations, registry, or observer evidence.
- [x] 1.6 Implement the API-first self-evolution audit/trigger verification route.
- [x] 1.7 Update the monitoring report with verification results.

## 2. Validation

- [x] 2.1 Run `openspec validate fix-skill-telemetry-replay-api-first-audit --strict`.
- [x] 2.2 Run targeted runtime-host Skill provider tests.
- [x] 2.3 Run targeted macaca-web audit adapter tests.
- [x] 2.4 Run `cargo check -p macaca-runtime-host -p macaca-web`.
- [x] 2.5 Run `git diff --check`.
