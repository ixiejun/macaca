# Industrial WASM Runtime Industrialization Design

## Purpose

This design turns the WASM Application Runtime audit into a complete
industrialization program.  The existing Route C WASM work already provides the
provider-neutral control plane: ABI DTOs, package admission, default in-process
execution, sandbox policy, host import bridge, lifecycle/checkpoint metadata,
guest harness fixtures, and certification reports.  The remaining work is to
upgrade the execution, isolation, supply-chain, SDK, and observability layers
without breaking those contracts.

## Current Baseline

The current implementation is industrial-contract-ready:

- `macaca-proto` owns provider-neutral WASM DTOs, resource policy, lifecycle
  state, host import commands, diagnostics, and package admission metadata.
- `macaca-runtime-host` owns `WasmApplicationRuntimeProvider`,
  `WasmExecutionSession`, the unavailable Null Object provider, the default
  in-process provider, the host import bridge, lifecycle support, guest harness,
  and certification harness.
- `macaca-app` owns package certification and WASM admission checks.
- `macaca-sdk` owns provider-neutral scaffolds and fixtures.

The current default provider intentionally executes a narrow core-WASM surface.
It does not provide full Component Model canonical ABI execution, production
engine isolation, hardened daemon execution, supply-chain trust, generated
multi-language SDKs, or production telemetry sinks.

## Design Goals

- Provide full WASM Component Model execution behind the existing provider
  boundary.
- Support hardened out-of-process execution without leaking daemon, process, or
  engine details into public ABI.
- Make package provenance, signatures, and certification reports first-class
  admission inputs.
- Turn the scaffold/harness into a practical guest SDK and bindgen workflow.
- Connect runtime, admission, host import, lifecycle, and certification events
  to production observability sinks.
- Preserve Macaca's generic Agent OS boundary.  No implementation may hardcode
  workflow names, application names, provider names, driver names, or
  application-specific behavior.
- Keep files focused and below the project line limit by splitting modules by
  responsibility.

## Non-Goals

- The kernel will not own WASM engine selection or guest lifecycle.
- Public proto, SDK, app framework, Web, CLI, and Gateway layers will not depend
  on Wasmtime, WasmEdge, daemon IPC handles, or process-specific details.
- This design will not introduce specialized behavior for one application,
  workflow, tenant, driver, or commercial package.
- Raw WASM bytes, guest memory, command payloads, environment values, filesystem
  paths, network addresses, prompts, or secrets will not be logged.

## Architecture

The architecture remains a layered WASM Runtime Fabric:

1. Developer SDK and bindgen tooling produce WIT bindings, manifests, fixtures,
   local tests, and package descriptors.
2. Package admission checks ABI compatibility, artifact digest, provenance,
   signatures, import/export declarations, runtime capability requirements, and
   certification reports.
3. Runtime-host selects a provider strategy by execution profile and deployment
   policy.
4. The provider creates a session through a private adapter:
   - in-process core-WASM provider for compatibility and local tests,
   - production Component Model provider for WIT/canonical ABI execution,
   - hardened out-of-process provider for process isolation.
5. Host imports remain Commands that flow through service runtime, capability
   checks, trace context, payload guards, and audit reports.
6. Resource and lifecycle controls enforce policy at admission, provider, engine,
   and daemon boundaries.
7. Observability sinks receive sanitized audit events and metrics through an
   Observer-style fan-out interface.

## Design Patterns

- Bridge: provider/session traits, host import portal, and daemon IPC isolate
  callers from execution implementation.
- Abstract Factory and Strategy: provider registry creates sessions from the
  selected execution strategy without leaking concrete engine types.
- Adapter: Wasmtime/WasmEdge and daemon transports are private adapters behind
  runtime-host.
- Command: host imports, lifecycle operations, certification runs, and daemon
  requests use structured command envelopes.
- Specification: admission, supply-chain, runtime capability, and certification
  gates are explicit rule objects with stable reason codes.
- State: lifecycle transition rules remain centralized.
- Memento: checkpoint, certification, provenance, daemon response, and audit
  reports carry sanitized serializable state.
- Observer: telemetry sinks receive sanitized runtime events without coupling
  runtime logic to one backend.
- Null Object: unavailable and disabled providers continue to fail closed.

## Phase 1: Component Model Execution Provider

Add `add-wasm-component-model-execution-provider`.

The provider will add a production engine adapter inside `runtime-host` while
preserving the current provider-neutral contract.  The adapter must support
Component Model validation, WIT package matching, canonical ABI import/export
dispatch, engine-enforced resource controls, sanitized trap diagnostics, and
host import bridge integration.

The implementation may add an optional engine dependency, but only inside
`macaca-runtime-host`.  Public crates must continue to depend only on Macaca
DTOs and traits.

## Phase 2: Hardened Out-of-Process Provider

Add `add-wasm-hardened-out-of-process-provider`.

The hardened provider will turn the current envelope mock into a real daemon
transport contract and provider strategy.  Runtime-host remains the owner of
policy, provider selection, health checks, and sanitized diagnostics.  The
daemon owns process isolation, crash boundaries, cancellation, timeouts,
backpressure, and per-session execution.

The first implementation should support a deterministic local daemon transport
for tests before adding OS-specific hardening.  The provider must fail closed
when the daemon is unavailable, unhealthy, overloaded, or returns malformed
responses.

## Phase 3: Artifact Supply-Chain Verification

Add `add-wasm-artifact-supply-chain-verification`.

Admission will verify artifact digest, signature, signer identity, build
provenance, source origin, ABI declaration, import/export declarations, and
certification report compatibility before an application can be marked
industrial-ready.  The verification rules will use Specification objects and
produce sanitized reason codes.

The first implementation should use provider-neutral signature and provenance
DTOs with deterministic test keys/fixtures.  External key management and
commercial Store policy can be layered later through the same interfaces.

## Phase 4: Guest SDK Bindgen Toolchain

Add `add-wasm-guest-sdk-bindgen-toolchain`.

The SDK will move from static scaffolds to a real developer workflow:
generate bindings from WIT, build a Rust guest crate scaffold, run local mock
host-import tests, produce package fixtures, emit admission-ready descriptors,
and run certification locally.

The SDK must stay provider-neutral.  Generated code targets the Macaca ABI and
host import contracts, not a concrete engine or daemon.

## Phase 5: Production Observability Sinks

Add `add-wasm-production-observability-sinks`.

Runtime-host will expose sanitized telemetry event types and sink traits for
admission, provider selection, compile/instantiate/invoke, resource decisions,
host imports, lifecycle transitions, daemon health, certification, and
supply-chain checks.  Sinks must receive stable trace identifiers, reason codes,
safe subjects, durations, counters, and redacted diagnostics.

The first implementation should include an in-memory sink for tests and a
tracing/OpenTelemetry-compatible sink where existing dependencies permit it.

## Cross-Phase Invariants

- Every execution path must require trace context or return a structured
  missing-trace reason.
- Every denial, timeout, cancellation, trap, incompatibility, and unavailable
  path must produce a sanitized diagnostic and log/audit event.
- Resource enforcement must be layered: admission checks, provider checks,
  engine/daemon checks, and service portal checks.
- No raw payload, guest bytes, guest memory, secrets, environment, filesystem,
  or network values may enter logs, metrics, traces, certification reports, or
  checkpoint mementos.
- Public contracts must remain provider-neutral and extensible.

## OpenSpec Changes

The implementation program is split into five new OpenSpec changes:

- `add-wasm-component-model-execution-provider`
- `add-wasm-hardened-out-of-process-provider`
- `add-wasm-artifact-supply-chain-verification`
- `add-wasm-guest-sdk-bindgen-toolchain`
- `add-wasm-production-observability-sinks`

Each change must include `proposal.md`, `design.md`, `tasks.md`, and spec deltas
with strict OpenSpec validation before implementation begins.

## Verification Strategy

Each phase must add tests before implementation:

- Proto DTO serialization and sanitization tests.
- Admission and certification positive/negative tests.
- Runtime-host provider, daemon, resource, host import, lifecycle, and telemetry
  tests.
- SDK scaffold and bindgen fixture tests.
- OpenSpec `--strict` validation for every change.
- `cargo test` filters for the changed crates before each commit.

The final readiness gate is a Route C regression update that proves an
industrial-ready WASM package requires Component Model execution capability,
hardened isolation where configured, verified artifact provenance, local SDK
fixture compatibility, sanitized observability, and hardened certification.
