## 1. Contracts
- [x] 1.1 Add a provider-neutral Heartbeat run completion command and command name.
- [x] 1.2 Expose completion through the Heartbeat service trait, unavailable provider, and runtime-host service adapter.

## 2. Runtime Behavior
- [x] 2.1 Stop marking accepted native profile wakes succeeded before dispatch completion.
- [x] 2.2 Complete heartbeat runs after background dispatch with sanitized outcome metadata.
- [x] 2.3 Preserve nonblocking HeartbeatLane behavior.

## 3. Validation
- [x] 3.1 Add focused tests for successful dispatch completion in run history.
- [x] 3.2 Add focused tests for failed dispatch completion in run history.
- [x] 3.3 Run focused Rust tests.
- [x] 3.4 Run `openspec validate fix-heartbeat-dispatch-outcome-mementos --strict`.
- [x] 3.5 Run GitNexus detect changes.
