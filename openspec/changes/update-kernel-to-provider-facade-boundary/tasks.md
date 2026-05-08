## 1. Preparation

- [x] 1.1 Read the S2 brainstorm and implementation plan before editing code.
- [x] 1.2 Review current kernel provider dependencies, constructors, registry, scheduler, and service adapter call sites.
- [x] 1.3 Run GitNexus impact for every existing symbol that will be edited.

## 2. OpenSpec and governance

- [x] 2.1 Create or update the S2 change with a kernel boundary cleanup scope.
- [x] 2.2 Validate the change with `openspec validate update-kernel-to-provider-facade-boundary --strict`.
- [x] 2.3 Confirm the proposal keeps deprecated shims searchable and does not delete migration breadcrumbs.

## 3. Kernel boundary cleanup

- [x] 3.1 Add a temporary provider compatibility adapter module if needed.
- [x] 3.2 Refactor kernel construction toward facade-oriented, provider-neutral entry points.
- [x] 3.3 Mark direct provider-facing kernel constructors and builders deprecated.
- [x] 3.4 Keep current runtime behavior through compatibility shims.

## 4. Dependency and helper cleanup

- [x] 4.1 Reduce direct kernel dependencies on provider crates where feasible.
- [x] 4.2 Isolate any remaining provider imports to temporary compat boundaries.
- [x] 4.3 Preserve structured logs and trace-friendly diagnostics at key nodes.

## 5. Tests and verification

- [x] 5.1 Update kernel tests to prefer provider-neutral doubles where possible.
- [x] 5.2 Keep migration compatibility tests for deprecated paths.
- [x] 5.3 Run route C dependency boundary checks and workspace verification before completion.
