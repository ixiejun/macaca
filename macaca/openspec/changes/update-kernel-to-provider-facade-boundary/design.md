## Context

`macaca-kernel` remains the system invariant layer, but some construction and helper paths still depend on provider implementation crates. Route C requires the kernel to own primitives, registries, policy, trace, and facades, not provider ownership.

S2 narrows the kernel by moving provider-facing compatibility into a temporary adapter layer and by steering new callers toward provider-neutral or service-runtime-based composition.

## Goals

- Remove provider ownership from the kernel core path.
- Keep current behavior available through deprecated compatibility shims.
- Preserve deterministic logs, traceability, and audit-friendly diagnostics.
- Reduce direct provider dependencies in kernel core modules where possible.

## Non-Goals

- No provider implementation migration.
- No new provider/runtime abstraction that duplicates `macaca-runtime-host`.
- No user-visible workflow, shell, or application semantic change.

## Decisions

### Decision: Use a facade plus temporary compatibility adapter

The kernel should expose a narrower facade-oriented surface while any remaining provider-facing entry points live in a dedicated migration adapter module.

Why:

- The facade keeps kernel ownership centered on invariants.
- The adapter isolates legacy construction paths so they can be removed later.
- This avoids turning compatibility into a second kernel implementation.

### Decision: Keep deprecated constructors callable

Existing direct provider-oriented constructors and builder paths remain callable but are marked deprecated with explicit migration guidance.

Why:

- Existing consumers need a stable migration window.
- Deleted APIs would hide historical call sites and slow migration audits.
- Deprecated shims make the transition visible in compiler output and code search.

### Decision: Move provider-specific wiring out of core modules

If provider imports are still required during migration, they should be isolated to a compatibility module rather than core kernel composition.

Why:

- This keeps the kernel core easier to audit.
- It makes the allowlist debt obvious instead of burying it in normal code paths.
- It reduces the chance that future additions accidentally reuse legacy wiring.

## Alternatives considered

- Keep provider deps in kernel and only deprecate APIs.
  - Rejected because it changes style but not ownership.
- Remove all provider deps in one shot.
  - Rejected because it risks breaking current behavior and violates additive-first migration.
- Create a separate compat crate.
  - Considered viable, but an internal compat module is smaller and sufficient for this phase.

## Risks and Trade-offs

- Risk: deprecated shims become the new default usage.
  - Mitigation: make new facade-oriented or service-runtime-oriented paths the recommended replacement and keep the old APIs obviously deprecated.
- Risk: kernel tests continue to depend on provider crates.
  - Mitigation: migrate tests toward provider-neutral doubles and compat-only imports.
- Risk: direct provider dependencies remain in `Cargo.toml` longer than ideal.
  - Mitigation: only keep the minimum required for migration, and isolate them away from core logic.

## Migration Plan

1. Introduce provider compatibility adapters.
2. Add or refine facade-oriented construction paths.
3. Mark legacy provider-oriented constructors deprecated.
4. Move provider-specific wiring out of kernel core modules where feasible.
5. Prune unused direct provider dependencies after the compat path is stable.

## Open Questions

- Which remaining kernel helper paths still need provider crates only for tests?
- Which direct dependencies can be removed immediately versus after compat isolation?
