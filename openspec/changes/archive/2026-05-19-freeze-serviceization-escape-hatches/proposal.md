# Change: Freeze serviceization escape hatches

## Why
Macaca OS still carries documented serviceization migration debt in kernel,
Web, CLI, application lifecycle, and role-routing paths. The next refactor
tracks need executable guardrails first so new direct paths cannot appear while
existing callers move behind service clients and facades.

## What Changes
- Add a static production-source gate that rejects new direct AppRuntime start
  calls, Web runtime/provider field reads, and hardcoded agent role names outside
  approved migration, fixture, or test surfaces.
- Enrich Route C dependency allowlist entries with owner track, caller evidence,
  replacement boundary, expiry phase, and validation command.
- Move production `Kernel` construction to a provider-neutral agent execution
  port so the kernel no longer stores concrete provider compatibility bundles.
- Keep existing behavior intact; this change freezes new violations and makes
  current migration debt auditable before ownership moves.

## Impact
- Affected specs: serviceization-escape-hatches
- Affected code:
  - `macaca/crates/tests/macaca-integration-tests/tests/serviceization_escape_hatches.rs`
  - `macaca/crates/tests/macaca-integration-tests/tests/route_c_dependency_boundaries/allowlist.rs`
  - `macaca/crates/tests/macaca-integration-tests/tests/route_c_dependency_boundaries/gate.rs`
  - `macaca/crates/application/macaca-agent/src/execution.rs`
  - `macaca/crates/kernel/macaca-kernel/src/provider_compat.rs`
  - `macaca/crates/kernel/macaca-kernel/src/kernel.rs`
  - `macaca/crates/kernel/macaca-kernel/src/kernel_builder.rs`
  - `macaca/crates/kernel/macaca-kernel/Cargo.toml`
