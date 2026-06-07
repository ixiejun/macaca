## 1. Governance docs

- [x] 1.1 Add `agent-os-microkernel-boundaries.md`.
- [x] 1.2 Add `route-c-regression-matrix.md`.
- [x] 1.3 Add `route-c-phase-template.md`.
- [x] 1.4 Add `route-c-architecture-governance.md`.
- [x] 1.5 Link the new docs from `SYSTEM_OVERVIEW.md` and `refactor-order.md`.

## 2. Baseline verification

- [x] 2.1 Add a no-network Route C baseline integration test.
- [x] 2.2 Ensure the test verifies governance documents cover required scenarios and boundaries.
- [x] 2.3 Run targeted integration tests.

## 3. Validation

- [x] 3.1 Run `openspec validate add-route-c-baseline-governance --strict`.
- [x] 3.2 Run workspace checks needed for changed test code.
- [x] 3.3 Run GitNexus detect_changes before finalizing.
