# S2 Kernel 去 Provider 依赖 Brainstorm

## Context

S2 follows `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`. S0 has already introduced an executable dependency gate, and S1 has introduced `ServiceRuntime v1` in `macaca-runtime-host`. The next move is to shrink `macaca-kernel` so it only owns invariants, primitives, registries, adapters, and facades.

Current kernel state still includes direct provider-style coupling:

- `macaca/crates/macaca-kernel/src/kernel.rs` depends directly on `macaca_llm::LlmProvider` and `macaca_tools::ToolCatalog`.
- `macaca/crates/macaca-kernel/src/kernel_builder.rs` still requires direct provider instances at construction time.
- `macaca/crates/macaca-kernel/Cargo.toml` still links directly to `macaca-llm`, `macaca-tools`, `macaca-memory`, `macaca-task`, and `macaca-persist`.
- `macaca/crates/macaca-kernel/src/services.rs` still contains adapter wiring around memory and persistence.
- `macaca/crates/macaca-kernel/src/registry.rs` tests still rely on provider traits.
- `macaca/crates/macaca-kernel/src/scheduler.rs` tests still rely on provider traits.

The Route C boundary documents say the kernel should own invariants, not replaceable provider implementations. S2 must therefore keep the kernel usable while removing provider ownership from the kernel API and moving compatibility paths behind adapters, facades, and service/runtime boundaries.

## Constraints

- Must strictly follow `macaca/docs/agent-os-microkernel-boundaries.md`.
- Must preserve `macaca/docs/route-c-serviceization-allowlist.md` as a migration-debt snapshot until later phases remove the debt genuinely.
- Must follow `macaca/docs/route-c-architecture-governance.md`: no trace, no call; no permission/policy, no call; no hardcoding provider/app/workflow names.
- Must remain additive-first and preserve existing YAML application loading, `/api/chat/v2`, trace, task board, resume, driver, skill/MCP, Web UI, and CLI behavior.
- Must keep all new code with detailed English comments and structured logs at critical nodes.
- Rust files must stay below 500 lines.

## Design Pattern Candidates

### Option A: Kernel Facade + Compatibility Adapter + Deprecated Constructors

Refactor `macaca-kernel` so provider-neutral kernel primitives remain, while direct provider construction is moved behind a compatibility adapter module. Keep existing constructors as deprecated shims that delegate to new compatibility/facade methods rather than owning provider logic directly.

Patterns:

- Facade: kernel exposes a narrower `Kernel`/`KernelFacade` surface.
- Adapter: provider-facing compatibility code is isolated in `provider_compat`.
- Dependency Inversion: kernel APIs depend on abstractions or facades instead of concrete provider crates.
- Strategy: scheduler and policy behavior remain strategy-driven.
- Bridge: any remaining provider access should go through service/runtime or future compat layers, not kernel-owned branches.
- Decorator: trace/policy/resource observability remains composed outside provider code.

Pros:

- Preserves current behavior while shrinking kernel ownership.
- Makes deprecation path explicit and reviewable.
- Lets later phases remove provider dependencies one by one instead of forcing a big bang rewrite.
- Aligns with the existing allowlist: current direct deps are debt, not architecture.

Cons:

- Requires new compatibility layers and disciplined deprecation management.
- Kernel may still compile against provider crates temporarily while public APIs transition.

Risk:

- If deprecated constructors remain too useful, consumers may never migrate. This is a governance risk, not just a code risk.

### Option B: Extract a New `macaca-kernel-compat` Crate

Move all provider-dependent compatibility code out of `macaca-kernel` into a sibling crate, leaving `macaca-kernel` almost pure.

Pros:

- Strong isolation of provider dependencies.
- Easier to see what remains in kernel and what is compat only.

Cons:

- Adds a new crate and likely more workspace churn.
- May complicate import paths for existing consumers.
- Not necessary if the existing kernel can be narrowed with internal modules and deprecations.

Risk:

- Moderate migration overhead. Useful only if internal module splitting becomes too large.

### Option C: Keep Provider Dependencies in Kernel but Fence Them With Facades

Leave `macaca-kernel` Cargo deps as-is, but route all public use through facade methods and mark direct provider-facing APIs deprecated.

Pros:

- Lowest immediate code churn.
- Easier short-term compile success.

Cons:

- Does not truly remove kernel provider dependency.
- Lets the kernel remain a macro-kernel in practice, even if APIs are nicer.
- Conflicts with S2's explicit goal.

Risk:

- High architectural risk. This is a fallback only if compatibility code cannot be separated safely, but it should not be the recommended route.

### Option D: Move Kernel Provider Access to `macaca-runtime-host` Only

Replace kernel direct provider dependencies with runtime-host service clients, leaving the kernel dependent on `ServiceRuntime` or service registry/facades only.

Pros:

- Best alignment with microkernel boundaries.
- Simplifies the kernel's long-term ownership model.

Cons:

- Larger behavioral shift.
- Requires careful adaptation of existing kernel tests and construction paths.
- Must preserve backward compatibility via deprecated constructors during the transition.

Risk:

- Medium integration risk, but structurally the cleanest long-term direction.

## Recommended Approach

Choose a hybrid of Option A and Option D:

- Keep the kernel functional.
- Introduce a compatibility adapter module inside kernel only as a temporary migration bridge.
- Deprecate provider-facing constructors and direct provider injection.
- Convert internal kernel composition to abstraction-first entry points.
- Move provider access behind service/runtime or facade boundaries where possible.
- Remove direct provider dependencies from runtime-facing kernel code first, then from tests and helper code.

This is the smallest plan that still actually reduces kernel ownership instead of just renaming it.

## Key Risks and Mitigations

- Risk: `Kernel::new` and `KernelBuilder::new` remain the dominant entry point and keep provider coupling alive.
  - Mitigation: introduce new facade-based constructors and mark the old ones deprecated with explicit replacement paths.

- Risk: kernel tests still rely on provider traits and therefore keep the dependency graph alive.
  - Mitigation: migrate tests to provider-neutral mocks, facades, or service runtime test doubles.

- Risk: `services.rs` and `audit.rs` blur the line between kernel primitives and service/provider adapters.
  - Mitigation: split provider-facing adapters into compat modules, leave primitives in kernel core, and preserve trace/audit logs.

- Risk: the S0 dependency gate will fail during transition.
  - Mitigation: use the allowlist as a migration-debt snapshot, but only for edges that are truly still needed during the transition.

- Risk: over-aggressive removal breaks current flows.
  - Mitigation: preserve current user-visible behavior through deprecated shims until service/runtime paths are proven.

- Risk: kernel can no longer create or test `AgentServices` bundles the old way.
  - Mitigation: move to facade-oriented injection and test doubles, not direct provider construction.

## Decision

Proceed with the kernel-facade + compatibility-adapter approach. It is the most pragmatic route to truly reducing provider ownership in `macaca-kernel` without breaking current behavior or forcing a rewrite of the rest of the workspace.
