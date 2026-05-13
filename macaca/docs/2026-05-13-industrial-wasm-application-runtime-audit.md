# Industrial WASM Application Runtime Audit

Audit date: 2026-05-13

## Basis

This audit checks the industrial WASM Application Runtime implementation against
`docs/superpowers/plans/2026-05-12-industrial-wasm-application-runtime-brainstorm.md`
and the related OpenSpec changes for Route C WASM runtime work.  It also checks
the current runtime, proto, SDK facade, certification, WIT, and governance files
that implement those changes.

The brainstorm selected Option D, the layered dual execution-plane architecture:
developer SDK and WIT contracts feed package admission and the application
control plane, while `runtime-host` owns a replaceable provider boundary that
can run in-process today and be replaced by a hardened out-of-process provider
later.  The audit therefore separates completed provider-neutral control-plane
work from remaining production execution-engine hardening.

## Executive Summary

The industrial runtime foundation is substantially implemented.  The codebase
now has provider-neutral WASM DTOs, admission and ABI negotiation, an
in-process default provider, sandbox policy DTOs and runtime guards, host-import
service portal plumbing, lifecycle state/checkpoint metadata, guest harness
fixtures, certification fixtures, and a hardened-provider data contract.

The implementation is contract-ready but not yet a complete industrial
third-party execution fabric.  The default provider executes a deliberately
small core-WASM surface through a private adapter for nullary exports.  It does
not yet provide full WASM Component Model canonical ABI execution, real WIT
binding lowering/lifting, a production Wasmtime/WasmEdge adapter, or a hardened
out-of-process daemon.  Those gaps are architectural follow-ups rather than
missing pieces in the completed OpenSpec increments.

## OpenSpec Completion

All related OpenSpec task files are checked complete:

| Change | Status | Implementation evidence |
| --- | --- | --- |
| `add-wasm-component-application-abi-skeleton` | Complete | Application ABI skeleton and WASM component profile in proto/app certification surfaces. |
| `add-wasm-runtime-provider-contract` | Complete | Provider-neutral descriptors, diagnostics, provider/session traits, registry, unavailable Null Object provider. |
| `add-wasm-package-admission-abi-negotiation` | Complete | Artifact descriptors, ABI requirements, import/export requirements, admission reports, sanitized diagnostics. |
| `add-wasm-default-in-process-runtime-provider` | Complete | Default provider, private engine adapter, compile cache, artifact loading, sanitized execution results. |
| `add-wasm-sandbox-resource-governance` | Complete | Resource policy DTOs, WASI deny-by-default policy, payload/timeout/concurrency guards, audit reports. |
| `add-wasm-host-import-service-portal` | Complete | Host import bridge, service runtime dispatch, trace/capability/payload checks, host import audit reports. |
| `add-wasm-lifecycle-state-checkpoint` | Complete | State-machine DTOs, lifecycle transitions, metadata checkpoint/restore/upgrade/rollback reports. |
| `add-wasm-guest-sdk-toolchain-test-harness` | Complete | SDK scaffold, package fixture, runtime guest harness, mock host imports, WIT label checks. |
| `add-wasm-certification-fixtures-hardened-provider-contract` | Complete | Certification profiles, conformance/negative fixtures, sanitized reports, hardened-provider envelope mock. |

## Capability Audit

| Capability | Current status | Evidence | Remaining industrial gap |
| --- | --- | --- | --- |
| ABI and WIT contract | Implemented baseline | WIT files under `macaca/application-wit`, WASM ABI DTOs in `macaca-proto`, OpenSpec strict tasks complete. | Needs long-term semantic version/deprecation policy and broader multi-version compatibility tests. |
| Package admission and ABI negotiation | Implemented | `macaca-app` WASM admission spec validates artifact presence, digest, ABI, import permissions, runtime capabilities, and sanitized reports. | Needs supply-chain signature, origin, provenance, and reproducible build metadata before Store-scale trust. |
| Provider abstraction | Implemented | `WasmApplicationRuntimeProvider`, `WasmExecutionSession`, registry, unavailable provider, and provider-neutral proto DTOs keep engine details private. | Needs additional real provider implementations and integration wiring for provider selection in deployment profiles. |
| Default in-process execution | Implemented with a narrow execution surface | `DefaultInProcessWasmRuntimeProvider`, private `engine_adapter`, compile cache, artifact loader, and nullary export invocation tests. | Does not yet execute full WASM Component Model/WIT imports and exports; production engine adapter is still future work. |
| Sandbox and resource governance | Implemented as policy and guard layer | Resource envelope, WASI policy, quota keys, active policy merge, timeout/payload/concurrency checks, resource audit reports. | Needs engine-enforced fuel/epoch interruption, memory isolation, wall-clock interruption, and process-level containment from a production engine/daemon. |
| WASI and raw host resources | Implemented deny-by-default model | WASI policy defaults deny raw env/filesystem/network; certification includes raw env/filesystem/network negative fixtures. | Needs capability-scoped virtual storage/runtime host implementation beyond policy declaration. |
| Host imports and service portal | Implemented | Host import bridge converts imports into service-runtime commands with trace, capability, payload, and service status auditing. | Needs broader import catalog coverage and real guest binding integration once full Component Model execution lands. |
| Lifecycle and checkpoint | Implemented as provider-neutral metadata lifecycle | State machine, trace-required transitions, audit events, metadata mementos, restore/upgrade/rollback compatibility checks. | Checkpoints are intentionally sanitized metadata only; no guest memory snapshot, suspension, or drain semantics yet. |
| Observability and audit | Implemented for key control points | Sanitized diagnostics, trace-required requests/commands, admission reports, resource audit, host import audit, lifecycle audit, certification reports. | Needs end-to-end production telemetry sinks, metrics aggregation, and operator dashboards. |
| Guest SDK and local harness | Implemented as scaffold/harness | SDK WASM scaffold, fixture builders, runtime harness, mock host outcomes, WIT label report. | Needs generated multi-language SDKs, binding generation workflow, packaging commands, and developer CLI integration. |
| Certification and hardened-provider contract | Implemented as conformance harness and mock contract | Runtime certification profiles, conformance and security-negative matrices, sanitized reports, hardened envelope/response mock. | Needs real hardened out-of-process provider daemon, IPC protocol, cancellation/backpressure enforcement, and CI/Store gating integration. |

## Architecture Boundary Review

The implementation follows the brainstorm's main architectural boundaries:

- Runtime engine details do not leak into public ABI.  The public contract uses
  provider-neutral DTOs and traits; the in-process parser/interpreter is private
  to `runtime-host`.
- The kernel does not own WASM runtime execution.  Runtime-host owns the
  replaceable provider boundary and service bridge.
- The SDK facade remains provider-neutral and produces package/application
  descriptors rather than engine-specific handles.
- Host imports flow through Command-style service portal dispatch instead of
  direct provider-specific service calls.
- Trace and sanitized diagnostics are first-class across admission, execution,
  host import, resource governance, lifecycle, and certification paths.
- Resource and host access policies are deny-by-default and do not expose raw
  filesystem, environment, network, or raw command payloads in audit reports.

The design patterns called out by the brainstorm are present:

- Bridge: host import bridge and provider/session boundary separate application
  ABI from service runtime and concrete execution strategy.
- Strategy and Abstract Factory: `WasmApplicationRuntimeProvider` creates
  provider-specific execution sessions behind a stable contract.
- Null Object: unavailable provider provides deterministic fail-closed behavior.
- Specification: admission and certification evaluate fixtures/context through
  explicit rule objects and reason codes.
- State: lifecycle transitions are centralized in a state machine.
- Memento: checkpoints, certification reports, cache reports, and hardened
  envelopes carry sanitized serializable state.

## Risk Assessment

| Risk | Current severity | Audit finding |
| --- | --- | --- |
| Public ABI leaks concrete engine details | Low | Current contract is provider-neutral; private adapter stays inside runtime-host. |
| WASI/resource exposure too broad | Medium | Policy defaults are deny-by-default, but production enforcement depends on future engine/daemon integration. |
| Host imports bypass service runtime | Low | Current bridge routes through service runtime semantics with capability and trace checks. |
| Lifecycle incomplete for long-running apps | Medium | State/checkpoint/restore/upgrade/rollback metadata exists; real suspension/drain/runtime snapshots are not implemented. |
| Logs leak raw payload or secrets | Low | Sanitization is repeatedly tested across unavailable runtime, admission, certification, and hardened reports. |
| SDK/runtime drift | Medium | Harness and fixtures reduce drift, but real bindgen/toolchain workflows are not yet implemented. |
| Store-scale certification readiness | Medium | Conformance and negative fixtures exist; supply-chain provenance and CI/Store gate integration remain. |
| Hardened execution isolation | High | Contract and mock adapter exist, but no out-of-process daemon or production engine integration exists yet. |

## Remaining Work

Recommended follow-up OpenSpec changes:

1. `add-wasm-component-model-execution-provider`: integrate a production engine
   such as Wasmtime or WasmEdge behind the existing provider traits, with WIT
   canonical ABI import/export execution and engine-enforced fuel/epoch limits.
2. `add-wasm-hardened-out-of-process-provider`: implement daemon/IPC execution,
   cancellation, backpressure, health checks, process isolation, and crash
   recovery using the existing hardened-provider envelope contract.
3. `add-wasm-artifact-supply-chain-verification`: add signatures, build
   provenance, origin, reproducible artifact metadata, and Store admission gates.
4. `add-wasm-guest-sdk-bindgen-toolchain`: add generated bindings, real
   multi-language SDK workflows, packaging commands, and CLI/local developer
   loops.
5. `add-wasm-production-observability-sinks`: connect runtime audits, resource
   metrics, traps, host-import decisions, and lifecycle events to operator-grade
   telemetry and dashboards.

## Conclusion

The Route C WASM Application Runtime work has completed the planned control
plane, provider contract, governance, lifecycle, SDK fixture, and certification
increments.  It is a coherent and extensible runtime fabric foundation for
Macaca's Agent OS model.

The system should be described as industrial-contract-ready, not yet fully
industrial-execution-complete.  The remaining work is concentrated in
production-grade Component Model execution, hardened out-of-process isolation,
supply-chain trust, real SDK/bindgen workflows, and production telemetry
integration.
