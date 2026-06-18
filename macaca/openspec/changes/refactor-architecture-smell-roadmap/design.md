# Design: Refactor Architecture Smell Roadmap

## Context

Macaca is a microkernel Agent OS. The stable ownership model is explicit:

- Kernel owns invariants and primitive registries only.
- Services own replaceable capabilities such as task planning/decomposition, execution control, review, recovery, LLM, memory, context, driver, skill, MCP, gateway, application lifecycle, payments, Web3, and EVM.
- Runtime-host owns host-side service providers, decorators, provider factories, bootstrapping, and diagnostics.
- SDK/SystemFacade owns provider-neutral clients and Null Object behavior.
- Web/CLI/frontend/gateway shells own input/output adaptation only.

The smell report shows the system is already close to that model, but residual smell appears where semantic behavior remains in adapters or where composition modules are too dense to audit confidently.

## Goals

- Make each smell-roadmap item executable, traceable, and testable.
- Preserve existing public behavior for `/api/chat/v2`, task boards, application execution, trace replay, Web UI, CLI, and optional-module unavailable states.
- Strengthen architecture gates so future regressions fail early.
- Improve runtime-host and proto maintainability without inventing application-specific branches.
- Use design patterns already accepted by Macaca governance.

## Non-Goals

- No application-specific logic.
- No provider-name, model-name, driver-name, gateway-name, chain-name, payment-name, workflow-name, or app-name branches in OS-layer code.
- No kernel expansion.
- No shell-owned task/planning semantics.
- No hard failure on trend-only smell reports in the first CI lane.

## Design Patterns

- **Command:** Task decomposition, architecture-smell audits, and provider-family extraction readiness are represented as typed commands/results where they cross boundaries.
- **Strategy:** Task decomposition behavior is selected by a service-owned strategy, not by shell keyword branches.
- **Facade:** Web/CLI interact through focused SDK clients or `SystemFacade`.
- **Adapter/Bridge:** Shell routes adapt HTTP/SSE/CLI input to service/facade commands and map results back to existing response shapes.
- **Decorator:** Trace, policy, resource, entitlement, and audit checks stay at service runtime boundaries.
- **State:** Static registries and long-lived process-local resources document lifecycle states and reset/test-isolation behavior.
- **Observer:** Architecture-smell trend reports and boundary events are emitted as auditable diagnostics.
- **Specification:** Dependency, file-size, shell semantics, DTO density, and smell trend rules are executable gates.
- **Abstract Factory:** Runtime-host remains the approved composition root for provider factories until extraction-readiness criteria justify new service crates.

## Decisions

### Decision 1: Task decomposition semantics move to service ownership

`macaca-web` must stop classifying tasks through shell keyword chains. The canonical target is a task/autonomy service command with a provider-neutral request/result DTO. The service owns a replaceable `TaskDecompositionStrategy`; Web only sends input and renders results.

**Alternatives considered:**

- Keep the shell heuristic but wrap it in a helper. Rejected because ownership remains in the shell.
- Move heuristics into the SDK. Rejected because SDK must be a facade/client boundary, not a semantic owner.

### Decision 2: Runtime-host remains the composition root, but provider modules split internally first

`macaca-runtime-host` is large, but extracting new crates too early can create more boundary churn. The first refactor splits near-limit provider files into descriptor, command, handler, state, adapter, and fixture modules. Mature service families must pass extraction-readiness before crate extraction.

### Decision 3: Advisory smell gates complement hard architecture gates

Hard gates still fail on boundary violations, hardcoded OS-layer names, direct provider calls, and files over 500 lines. New smell trend gates initially report advisory findings for 450-line headroom, complexity trends, DTO density, and coupling hotspots. This avoids blocking necessary work while making debt visible.

### Decision 4: Static state is allowed only with explicit lifecycle documentation

Process-local state such as `OnceLock` registries, session locks, or SDK static driver handles must document owner, initialization, reset/test-isolation behavior, restart semantics, and why composition-root state is not currently viable. New hidden global state without those comments is rejected.

### Decision 5: Text routing is limited to ingestion and declarative matching

Remaining `contains`-style routing must be replaced with typed descriptors or declarative mapping records where possible. When string matching is still required at ingestion boundaries, the code must log sanitized reason codes and point to a typed fallback path.

## Migration Plan

1. Establish gates and split tests first so regressions become visible before broad refactors.
2. Move task decomposition through service/facade paths while preserving existing response shapes.
3. Split near-limit modules in small ownership-preserving slices.
4. Add local indexes to selected request/event hot paths.
5. Split proto modules and provider modules only after impact analysis and targeted tests.
6. Enable architecture-smell reporting as non-failing trend output.

## Trace, Audit, and Logging

Every new service command or gate diagnostic must record sanitized operation names, service or rule identifiers, trace id when available, session/task/app scope when available, status, and reason code. Logs must not contain raw secrets, prompts, manifests, WASM bytes, package bytes, credentials, private keys, raw signatures, raw provider payloads, or unbounded user input.

## Risks and Mitigations

- **Risk:** Moving decomposition changes user-visible task board behavior.
  - **Mitigation:** Preserve existing DTO/result shapes and add regression tests for existing Web flows.
- **Risk:** Splitting runtime-host modules causes accidental re-export or dependency drift.
  - **Mitigation:** Run dependency gates and targeted crate checks after each slice.
- **Risk:** Advisory smell CI creates noise.
  - **Mitigation:** Make trend findings non-failing and deterministic, with suppressions requiring explicit reason codes.
- **Risk:** Proto splitting breaks serde compatibility.
  - **Mitigation:** Preserve public type names/re-exports and add serde roundtrip tests.

## Verification Strategy

- `openspec validate refactor-architecture-smell-roadmap --strict`
- Targeted crate checks for touched crates.
- Boundary gates:
  - `serviceization_escape_hatches`
  - `protocol_service_dependency_boundaries`
  - `os_layer_file_size_gate`
  - shell dependency purity and shell semantic ownership gates
- Regression tests for `/api/chat/v2`, task board isolation, trace replay, application execution, and optional unavailable states where touched.
- `cargo check --workspace` when shared contracts or module boundaries change.
