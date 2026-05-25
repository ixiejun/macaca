## 1. Specification

- [x] 1.1 Create OpenSpec proposal, design, tasks, and delta spec.
- [x] 1.2 Validate with `openspec validate add-autonomy-evolution-control-plane --strict`.

## 2. Service Contract

- [x] 2.1 Add provider-neutral evolution run state, target type, transition command, transition result, snapshot command, and unavailable result DTOs.
- [x] 2.2 Add explicit transition validation requiring trace, scope, evidence refs, and policy decision refs for side-effecting transitions.
- [x] 2.3 Add focused tests for valid transitions, invalid transitions, rejection, rollback, and policy-required transitions.

## 3. Runtime Host Provider Skeleton

- [x] 3.1 Add a service-owned in-memory provider skeleton that records bounded transition evidence.
- [x] 3.2 Add structured logs for transition start, transition denial, policy requirement, adapter dispatch placeholder, evidence append, and terminal state.
- [x] 3.3 Keep target-specific mutation delegated to target adapter Strategies.

## 4. SDK/SystemFacade Boundary

- [x] 4.1 Add focused SDK client methods for transition and snapshot commands.
- [x] 4.2 Add unavailable/null-object behavior that returns structured unavailable results without fake success.

## 5. Verification

- [x] 5.1 Run targeted Rust tests for the new control-plane DTO/state-machine module.
- [x] 5.2 Run dependency-boundary tests that prove the kernel and shells do not own control-plane semantics.
- [x] 5.3 Run `openspec validate add-autonomy-evolution-control-plane --strict`.
- [x] 5.4 Run `git diff --check`.
- [x] 5.5 Run GitNexus change detection before commit.
