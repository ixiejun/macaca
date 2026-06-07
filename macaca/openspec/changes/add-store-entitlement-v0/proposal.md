# Change: Add Store / Entitlement Runtime v0

## Why

Route C Phase 08 requires a provider-neutral commerce and entitlement baseline so paid packages, paid skills, paid plugins, and paid capability calls can be allowed/denied through auditable OS policy rather than ad-hoc checks in shell or app code.

Without a unified Store/Entitlement v0 contract, package install/start/call decisions will fragment across crates, leading to entitlement bypass paths, missing metering traces, and tight coupling to specific payment/store providers. This would violate microkernel/service boundaries and block later A2A payment and optional Web3/EVM phases.

## What Changes

- Add provider-neutral commerce contracts in `macaca-proto` for license, entitlement, subscription, revocation, metering, encrypted package metadata, and structured commerce errors.
- Add entitlement persistence contract in `macaca-persist` with deterministic state precedence (`revoked` overrides `valid`) and traceable read/write lifecycle.
- Add runtime-host Store/Entitlement facade in `macaca-runtime-host` to enforce package/runtime guard checks for install/start/call decisions.
- Add encrypted skill loading hooks in `macaca-skill` that require entitlement authorization before decrypt/activate and return structured errors for deny/failure paths.
- Add commercial package runtime guard integration in `macaca-app` for paid app/package allow/deny decisions while preserving existing free/open-source flows.
- Add metering event emission for paid capability calls through existing trace/event infrastructure with structured fields for app/package/developer/session/capability.
- Add detailed English comments and structured logs at key entitlement and metering decision points.

## Impact

- Affected specs: `store-entitlement-v0`
- Affected crates: `macaca-proto`, `macaca-persist`, `macaca-runtime-host`, `macaca-skill`, `macaca-app`, optionally `macaca-web` for additive inspection surfaces
- Affected code areas: commerce contracts, entitlement store contract, runtime entitlement guard facade, encrypted skill hook, metering event emission points, and targeted tests
- Regression matrix references: `RC-APP-001`, `RC-SKILL-001`, `RC-TRACE-001`

## Governance Alignment

- Follows `macaca/docs/agent-os-microkernel-boundaries.md`: Store/Entitlement remains a system service; kernel keeps invariants only.
- Follows `macaca/docs/route-c-regression-matrix.md`: Phase 08 explicitly preserves app load, skill/MCP smoke path, and trace integrity.
- Follows `macaca/docs/route-c-phase-template.md`: includes brainstorm-driven proposal/design/tasks/spec, additive-first slices, targeted tests, integration smoke, and detect_changes gates.
- Follows `macaca/docs/route-c-architecture-governance.md`: uses Facade/Strategy/Specification/Chain of Responsibility/Decorator patterns; requires trace and policy for entitlement and paid capability decisions.

## Non-Goals

- Do not integrate real payment providers, chain settlement, or marketplace billing in Phase 08.
- Do not require Web3/EVM modules for entitlement decisions.
- Do not block free/open-source local development packages.
- Do not place Store/Entitlement business logic into kernel.
- Do not hardcode app names, workflow names, provider names, gateway names, driver names, model names, chain names, or business-specific routing.
