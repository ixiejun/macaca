# Design: Route C Serviceization Dependency Gate

## Context

`2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md` defines the next route after Route C skeleton phases: serviceize and modularize non-kernel capabilities. S0 is the safety gate before real migration. It should prevent new forbidden dependency edges while acknowledging current transitional edges through an explicit migration allowlist.

The boundary source of truth is `macaca/docs/agent-os-microkernel-boundaries.md`. Kernel may keep only system invariants. Replaceable capabilities must become system services or optional modules. Web/CLI/frontend must stay shells. S0 translates those architecture rules into executable dependency checks.

## Goals

- Add an executable workspace dependency gate.
- Classify workspace crates into stable architecture layers.
- Detect direct workspace dependency edges that violate Route C boundaries.
- Allow known temporary violations only when documented in an allowlist.
- Fail unknown workspace crates so new crates cannot bypass classification.
- Produce actionable diagnostics with rule id, source crate, target crate, layers, rationale, and replacement path.
- Keep S0 additive, no-provider-migration, and compatible with existing behavior.

## Non-Goals

- No provider dependency removal.
- No service runtime implementation.
- No runtime service call policy enforcement.
- No frontend change.
- No external dependency governance tool.
- No symbol-level construction scanning in S0.

## Design Patterns

### Specification

Boundary rules are specifications over dependency edges. Each rule describes from-layer/from-crate constraints, forbidden target layers/crates, severity, rationale, and replacement path.

### Visitor

The test visits the `cargo metadata` workspace package graph, builds direct workspace edges, and evaluates each edge against rule specifications.

### Chain of Responsibility

Each edge flows through ordered evaluators:

- crate classification,
- forbidden rule matching,
- allowlist matching,
- advisory reporting,
- violation emission.

This avoids one large conditional block and makes future rules easier to add.

### Strategy

The design leaves room for future modes such as advisory, warn, or fail-fast. S0 can begin with fail-on-new-forbidden-edge plus allowlist.

### Memento

`route-c-serviceization-allowlist.md` captures the current migration-debt snapshot. It is not approval of the architecture; it records why a violation exists, what will replace it, and when it should expire.

## Layer Model

Initial layers:

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

Every workspace crate must be classified. Unknown crates fail with an actionable message.

## Initial Rules

### `kernel-no-provider-deps`

`macaca-kernel` must not add new direct dependencies on provider implementation crates outside the allowlist.

### `presentation-no-provider-construction-hub`

Presentation shell crates must not add direct dependencies on provider implementation crates outside the allowlist.

### `cli-no-web-internals`

`macaca-cli` must not become dependent on Web internals beyond documented Web server startup compatibility.

### `optional-not-base-required`

Optional module crates must not become required base OS dependencies outside the allowlist.

### `service-provider-no-presentation`

Service provider crates must not depend on presentation shell crates.

## Allowlist Format

The allowlist document must contain rows with:

- rule id,
- from crate,
- to crate,
- current reason,
- replacement service/facade path,
- target migration phase,
- expiry condition,
- owner/status.

Implementation may embed a test-local allowlist table for deterministic checking, but the doc remains the human-readable memento. The embedded rows and document should match.

## Diagnostics

Violation messages should include:

- rule id,
- source crate,
- target crate,
- source layer,
- target layer,
- rationale,
- suggested replacement path,
- note that new exceptions require OpenSpec and allowlist update.

## Trace and Audit

S0 is test/doc infrastructure, not runtime service execution. Runtime trace emission is not required. Auditability is provided by deterministic diagnostics, documented allowlist rows, and governance docs. If future dependency gates move into runtime or CI services, they should emit structured logs at rule evaluation and denial nodes.

## Risks and Mitigations

- Risk: existing workspace edges create many violations.
  - Mitigation: document existing violations as migration allowlist rows and fail only unallowlisted violations.
- Risk: allowlist becomes permanent.
  - Mitigation: require target migration phase and expiry condition.
- Risk: false confidence because Cargo edges do not catch runtime construction.
  - Mitigation: explicitly document S0 scope and add future S1/S2 symbol-level scans.
- Risk: gate blocks new crate scaffolding.
  - Mitigation: unknown crate diagnostic tells maintainers to classify the crate and update OpenSpec.

## Verification Plan

- `openspec validate add-route-c-serviceization-dependency-gate --strict`
- `cargo metadata --no-deps --format-version 1`
- `cargo test -p macaca-integration-tests route_c_dependency_boundaries`
- `cargo test -p macaca-integration-tests --test route_c_baseline`
- `cargo check --workspace`
- `npx gitnexus detect-changes --repo agent`
