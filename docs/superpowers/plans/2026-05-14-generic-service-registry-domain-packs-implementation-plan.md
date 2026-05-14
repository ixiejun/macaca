# 2026-05-14 Generic Service Registry + Domain Packs Implementation Plan

## 1. Scope and Execution Policy

This plan implements the approved design:

- Mainline architecture: **A = Global Service Registry**
- Support architecture: **B = Domain Packs**

Hard constraints:

1. No app-specific business logic in Macaca OS core.
2. No hardcoded application-specific service allowlists in runtime code.
3. All enforcement must be data-driven from manifest/pack + platform policy.
4. Every critical execution node must emit traceable audit logs.
5. All newly added code must include detailed English comments.

## 2. Success Criteria

1. `service.call` always traverses unified router + policy enforcement.
2. `L2Wasm` apps never fall back to global agents/tools when app-scoped execution identity is missing.
3. Domain Pack declarations expand to effective service capabilities at admission/start.
4. Each service call emits lifecycle audit events with structured metadata.
5. Stock-style app capability declarations are fully manifest/pack-driven.

## 3. Workstreams and Phase Order

## Phase 0 — Safety Gate (Stop Cross-App Leakage)

### Tasks

1. Remove/disable app runtime fallback that maps empty app-scoped agent set to global agent set.
2. Introduce deterministic runtime state for unresolved app-scoped execution identity:
   - `runtime_unavailable` (or equivalent typed state).
3. Ensure UI/API shows controlled status rather than executing with global coordinator.

### Acceptance

1. L2 app with no executor identity cannot run coordinator/browser path.
2. Integration test proves no cross-app agent/tool leakage.

---

## Phase 1 — Service Control Plane Skeleton (A Core)

### Tasks

1. Add `ServiceContractRegistry` with versioned contract model.
2. Add `ServicePolicyEngine` with layered policy merge:
   - platform baseline
   - env/tenant override
   - app declaration override (bounded)
3. Add `ServiceRouter` as single entry for host-side `service.call`.
4. Add canonical request/response model with typed errors and decision metadata.

### Acceptance

1. All host-side service calls pass policy check before provider dispatch.
2. Denied calls return typed errors with policy reason code.
3. Unit tests cover contract resolution and policy precedence.

---

## Phase 2 — Provider Strategy and Resilience

### Tasks

1. Implement `ProviderSelector` strategies:
   - `latency_first`
   - `cost_first`
   - `trust_first`
   - `sticky`
2. Implement `ProviderAdapter` normalization boundary.
3. Add resilience controls:
   - retry budget
   - timeout budget
   - circuit breaker
4. Add provider-level health state telemetry.

### Acceptance

1. Router can switch providers without changing app code.
2. Retry/circuit behavior is deterministic and test-covered.
3. Provider failures produce normalized output and audit events.

---

## Phase 3 — Domain Packs (B Support Layer)

### Tasks

1. Implement `DomainPackCatalog` and pack schema (`pack_id`, version, services, defaults).
2. Add manifest fields:
   - `use_packs`
   - `required_services`
   - `optional_services`
   - `service_policy_overrides`
3. Implement effective capability expansion pipeline:
   - `expand(pack) ∪ declared services`
   - intersect with platform policy
4. Ship first data pack: `pack.finance.v1` (data-only definition).

### Acceptance

1. Finance app declares pack; OS derives effective capabilities automatically.
2. No runtime source file contains stock-app-specific service constants.
3. Admission/start pipeline logs effective capability set hash.

---

## Phase 4 — L2 WASM Service Path Binding

### Tasks

1. Bind guest `service.call` import to `ServiceRouter`.
2. Ensure WASM execution context supplies:
   - `app_id`
   - `session_id`
   - `trace_id`
   - `policy scope`
3. Gate execution on resolved capability set and runtime readiness.
4. Keep L2 admission/UI listing decoupled from execution readiness.

### Acceptance

1. WASM app can invoke declared services through router.
2. Undeclared service invocation is denied with policy reason.
3. No fallback to coordinator/browser path for L2 runtime.

---

## Phase 5 — Audit, Trace, and Ops Diagnostics

### Tasks

1. Emit mandatory events:
   - `service_call_requested`
   - `service_call_authorized` / `service_call_denied`
   - `service_call_dispatched`
   - `service_call_succeeded` / `service_call_failed`
2. Persist fields:
   - `trace_id`, `span_id`, `app_id`, `service_id`, `provider_id`
   - `decision`, `latency_ms`, `retry_count`, `circuit_state`
   - `input_hash`, `output_hash`
3. Add diagnostics endpoints/queries for call chain replay.

### Acceptance

1. A session can be replayed as evidence chain from request to result.
2. Audit records are complete for allow, deny, and failure paths.

## 4. File/Module Change Map (Planned)

Likely change areas (exact split to be refined in execution plan tasks):

1. `crates/application/macaca-app/`:
   - admission/runtime status semantics
   - manifest capability schema extension
2. `crates/runtime/macaca-runtime-host/`:
   - guest import binding to router
   - provider bridge and resilience integration
3. `crates/shells/macaca-web/`:
   - remove fallback path
   - status projection for non-executable L2 states
4. `crates/foundation/macaca-proto/`:
   - service contract/policy DTOs
   - audit event payload schemas

## 5. Test Plan

### Unit Tests

1. Contract version resolution and compatibility.
2. Policy precedence and deterministic deny reasons.
3. Provider selection strategy outputs.

### Integration Tests

1. L2 app visible but blocked from execution when executor unavailable.
2. L2 app with valid executor path can call declared services only.
3. Cross-app fallback path is absent.

### E2E/Session Replay Tests

1. `service.call` request→decision→dispatch→result chain is queryable.
2. Audit trail hashes are emitted and stable.

## 6. Rollout and Risk Controls

1. Ship behind feature flags:
   - `service_router_v1`
   - `domain_pack_resolution_v1`
   - `l2_wasm_router_binding_v1`
2. Enable progressively in staging, then production.
3. Keep rollback path by flag disable + preserved legacy pathway for non-L2 apps only.

## 7. Deliverables

1. Production-ready generic service registry/routing/policy modules.
2. Domain pack resolution pipeline with finance pack example.
3. L2 guest import binding to router (no app-specific hardcoding).
4. Full audit/trace event lifecycle with session replayability.
5. Updated docs/specs and migration notes for application developers.

