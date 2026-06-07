# 2026-05-14 Macaca Generic Service Registry + Domain Packs Design (A Main + B Support)

## 1. Context and Problem Statement

Macaca OS currently supports `service.call` in principle, but the runtime path is not fully industrialized for generic WASM applications:

1. The platform can expose a WASM app entry in UI without offering a generic execution path bound to app-declared capabilities.
2. When app-scoped agents are missing, Web runtime can fall back to global agents, causing cross-app execution leakage.
3. Service capability allowlists risk being implemented as code constants, which would hardcode application business logic into OS infrastructure.

The target is to establish a generic, auditable, and extensible service execution plane where:

- OS owns generic mechanisms (routing, policy enforcement, audit, resilience).
- Applications declare required capabilities and packs in metadata.
- No application-specific branch logic is added into system code.

## 2. Goals and Non-Goals

### 2.1 Goals

1. Introduce a generic service control plane for WASM and non-WASM apps.
2. Support app-declared capabilities through manifest metadata and versioned contracts.
3. Enforce policy centrally with complete trace/audit logs for every service call.
4. Prevent cross-app fallback execution and unintended global tool usage.
5. Keep architecture open for provider/plugin growth with minimal core code churn.

### 2.2 Non-Goals

1. Implementing domain-specific data logic (e.g., stock valuation logic) in Macaca core.
2. Hardcoding service IDs per application in runtime source files.
3. Replacing existing app/task orchestration architecture in one step.

## 3. Architecture Decision: A Main + B Support

### 3.1 A Main: Global Service Registry

Use a platform-level generic service registry and routing plane as the single control point for `service.call`.

### 3.2 B Support: Domain Packs

Use Domain Packs as developer-facing capability bundles layered on top of the global registry, not as a separate execution path.

### 3.3 Why This Combination

1. A guarantees long-term infra consistency, observability, and governance.
2. B improves app authoring speed and discoverability without coupling runtime semantics to app business.
3. The combination avoids duplicate routing stacks and preserves one enforcement boundary.

## 4. Design Patterns Applied

1. **Registry Pattern**: `ServiceContractRegistry` as authoritative source of service definitions and versions.
2. **Strategy Pattern**: `ProviderSelector` for route policy (`latency_first`, `cost_first`, `trust_first`, `sticky`).
3. **Adapter Pattern**: Provider adapters normalize heterogeneous provider APIs to a canonical contract.
4. **Policy Enforcement Point (PEP)** + **Policy Decision Point (PDP)**:
   - PEP: `ServiceRouter` gate before dispatch.
   - PDP: `ServicePolicyEngine` evaluating merged policy.
5. **Decorator/Observer Pattern**: Unified trace + audit emission around each service call lifecycle event.
6. **Factory Pattern**: Build runtime provider chains from registry metadata and pack declarations.

## 5. Core Components

### 5.1 ServiceContractRegistry (Core)

Responsibilities:

1. Store service contract descriptors:
   - `service_id`
   - schema versions
   - error taxonomy
   - SLA metadata (timeout ceiling, idempotency, cost tier)
2. Resolve effective contract version per app request.
3. Expose read APIs for admission, router, and diagnostics.

### 5.2 ServicePolicyEngine (Core)

Responsibilities:

1. Merge policy layers:
   - platform baseline policy
   - environment/tenant policy
   - app-declared policy
2. Evaluate `allow/deny` with structured reason codes.
3. Produce applied runtime limits:
   - timeout
   - retries
   - rate limits
   - budget limits

### 5.3 ServiceRouter (Core)

Responsibilities:

1. Accept canonical `service.call` requests from guest runtime.
2. Perform policy check via `ServicePolicyEngine`.
3. Select provider via `ProviderSelector`.
4. Dispatch through provider adapter.
5. Emit trace and audit events for all stages.

### 5.4 ProviderSelector + ProviderAdapter (Core)

Responsibilities:

1. Select provider deterministically according to strategy and constraints.
2. Enforce per-provider resilience:
   - circuit breaker
   - retry budget
   - fallback chain
3. Normalize errors and outputs to contract-compliant responses.

### 5.5 DomainPackCatalog (Support)

Responsibilities:

1. Maintain versioned pack definitions (e.g., `pack.finance.v1`).
2. Expand pack references to service declarations and default policy templates.
3. Keep pack evolution data-driven and independent from runtime code branches.

## 6. Manifest and Metadata Model

Applications declare capabilities as data, not code branches.

### 6.1 App Manifest Fields (Proposed)

1. `use_packs: [pack_id@version]`
2. `required_services: [service_id@version_range]`
3. `optional_services: [service_id@version_range]`
4. `service_policy_overrides`
5. `provider_constraints` (optional)
6. `data_governance`:
   - classification
   - redaction policy
   - retention policy

### 6.2 Effective Capability Resolution

`effective_services = expand(use_packs) ∪ required_services ∪ optional_services`

Then intersect with platform policy:

`allowed_services = effective_services ∩ platform_permitted_services`

## 7. Runtime Execution Flow

1. Guest calls `service.call(service_id, payload, metadata)`.
2. Host runtime builds `ServiceRequestContext` (`app_id`, `session_id`, `agent_id`, trace fields).
3. Router asks policy engine for authorization and applied limits.
4. Deny path returns typed policy denial response; allow path proceeds.
5. Provider selector chooses concrete provider.
6. Adapter executes and normalizes result.
7. Router emits lifecycle audit events and returns contract-compliant output.

## 8. Security, Traceability, and Auditability

Every call must produce immutable audit records containing:

1. `trace_id`, `span_id`, `correlation_id`
2. `app_id`, `session_id`, `service_id`, `provider_id`
3. `decision` (allow/deny + reason code)
4. `latency_ms`, `retry_count`, `circuit_state`
5. `input_hash`, `output_hash` (with optional redacted snapshots)

Mandatory lifecycle event types:

1. `service_call_requested`
2. `service_call_authorized` / `service_call_denied`
3. `service_call_dispatched`
4. `service_call_succeeded` / `service_call_failed`

## 9. Critical Behavior Fixes (Immediate)

### 9.1 Remove Cross-App Fallback

If an application has no app-scoped execution identity (e.g., no valid agent binding for current runtime mode), the system must not fall back to global agents/tools.

Expected behavior:

1. Return deterministic runtime-unavailable state for that app mode.
2. Preserve app isolation and prevent cross-app tool execution leakage.

### 9.2 L2 WASM Execution Mode Clarification

L2 WASM app admission and UI listing should be decoupled from execution readiness.

1. Admission/UI visibility is allowed once metadata is valid.
2. Execution starts only when a valid WASM executor path is bound to service router.
3. No interim global-agent fallback is permitted.

## 10. Migration and Rollout Plan (High-Level)

### Phase 1: Safety and Control-Plane Skeleton

1. Remove global fallback for missing app-scoped execution.
2. Introduce `ServiceContractRegistry` + `ServicePolicyEngine` + `ServiceRouter` seams.
3. Emit standardized audit events for request/decision/dispatch/result.

### Phase 2: Domain Pack Enablement

1. Add `DomainPackCatalog` and manifest expansion pipeline.
2. Create first pack (`pack.finance.v1`) as data-only definition.
3. Validate app-level declaration and policy merge behavior.

### Phase 3: Production Hardening

1. Provider health probes, circuit breaker, retry budgets.
2. Latency/cost/trust route strategies.
3. Enhanced policy diagnostics and governance dashboards.

### Phase 4: Ecosystem Expansion

1. Additional packs (office/media/research/etc.).
2. External provider plugin onboarding rules.
3. Compatibility/versioning governance for service contracts and packs.

## 11. Validation Strategy

### 11.1 Contract and Policy Tests

1. Contract version resolution tests.
2. Policy merge precedence tests.
3. Authorization deny/allow determinism tests.

### 11.2 Runtime Integration Tests

1. L2 app listed but execution blocked when executor unavailable.
2. No global fallback when app-scoped runtime identity is missing.
3. `service.call` success and failure paths produce required audit events.

### 11.3 Regression Tests

1. Existing non-WASM apps continue to run through current app runtime path.
2. No cross-app capability leakage under concurrent sessions.

## 12. Risks and Mitigations

1. **Risk**: Contract/version explosion.
   - **Mitigation**: strict semantic version policy + compatibility matrix.
2. **Risk**: Policy complexity and operator errors.
   - **Mitigation**: layered policy linting + explain-mode decision logs.
3. **Risk**: Provider instability.
   - **Mitigation**: circuit breaker + fallback chain + health telemetry.

## 13. Acceptance Criteria

1. Macaca core contains no app-specific service allowlist constants.
2. App capability declarations are data-driven from manifest/pack metadata.
3. Every `service.call` emits complete trace/audit lifecycle records.
4. L2 WASM apps never execute through global fallback agents/tools.
5. A sample finance app can run purely through declared services and policy enforcement.

