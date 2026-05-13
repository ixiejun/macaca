## 1. Boundary and Impact
- [x] 1.1 Review Application lifecycle service, WASM provider session, ApplicationHostCommand, default provider dispatch, checkpoint DTOs, and unavailable provider behavior.
- [x] 1.2 Run GitNexus impact analysis for lifecycle and host command symbols.
- [x] 1.3 Confirm lifecycle logic remains generic and does not import concrete business service implementations.

## 2. OpenSpec
- [x] 2.1 Add lifecycle states and allowed transition spec.
- [x] 2.2 Add checkpoint/restore memento contract spec.
- [x] 2.3 Add upgrade/rollback decision spec.
- [x] 2.4 Add transition audit event spec.
- [x] 2.5 Validate OpenSpec change strictly.

## 3. Lifecycle State Machine
- [x] 3.1 Add provider-neutral lifecycle state, operation, transition command, transition result, audit, checkpoint, restore, upgrade, and rollback DTOs with detailed English comments.
- [x] 3.2 Add transition validator with fail-closed reason codes for invalid transition, missing trace, policy denied, unsupported, ABI mismatch, and resource exhausted.
- [x] 3.3 Add focused unit tests for valid and invalid transition behavior.

## 4. Runtime Integration
- [x] 4.1 Add lifecycle methods to the WASM execution session boundary while keeping default fail-closed compatibility.
- [x] 4.2 Integrate lifecycle state machine into default in-process WASM sessions for init, start, event, render, shutdown, checkpoint, restore, upgrade, and rollback.
- [x] 4.3 Return structured unsupported for pause, resume, and drain while the in-process engine has no real suspension/drain support.
- [x] 4.4 Emit sanitized logs and audit metadata for requested, completed, failed, unsupported, drained, checkpointed, restored, upgraded, and rolled-back operations.

## 5. Checkpoint / Restore / Upgrade
- [x] 5.1 Ensure checkpoint metadata excludes raw guest memory and raw command payload.
- [x] 5.2 Add restore compatibility checks for ABI version and artifact hash metadata.
- [x] 5.3 Add upgrade/rollback reports based on artifact id/hash/ABI compatibility without application-specific special cases.
- [x] 5.4 Preserve unavailable-safe fallback semantics.

## 6. Validation
- [x] 6.1 Run `cargo test -p macaca-app wasm_lifecycle --manifest-path macaca/Cargo.toml`.
- [x] 6.2 Run `cargo test -p macaca-runtime-host wasm_lifecycle --manifest-path macaca/Cargo.toml`.
- [x] 6.3 Run `cargo test -p macaca-integration-tests application_platform_contracts --manifest-path macaca/Cargo.toml`.
- [x] 6.4 Run `openspec validate add-wasm-lifecycle-state-checkpoint --strict`.
- [x] 6.5 Run GitNexus detect changes and verify affected scope.
