# S0 Serviceization Boundary Audit and Dependency Gate Plan

## Scope

Implement S0 from `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`: serviceization boundary audit and dependency gate. This phase does not migrate providers. It creates executable guardrails that make current and future boundary violations visible.

## Architecture Decision

Use an executable Specification + Visitor gate:

- Specification: layer rules and forbidden dependency edges.
- Visitor: traverse `cargo metadata` workspace dependency graph.
- Chain of Responsibility: evaluate each edge through forbidden rules, allowlist rules, and advisory rules.
- Strategy: keep room for advisory/warn/fail modes later.
- Memento: document current exceptions in an allowlist with replacement service path and migration phase.

This is more maintainable than ad-hoc grep checks and less risky than immediately failing all current violations.

## Required Inputs

- `macaca/docs/agent-os-microkernel-boundaries.md`
- `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`
- Current `macaca/Cargo.toml` workspace members
- `cargo metadata --no-deps --format-version 1`

## Proposed OpenSpec Change

Expected change id:

- `add-route-c-serviceization-dependency-gate`

Expected artifacts:

- `openspec/changes/add-route-c-serviceization-dependency-gate/proposal.md`
- `openspec/changes/add-route-c-serviceization-dependency-gate/design.md`
- `openspec/changes/add-route-c-serviceization-dependency-gate/tasks.md`
- `openspec/changes/add-route-c-serviceization-dependency-gate/specs/serviceization-dependency-gate/spec.md`

## Implementation Slices

### Slice S0.1: Dependency layer model

Files:

- New or test-local: `macaca/crates/macaca-integration-tests/tests/route_c_dependency_boundaries.rs`
- New doc: `macaca/docs/route-c-serviceization-allowlist.md`

Define crate layers:

- `proto`
- `kernel`
- `ipc-service-bus`
- `service-contract`
- `service-provider`
- `runtime-host`
- `application-framework`
- `presentation-shell`
- `optional-module`
- `integration-test`

Rules:

- Layer names must be stable strings.
- Unknown workspace crates must fail with an actionable message so new crates cannot bypass classification.

### Slice S0.2: Cargo metadata visitor

Implement a small test helper that runs:

```bash
cargo metadata --no-deps --format-version 1
```

Visitor behavior:

- Parse workspace packages.
- Build direct dependency edges between workspace crates.
- Classify each package by layer.
- Evaluate each direct edge against dependency specifications.
- Emit violations with `from`, `to`, `from_layer`, `to_layer`, rule id, and suggested replacement path.

Design constraints:

- No new external dependency unless existing serde_json is insufficient.
- Keep helper code below 500 lines; split if necessary.
- Comments must explain why each rule exists and how it maps to microkernel boundaries.

### Slice S0.3: Boundary specifications

Initial fail rules:

- `kernel-no-provider-deps`: `macaca-kernel` must not add new direct provider dependencies outside allowlist.
- `presentation-no-provider-construction-hub`: presentation shell crates must not add direct dependencies on provider implementation crates outside allowlist.
- `cli-no-web-internals`: `macaca-cli` must not depend on Web internals beyond documented Web server startup compatibility.
- `optional-not-base-required`: optional module crates must not be required by base OS crates outside allowlist.
- `service-provider-no-presentation`: service provider crates must not depend on presentation shell crates.

Initial advisory rules:

- SDK direct provider dependencies are migration debt and should route through SystemFacade/service clients in S3.
- Web direct provider dependencies are migration debt and should route through SystemFacade/service runtime in S12.

### Slice S0.4: Migration allowlist memento

Create `macaca/docs/route-c-serviceization-allowlist.md` with a table:

- Rule id
- From crate
- To crate
- Current reason
- Replacement service/facade path
- Target migration phase
- Expiry condition
- Owner/status

Allowlist principles:

- Allowlist is not approval of architecture.
- Every row must point to a future migration phase.
- New rows require OpenSpec update.
- Removing a row is preferred whenever service paths exist.

### Slice S0.5: Governance doc update

Update `macaca/docs/route-c-architecture-governance.md` to reference the executable dependency gate:

- State that dependency boundary violations must be represented either as failing tests or documented allowlist rows.
- State that new provider dependencies in kernel/presentation are forbidden unless an OpenSpec explicitly updates the allowlist.

### Slice S0.6: Verification

Run:

```bash
cargo test -p macaca-integration-tests route_c_dependency_boundaries
cargo metadata --no-deps --format-version 1
openspec validate add-route-c-serviceization-dependency-gate --strict
```

If implementation changes no frontend code, frontend lint/typecheck is not required.

## Dependency Gate Rule Shape

Recommended Rust model:

```rust
struct CrateLayerSpec {
    crate_name: &'static str,
    layer: &'static str,
}

struct BoundaryRule {
    id: &'static str,
    from_layer: &'static str,
    forbidden_to_layers: &'static [&'static str],
    severity: RuleSeverity,
    rationale: &'static str,
    replacement: &'static str,
}

struct AllowlistEntry {
    rule_id: &'static str,
    from_crate: &'static str,
    to_crate: &'static str,
    target_phase: &'static str,
    replacement: &'static str,
}
```

The exact implementation can vary, but it should preserve:

- typed rule ids,
- actionable diagnostics,
- a documented allowlist,
- no application/provider hardcoding beyond crate names required by dependency governance.

## Trace and Audit Considerations

S0 is mostly test/doc infrastructure, not runtime service execution. Runtime trace is not required for the test itself. However:

- Test diagnostics must be audit-friendly and deterministic.
- Future runtime gates introduced after S1 must emit structured logs when service calls are denied.
- Any helper code added outside tests should use structured `tracing` logs at key execution nodes.

## Non-Goals

- Do not remove kernel/provider dependencies in S0.
- Do not rewrite Web or CLI in S0.
- Do not implement ServiceRuntime v1 in S0.
- Do not move Task/LLM/Memory/Driver/Skill/MCP providers yet.
- Do not introduce a new policy language or external dependency tool.

## Risks and Mitigations

- Risk: Current dependency graph produces many violations.
  - Mitigation: document current edges in allowlist with migration phase and replacement path.
- Risk: Allowlist becomes permanent.
  - Mitigation: require expiry condition and target phase per row.
- Risk: Gate misses runtime construction hardcoding.
  - Mitigation: S0 explicitly gates Cargo edges first; later phases can add symbol/code scans.
- Risk: New crate is unclassified.
  - Mitigation: fail unknown workspace crates with a clear classification message.

## Completion Criteria

- OpenSpec proposal/design/tasks/spec exists and validates.
- Boundary test exists and runs.
- Allowlist doc exists and explains current exceptions.
- Architecture governance doc references the gate.
- No existing Route C baseline behavior is changed.
- No unrelated dirty files are committed.
