## Context
Macaca must behave as a generic Agent OS runtime. Audit must be infrastructure-native, replayable, and independent from any single application. Current status is close, but runtime composition does not yet guarantee one shared evidence chain across production WASM execution and system replay service.

## Goals / Non-Goals
- Goals:
  - Guarantee a single source of truth for service-call audit events in host runtime scope.
  - Expose replay exclusively through generic system-service contracts.
  - Keep implementation pattern-based, extensible, and observable.
- Non-Goals:
  - Defining vertical business audit schemas.
  - Building tenant-level archival storage in this change.

## Decisions
- Decision: Use Dependency Injection for sink ownership at host composition boundary.
  - Rationale: Avoid hidden singleton coupling and allow future pluggable sinks.
- Decision: Keep `ServiceCallAuditSystemServiceProvider` as Facade over sink query methods.
  - Rationale: Stabilize command surface and isolate storage implementation changes.
- Decision: Keep `WasmHostImportBridge` as Bridge/PEP boundary and inject shared sink into its router.
  - Rationale: Preserve policy enforcement and audit emission at one routing chokepoint.
- Decision: Keep replay response as sanitized serializable view object, not internal event struct.
  - Rationale: Maintain protocol stability and backward-compatible output evolution.

## Risks / Trade-offs
- Risk: Partial wiring leaves dual sinks and inconsistent replay output.
  - Mitigation: Add explicit startup wiring tests and end-to-end replay assertions.
- Risk: Runtime startup ordering regression.
  - Mitigation: Register/start `system.service_audit` using the same lifecycle pattern as existing system services.
- Risk: Increased log volume.
  - Mitigation: Keep logs structured and bounded; do not log payload bodies.

## Migration Plan
1. Introduce shared sink at host startup composition.
2. Inject sink into replay provider and production WASM bridge construction.
3. Verify command contract remains stable.
4. Add/adjust integration tests for closed-loop replay path.
5. Run targeted runtime-host and integration test suites.

## Open Questions
- Should replay provider enforce pagination in this phase or next hardening phase?
- Should replay queries support time-window filters in the same command surface or a new command family?
