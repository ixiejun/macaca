## 1. Preparation

- [x] 1.1 Read `docs/superpowers/plans/2026-06-07-unify-application-execution-path.md`.
- [x] 1.2 Re-read `macaca/docs/macaca-os-architecture-governance.md`, `macaca/docs/macaca-os-microkernel-boundaries.md`, and `macaca/docs/macaca-os-serviceization-allowlist.md`.
- [x] 1.3 Run `openspec validate unify-application-execution-path --strict`.
- [x] 1.4 Run GitNexus impact analysis before editing existing symbols; record CRITICAL/HIGH warnings as memo only unless they identify a direct correctness risk.
- [x] 1.5 Inspect current execution paths in `service.application_execution`, `service.task`, `service.agent_execution`, WASM host imports, YAML adapters, and Web loop compatibility code.

## 2. Contract And Ownership Model

- [x] 2.1 Add or confirm provider-neutral execution ownership fields in application-execution DTOs.
- [x] 2.2 Add or confirm task graph ownership fields in Task Service commands/results.
- [x] 2.3 Add English comments explaining that ownership markers are service categories, not application names or workflow names.
- [x] 2.4 Add tests proving WASM and YAML shaped requests produce equivalent execution envelopes.

## 3. Task Service Single Graph Admission

- [x] 3.1 Add Task Service tests that reject or compatibility-scope a second authoritative graph for one application execution session.
- [x] 3.2 Implement Task Service graph admission rules.
- [ ] 3.3 Emit structured logs for graph admitted, graph rejected, compatibility graph admitted, task claimed, task reviewed, task failed, and graph terminal projected.
- [x] 3.4 Run `cargo test -p macaca-task --lib`.

## 4. Compatibility Fallback Containment

- [ ] 4.1 Add a regression test reproducing the Workbench-shaped failure without referencing Workbench by name in service/runtime code.
- [x] 4.2 Move Web loop fallback decomposition to a Task Service compatibility strategy or adapter command.
- [x] 4.3 Ensure compatibility fallback failures emit diagnostics but cannot mark application execution terminal failed.
- [x] 4.4 Run `cargo test -p macaca-web loop_manager --lib`.

## 5. Hosted Execution Terminal Projection

- [x] 5.1 Add runtime-host tests with mixed authoritative, compatibility, and diagnostic host command rows.
- [x] 5.2 Update hosted execution aggregation to compute terminal state only from authoritative application-execution task graph rows.
- [x] 5.3 Emit diagnostic events for non-authoritative failed rows with bounded counts and reason codes.
- [x] 5.4 Run `cargo test -p macaca-runtime-host application_execution_hosted --lib`.

## 6. WASM And YAML Adapter Convergence

- [ ] 6.1 Add tests proving WASM `agent_delegate` and YAML workflow steps both traverse `service.application_execution -> service.task -> service.agent_execution`.
- [x] 6.2 Update WASM host import bridge to submit application intent and agent work through the unified service chain.
- [ ] 6.3 Update YAML application adapters to use the same command chain.
- [ ] 6.4 Run targeted WASM and YAML adapter tests.

## 7. Shell And UI Projection Boundary

- [ ] 7.1 Add tests proving Web routes call SDK/SystemFacade clients instead of owning provider loops, task graph semantics, or terminal projection.
- [ ] 7.2 Update application execution routes and session routes to expose projected state from service-owned data only.
- [ ] 7.3 Update frontend and app-owned UI caches so local event arrays are render-only mementos.
- [ ] 7.4 Verify refresh/replay keeps session logs, events, Task Board, and current state consistent.

## 8. End-To-End Proof

- [ ] 8.1 Build and restart backend using the new `macaca-web-server` binary.
- [ ] 8.2 Build and install the latest app-owned UI bundles when app artifacts change.
- [ ] 8.3 Run one WASM app-owned task and one YAML app task.
- [ ] 8.4 Confirm both tasks have one authoritative execution session, one authoritative task graph, one terminal current-state, and replayable EventLog rows.
- [ ] 8.5 Confirm compatibility diagnostics do not pollute Task Board terminal state.
- [ ] 8.6 Record sanitized proof evidence without raw provider payloads or secrets.

## 9. Verification And Commit

- [x] 9.1 Run `openspec validate unify-application-execution-path --strict`.
- [x] 9.2 Run `cargo fmt` from `macaca/`.
- [ ] 9.3 Run targeted Rust tests for `macaca-task`, `macaca-runtime-host`, `macaca-web`, and YAML/WASM application adapters.
- [ ] 9.4 Run frontend/app UI tests if UI files changed.
- [x] 9.5 Run GitNexus detect changes and record CRITICAL/HIGH warnings as memo only unless they identify a direct correctness issue.
- [x] 9.6 Commit the OpenSpec, implementation, tests, and sanitized evidence in reviewable commits.
