# S2 Kernel 去 Provider 依赖 Implementation Plan

## Scope

Implement S2 from `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`: remove provider ownership from `macaca-kernel`.

S2 reduces kernel coupling to LLM, tools, memory, task, and persistence provider crates by moving direct provider construction behind compatibility adapters, facades, and later service-runtime paths. S2 keeps behavior additive-first and does not remove current allowlist debt unless the code genuinely no longer needs it.

## Required Governance Inputs

- `macaca/docs/agent-os-microkernel-boundaries.md`
- `macaca/docs/route-c-serviceization-allowlist.md`
- `macaca/docs/route-c-architecture-governance.md`
- `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`
- `docs/superpowers/plans/2026-05-08-s1-service-runtime-v1-plan.md`
- Current `macaca-kernel` direct provider call sites and builders

## Architecture Decision

Use a kernel-facade + compatibility-adapter strategy.

Design patterns:

- Facade: expose a smaller kernel surface that stays provider-neutral.
- Adapter: isolate provider-facing compatibility code so it can be removed later.
- Dependency Inversion: shift kernel composition to abstractions and runtime/facade inputs.
- Strategy: keep scheduler and policy choices strategy-based.
- Bridge: runtime/service calls should flow through host-owned runtime or kernel adapters, not direct provider ownership.
- Command: old construction paths should translate into compatibility commands or facade calls where possible.
- Observer: keep trace/audit visibility around lifecycle and compatibility operations.

Rejected alternatives:

- Kernel-owned provider runtime: rejected because it preserves macro-kernel ownership.
- Fence-only approach without real dependency removal: rejected because it changes style, not architecture.
- Big-bang removal of all provider refs in one phase: rejected because it is too risky and not additive-first.

## Proposed OpenSpec Change

Expected change id:

- `update-kernel-to-provider-facade-boundary`

Expected artifacts:

- `openspec/changes/update-kernel-to-provider-facade-boundary/proposal.md`
- `openspec/changes/update-kernel-to-provider-facade-boundary/design.md`
- `openspec/changes/update-kernel-to-provider-facade-boundary/tasks.md`
- `openspec/changes/update-kernel-to-provider-facade-boundary/specs/kernel-facade/spec.md`

The proposal should explicitly state:

- Kernel provider construction is deprecated and being removed from the kernel core.
- Kernel continues to work during migration through compatibility shims.
- Existing behavior remains unchanged for current users.
- S2 is not the provider migration itself; it is the kernel boundary cleanup that enables later service migrations.

## Implementation Slices

### Slice S2.1: Impact and Dependency Check

Files to inspect before editing:

- `macaca/crates/macaca-kernel/Cargo.toml`
- `macaca/crates/macaca-kernel/src/lib.rs`
- `macaca/crates/macaca-kernel/src/kernel.rs`
- `macaca/crates/macaca-kernel/src/kernel_builder.rs`
- `macaca/crates/macaca-kernel/src/services.rs`
- `macaca/crates/macaca-kernel/src/scheduler.rs`
- `macaca/crates/macaca-kernel/src/registry.rs`
- `macaca/crates/macaca-kernel/src/facade.rs`

Required actions:

1. Run GitNexus impact before modifying any existing symbol.
2. Warn the user before touching HIGH or CRITICAL impact symbols.
3. Verify the S0 dependency gate scope before any dependency edits.

### Slice S2.2: Kernel Facade Boundary

Files:

- Modify: `macaca/crates/macaca-kernel/src/facade.rs`
- Modify: `macaca/crates/macaca-kernel/src/lib.rs`

Behavior:

- Keep `KernelFacade` as the narrow primitive composition surface.
- Ensure facade methods and exports emphasize invariants, registries, policy, resources, and trace.
- Do not expose provider construction as a kernel core responsibility.

Rules:

- Comments must explain why the facade is the kernel boundary.
- Add structured logs to any new compatibility-aware facade operations.
- Avoid app/provider/workflow/model hardcoding.

### Slice S2.3: Compatibility Adapter Module

Files:

- New: `macaca/crates/macaca-kernel/src/provider_compat.rs`

Define:

- provider-neutral compatibility wrappers for old kernel construction paths,
- deprecated adapter entry points for existing provider-backed flows,
- migration diagnostics that direct callers toward `ServiceRuntime` or facade-based injection,
- helper types for scheduler/test compatibility if needed.

Rules:

- This module is temporary and explicitly migration-only.
- It must not become the new long-term provider ownership center.
- It should not invent new provider abstractions that duplicate runtime-host.
- All adapter branches must be documented with why they exist and when they expire.

### Slice S2.4: Kernel Constructor Refactor

Files:

- Modify: `macaca/crates/macaca-kernel/src/kernel.rs`
- Modify: `macaca/crates/macaca-kernel/src/kernel_builder.rs`

Behavior:

1. Mark direct provider-construction entry points deprecated.
2. Add new facade-oriented or adapter-backed constructors.
3. Preserve current runtime behavior through compatibility shims.
4. Move direct provider ownership out of the main kernel construction flow.
5. Keep existing tests passing while steering new callers toward the new path.

Rules:

- Any old constructor must include a precise deprecation note that names the replacement path.
- New constructors must not require callers to pass raw provider implementations unless they are clearly adapter-only and deprecated.
- Kernel should no longer treat LLM/tools/provider crates as its primary ownership model.

### Slice S2.5: Registry, Scheduler, and Service Adapter Cleanup

Files:

- Modify: `macaca/crates/macaca-kernel/src/registry.rs`
- Modify: `macaca/crates/macaca-kernel/src/scheduler.rs`
- Modify: `macaca/crates/macaca-kernel/src/services.rs`
- Possibly modify: `macaca/crates/macaca-kernel/src/service_bus_bridge.rs`

Behavior:

- Replace test and helper dependencies on concrete provider crates with provider-neutral mocks or facade-based adapters where possible.
- Move memory/persist adapter wiring into explicit compat boundaries.
- Make scheduler and registry tests independent of direct provider construction where feasible.

Rules:

- Keep `services.rs` as an adapter layer, not a hidden provider hub.
- Preserve trace and logging around kernel operations.
- If any provider import is still required for compatibility, isolate it to the temporary compat module.

### Slice S2.6: Dependency Pruning

Files:

- Modify: `macaca/crates/macaca-kernel/Cargo.toml`
- Potentially modify workspace tests and imports in `macaca/crates/macaca-kernel/src/*`

Behavior:

- Remove direct kernel dependencies that are no longer required after compat extraction.
- Prefer moving imports to tests, compat modules, or runtime-host facades rather than leaving them in core kernel code.
- Keep `macaca-proto`, `macaca-ipc`, and microkernel primitive dependencies.

Rules:

- If a dependency is still needed for deprecated compat only, make that explicit in code structure and comments.
- Do not introduce new provider dependencies.

### Slice S2.7: Tests

Files:

- Update: `macaca/crates/macaca-kernel/src/kernel.rs` tests
- Update: `macaca/crates/macaca-kernel/src/kernel_builder.rs` tests
- Update: `macaca/crates/macaca-kernel/src/registry.rs` tests
- Update: `macaca/crates/macaca-kernel/src/scheduler.rs` tests
- New tests if needed: `macaca/crates/macaca-kernel/tests/kernel_facade.rs`

Test cases:

1. Kernel still constructs and executes current flows through deprecated shims.
2. New facade-oriented construction path works.
3. Deprecated constructor emits a clear migration message.
4. Registry and scheduler tests no longer require unnecessary provider coupling.
5. Trace/log boundary behavior remains intact.
6. Dependency gate continues to pass for kernel boundary rules.

### Slice S2.8: Documentation

Files:

- Update: `macaca/docs/route-c-architecture-governance.md`
- Potentially update: `macaca/docs/agent-os-microkernel-boundaries.md` if wording needs clarification around kernel compatibility shims

Documentation must state:

- Kernel core owns invariants and facades, not provider construction.
- Deprecated constructor paths exist only for migration.
- Kernel boundary cleanup is a prerequisite for later provider/service migration phases.

## Dependency Boundary Expectations

Potential effects:

- Kernel should stop being a primary consumer of `macaca-llm`, `macaca-tools`, `macaca-memory`, `macaca-task`, and `macaca-persist` in its core modules.
- Some temporary compat imports may remain in a dedicated adapter module.

Expected S0 gate outcome:

- New forbidden kernel-to-provider edges should be reduced, not added.
- Existing allowlist entries should only remain for genuine migration debt.
- If a needed compat edge is discovered, update OpenSpec and allowlist explicitly rather than burying it in core kernel logic.

## Verification

Run after implementation:

```bash
openspec validate update-kernel-to-provider-facade-boundary --strict
cargo fmt --check
cargo test -p macaca-kernel
cargo test -p macaca-integration-tests route_c_dependency_boundaries
cargo test -p macaca-integration-tests --test route_c_baseline
cargo check -p macaca-kernel
cargo check --workspace
npx gitnexus detect-changes --repo agent
```

Expected result:

- Kernel tests pass.
- Route C dependency gate remains green.
- Any remaining provider coupling is isolated, documented, deprecated, and temporary.

## Completion Criteria

- Superpowers brainstorm and plan exist.
- OpenSpec proposal/design/tasks/spec exists and validates before implementation.
- Kernel core construction path no longer treats provider implementation crates as its primary ownership model.
- Deprecated constructors exist only as migration shims with explicit replacement paths.
- Provider-facing compatibility code is isolated and reviewable.
- Trace/log behavior remains intact.
- No unrelated consumer-facing behavior regresses.
