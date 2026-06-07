# S0 Serviceization Boundary Audit Brainstorm

## Problem

Route C has already established many contracts and skeletons, but the workspace can still drift back into macro-kernel coupling. The current architecture plan explicitly notes that `macaca-kernel`, `macaca-web`, and `macaca-sdk` still have direct dependencies or construction paths that should eventually move behind services/facades.

S0 must stop that drift before deeper serviceization begins. The goal is not to remove every violation immediately. The goal is to make boundary violations visible, executable, traceable, and explicitly allowed only as temporary migration debt.

## Constraints

- Must follow `macaca/docs/agent-os-microkernel-boundaries.md`.
- Kernel owns invariants only.
- Services own replaceable capabilities.
- Plugins and optional modules must not bypass service registry.
- Web/CLI/frontend remain presentation shells.
- Existing YAML applications, `/api/chat/v2`, trace, task board, resume, driver, skill/MCP paths must not regress.
- Any later code must include detailed English comments and structured logs at key execution nodes.

## Design Pattern Candidates

### Specification

Use declarative dependency rules to state which layer may depend on which other layer. This is the primary pattern because the gate is a policy checker, not a business workflow.

### Visitor

Traverse `cargo metadata` dependency graph and classify each dependency edge. The visitor can emit violations, allowlist matches, and advisory warnings without hardcoding one-off checks throughout the test.

### Chain of Responsibility

Evaluate each dependency edge through ordered rule sets:

- forbidden edge rules,
- optional-module rules,
- presentation-shell rules,
- allowlist rules,
- advisory rules.

This keeps rule evolution modular and avoids a giant conditional block.

### Strategy

Allow strictness levels later:

- advisory mode for local exploration,
- warning mode for migration planning,
- fail-fast mode for CI.

S0 should start with fail-on-new-forbidden-edge while allowing documented current violations.

### Memento

Persist the current migration allowlist as a documented snapshot. Every exception carries owner, reason, replacement service path, and target migration phase.

## Options

### Option A: Documentation-only audit

- Pros: Fast, low risk.
- Cons: No executable gate, drift continues.
- Verdict: Rejected. S0 explicitly requires CI/test gate.

### Option B: Hard fail every current violation

- Pros: Strong architectural enforcement.
- Cons: Too disruptive because current plan already acknowledges existing direct dependencies.
- Verdict: Rejected for S0. This belongs to later migration phases after service paths exist.

### Option C: Executable gate with migration allowlist

- Pros: Stops new violations while documenting existing debt.
- Cons: Requires careful allowlist design to avoid normalizing debt.
- Verdict: Recommended.

### Option D: Use external dependency-policy tool

- Pros: Feature-rich.
- Cons: Adds dependency and policy syntax before the team has stabilized boundaries.
- Verdict: Defer. S0 can use `cargo metadata` and Rust integration tests first.

## Recommended Plan

Implement S0 as an executable dependency boundary gate:

- Define crate layer map in a test or helper.
- Use `cargo metadata --no-deps --format-version 1` to inspect workspace package dependencies.
- Encode forbidden dependency edges according to microkernel boundaries.
- Add `macaca/docs/route-c-serviceization-allowlist.md` as the migration memento.
- Fail new forbidden edges unless they are explicitly listed with migration phase and replacement service path.
- Add governance documentation explaining the boundary rule and how to add/remove allowlist entries.

## Initial Boundary Rules

- `macaca-kernel` must not add new direct provider dependencies.
- `macaca-web` must not become a provider construction hub.
- `macaca-cli` must not depend on Web internals except the current Web server startup boundary.
- Optional modules must not become base OS mandatory dependencies.
- Presentation shell crates must consume SDK/system facades for migrated capabilities.
- Service provider crates may depend on proto and their own implementation dependencies, but not on presentation shell crates.

## Risks

- False positives from current transitional dependencies.
- False negatives if rules only inspect Cargo edges and miss runtime construction paths.
- Allowlist may become permanent architecture debt.
- CI can become noisy if every migration churns the allowlist.

## Mitigations

- Start with crate dependency edges and a small allowlist.
- Require every allowlist row to include target phase, replacement path, and expiry condition.
- Add comments explaining rule intent and evaluation flow.
- Keep rule file small and split if it approaches 500 lines.
- Add future TODO for symbol-level construction scans after S1/S2 service runtime exists.

## Rollback

If the gate blocks unrelated work, temporarily downgrade one rule to advisory only through a documented allowlist row. Do not delete the gate; S0 is specifically meant to preserve architecture direction.
