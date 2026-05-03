# Change: Refactor macaca-agent primitive boundaries

## Why
`macaca-agent` has already introduced service facade/no-op fallbacks, BasicAgent builder, lifecycle policy, and capability composite primitives. These primitives are still co-located with concrete agent files and lack a small set of canonical construction and inspection APIs that upper crates can safely depend on.

## What Changes
- Add module boundaries for services, capability, and lifecycle primitives.
- Add `AgentServicesBuilder` as the canonical additive constructor for service bundles while preserving existing fields and behavior.
- Move capability composite types behind an agent-level capability module and add read-only inspection/conversion helpers.
- Add lifecycle transition value/preflight helpers without changing current transition semantics.
- Mark legacy direct construction APIs as deprecated but keep them available for migration discovery.
- Keep all existing public re-exports and runtime behavior compatible.

## Impact
- Affected specs: macaca-agent-core
- Affected code: `macaca/crates/macaca-agent/src/**`
- Follow-up consumers: `macaca-framework`, `macaca-sdk`, `macaca-web`, `macaca-kernel` in separate changes only.
