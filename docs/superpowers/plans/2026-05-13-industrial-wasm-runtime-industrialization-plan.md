# Industrial WASM Runtime Industrialization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete the remaining industrial WASM Application Runtime work by adding production Component Model execution, hardened out-of-process isolation, supply-chain verification, guest SDK bindgen tooling, and production observability sinks.

**Architecture:** Keep the existing provider-neutral Route C WASM Runtime Fabric and extend it in five independent OpenSpec-backed phases. Runtime-host owns concrete provider adapters and telemetry sinks; proto/app/sdk keep provider-neutral DTOs, admission rules, fixtures, and developer workflows.

**Tech Stack:** Rust, Cargo workspace under `macaca/`, OpenSpec, tracing, existing `macaca-proto`, `macaca-app`, `macaca-runtime-host`, `macaca-sdk`, optional runtime-host-only WASM engine dependency to be proposed in OpenSpec before use.

---

## File Structure

The implementation must keep files below 500 lines and split by responsibility.

- Create: `openspec/changes/add-wasm-component-model-execution-provider/proposal.md`
- Create: `openspec/changes/add-wasm-component-model-execution-provider/design.md`
- Create: `openspec/changes/add-wasm-component-model-execution-provider/tasks.md`
- Create: `openspec/changes/add-wasm-component-model-execution-provider/specs/wasm-runtime/spec.md`
- Create: `openspec/changes/add-wasm-hardened-out-of-process-provider/proposal.md`
- Create: `openspec/changes/add-wasm-hardened-out-of-process-provider/design.md`
- Create: `openspec/changes/add-wasm-hardened-out-of-process-provider/tasks.md`
- Create: `openspec/changes/add-wasm-hardened-out-of-process-provider/specs/wasm-runtime/spec.md`
- Create: `openspec/changes/add-wasm-artifact-supply-chain-verification/proposal.md`
- Create: `openspec/changes/add-wasm-artifact-supply-chain-verification/design.md`
- Create: `openspec/changes/add-wasm-artifact-supply-chain-verification/tasks.md`
- Create: `openspec/changes/add-wasm-artifact-supply-chain-verification/specs/wasm-package-admission/spec.md`
- Create: `openspec/changes/add-wasm-guest-sdk-bindgen-toolchain/proposal.md`
- Create: `openspec/changes/add-wasm-guest-sdk-bindgen-toolchain/design.md`
- Create: `openspec/changes/add-wasm-guest-sdk-bindgen-toolchain/tasks.md`
- Create: `openspec/changes/add-wasm-guest-sdk-bindgen-toolchain/specs/wasm-guest-toolchain/spec.md`
- Create: `openspec/changes/add-wasm-production-observability-sinks/proposal.md`
- Create: `openspec/changes/add-wasm-production-observability-sinks/design.md`
- Create: `openspec/changes/add-wasm-production-observability-sinks/tasks.md`
- Create: `openspec/changes/add-wasm-production-observability-sinks/specs/wasm-observability/spec.md`
- Modify: `macaca/crates/runtime/macaca-runtime-host/Cargo.toml`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/mod.rs`
- Create: `macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/component_model.rs`
- Create: `macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/component_model_tests.rs`
- Create: `macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/component_model_adapter.rs`
- Create: `macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/hardened_provider.rs`
- Create: `macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/hardened_transport.rs`
- Create: `macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/hardened_provider_tests.rs`
- Create: `macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/telemetry.rs`
- Create: `macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/telemetry_tests.rs`
- Modify: `macaca/crates/foundation/macaca-proto/src/wasm_package_admission.rs`
- Create: `macaca/crates/foundation/macaca-proto/src/wasm_supply_chain.rs`
- Modify: `macaca/crates/foundation/macaca-proto/src/lib.rs`
- Modify: `macaca/crates/application/macaca-app/src/certification/wasm_admission.rs`
- Create: `macaca/crates/application/macaca-app/src/certification/wasm_supply_chain.rs`
- Create: `macaca/crates/application/macaca-app/src/certification/wasm_supply_chain_tests.rs`
- Modify: `macaca/crates/facade/macaca-sdk/src/application_kit/wasm.rs`
- Create: `macaca/crates/facade/macaca-sdk/src/application_kit/wasm_bindgen.rs`
- Create: `macaca/crates/facade/macaca-sdk/src/application_kit/wasm_bindgen_tests.rs`
- Modify: `macaca/docs/route-c-regression-matrix.md`

## Task 1: Create OpenSpec Proposals for All Five Phases

**Files:**
- Create and validate all OpenSpec files listed in the File Structure section.

- [ ] **Step 1: Create `add-wasm-component-model-execution-provider` proposal**

Create `openspec/changes/add-wasm-component-model-execution-provider/proposal.md`:

```markdown
# Change: Add WASM Component Model execution provider

## Why
The current default provider executes only a narrow core-WASM nullary export surface. Industrial WASM applications require Component Model validation, WIT/canonical ABI import-export execution, engine-enforced resource controls, and sanitized trap diagnostics behind the existing provider-neutral contract.

## What Changes
- Add a runtime-host-only Component Model provider strategy.
- Add private engine adapter boundaries for Component Model validation and invocation.
- Route Component Model host imports through the existing service portal bridge.
- Enforce memory, fuel/epoch, timeout, and payload limits at provider and engine layers.
- Emit sanitized diagnostics and telemetry for compile, instantiate, invoke, trap, timeout, and resource decisions.

## Impact
- Affected specs: wasm-runtime
- Affected code: `macaca-runtime-host/src/wasm_runtime_provider`, `macaca-runtime-host/Cargo.toml`
```

- [ ] **Step 2: Create `add-wasm-component-model-execution-provider` design**

Create `openspec/changes/add-wasm-component-model-execution-provider/design.md`:

```markdown
## Context
The provider-neutral runtime contract already exists. The new provider must add production Component Model execution without exposing concrete engine types outside runtime-host.

## Goals / Non-Goals
- Goals: Component Model validation, WIT package matching, canonical ABI dispatch, host import bridge integration, engine-enforced limits, sanitized traps.
- Non-Goals: Public Wasmtime or WasmEdge DTOs, kernel-owned WASM execution, application-specific imports.

## Decisions
- Use Strategy and Abstract Factory through `WasmApplicationRuntimeProvider`.
- Use Adapter for the concrete engine boundary.
- Keep the existing in-process core-WASM provider for compatibility tests.
- Add engine dependency only to `macaca-runtime-host` after this proposal is approved.

## Risks / Trade-offs
- Engine dependency increases build surface. Mitigation: keep dependency private and feature-gated if needed.
- Component Model binding errors can leak payload details. Mitigation: sanitize all trap and ABI diagnostics.

## Migration Plan
Existing users keep the default provider. Deployment profiles opt into the Component Model provider when capability checks pass.
```

- [ ] **Step 3: Create `add-wasm-component-model-execution-provider` tasks**

Create `openspec/changes/add-wasm-component-model-execution-provider/tasks.md`:

```markdown
## 1. Implementation
- [ ] 1.1 Add failing runtime-host tests for Component Model provider descriptor, missing trace, invalid component, missing WIT export, host import bridge dispatch, timeout, and sanitized trap diagnostics.
- [ ] 1.2 Add private `component_model_adapter.rs` with an engine-neutral adapter trait and a production adapter implementation.
- [ ] 1.3 Add `component_model.rs` provider/session implementation using the existing `WasmApplicationRuntimeProvider` and `WasmExecutionSession` traits.
- [ ] 1.4 Wire the provider from `wasm_runtime_provider/mod.rs` without changing public proto/app/sdk dependencies.
- [ ] 1.5 Add engine-enforced resource checks and map all failures to `WasmRuntimeErrorReport`.
- [ ] 1.6 Add logging at compile, instantiate, invoke, timeout, trap, host import, and shutdown boundaries.
- [ ] 1.7 Run OpenSpec validation and targeted cargo tests.
```

- [ ] **Step 4: Create `add-wasm-component-model-execution-provider` spec delta**

Create `openspec/changes/add-wasm-component-model-execution-provider/specs/wasm-runtime/spec.md`:

```markdown
## ADDED Requirements

### Requirement: Component Model Provider Execution
The system SHALL provide a runtime-host-owned WASM Component Model execution provider that implements the existing provider-neutral `WasmApplicationRuntimeProvider` contract without exposing concrete engine types to public ABI, SDK, application framework, kernel, CLI, Web, or Gateway layers.

#### Scenario: Component export invocation succeeds
- **WHEN** an admitted WASM Component artifact declares a supported WIT export and the command includes trace context
- **THEN** the provider SHALL instantiate the component, invoke the export, route host imports through the service portal, and return a sanitized successful command result

#### Scenario: Component execution fails closed
- **WHEN** a component is invalid, misses a required export, traps, exceeds resource limits, or omits trace context
- **THEN** the provider SHALL return a structured sanitized diagnostic with a stable reason code and SHALL NOT log raw guest bytes, payloads, memory, secrets, filesystem paths, environment values, or network values
```

- [ ] **Step 5: Repeat proposal/design/tasks/spec creation for hardened provider**

Create `openspec/changes/add-wasm-hardened-out-of-process-provider/proposal.md`:

```markdown
# Change: Add WASM hardened out-of-process provider

## Why
Industrial WASM execution needs process isolation, health checks, cancellation, timeout, backpressure, and crash recovery. The current hardened provider envelope is a mock contract only.

## What Changes
- Add a runtime-host provider strategy that dispatches to a hardened daemon transport.
- Add provider-neutral daemon request/response validation and sanitized diagnostics.
- Add health, overload, malformed response, timeout, cancellation, and crash recovery handling.
- Preserve existing provider/session traits and host import command semantics.

## Impact
- Affected specs: wasm-runtime
- Affected code: `macaca-runtime-host/src/wasm_runtime_provider`
```

Create `openspec/changes/add-wasm-hardened-out-of-process-provider/design.md`:

```markdown
## Context
The existing hardened envelope mock proves the data shape. A real provider must use the same semantics while moving execution out of process.

## Goals / Non-Goals
- Goals: daemon transport abstraction, health checks, timeout/cancellation, backpressure, malformed response rejection, sanitized diagnostics.
- Non-Goals: engine-specific public ABI, OS-specific sandbox policy in public crates, application-specialized daemon behavior.

## Decisions
- Use Bridge and Adapter for daemon transport.
- Use Null Object fail-closed behavior when the daemon is unavailable.
- Keep daemon request and response envelopes sanitized and provider-neutral.

## Risks / Trade-offs
- Out-of-process execution adds operational complexity. Mitigation: implement deterministic local transport tests before OS-specific hardening.

## Migration Plan
Deployment profiles can select the hardened provider after conformance tests pass. Existing in-process providers remain available for development.
```

Create `openspec/changes/add-wasm-hardened-out-of-process-provider/tasks.md`:

```markdown
## 1. Implementation
- [ ] 1.1 Add failing tests for daemon unavailable, unhealthy, overloaded, timeout, cancellation, malformed response, crash recovery, and sanitized diagnostics.
- [ ] 1.2 Add `hardened_transport.rs` transport trait with deterministic in-memory test transport.
- [ ] 1.3 Add `hardened_provider.rs` provider/session implementation.
- [ ] 1.4 Reuse existing hardened envelope and response DTOs where possible.
- [ ] 1.5 Add health and backpressure state handling.
- [ ] 1.6 Add logs for provider selection, daemon health, dispatch, cancellation, timeout, overload, and crash recovery.
- [ ] 1.7 Run OpenSpec validation and targeted cargo tests.
```

Create `openspec/changes/add-wasm-hardened-out-of-process-provider/specs/wasm-runtime/spec.md`:

```markdown
## ADDED Requirements

### Requirement: Hardened Out-of-Process Provider
The system SHALL provide a runtime-host-owned hardened WASM provider that executes through an out-of-process daemon transport while preserving the provider-neutral runtime contract.

#### Scenario: Daemon execution succeeds
- **WHEN** the daemon is healthy and accepts a traced execution envelope
- **THEN** the provider SHALL return a sanitized response mapped to the existing runtime command result and SHALL emit provider, daemon, and lifecycle audit events

#### Scenario: Daemon execution fails closed
- **WHEN** the daemon is unavailable, unhealthy, overloaded, timed out, cancelled, crashed, or returns a malformed response
- **THEN** the provider SHALL fail closed with a stable reason code and sanitized diagnostic
```

- [ ] **Step 6: Repeat proposal/design/tasks/spec creation for supply-chain verification**

Create `openspec/changes/add-wasm-artifact-supply-chain-verification/proposal.md`:

```markdown
# Change: Add WASM artifact supply-chain verification

## Why
Industrial package admission must verify artifact identity, signature, signer, provenance, origin, ABI, and certification compatibility before a WASM application can be marked industrial-ready.

## What Changes
- Add provider-neutral signature and provenance DTOs.
- Add supply-chain verification rules to package admission and certification.
- Add deterministic signed and unsigned fixtures.
- Add sanitized reason codes for digest mismatch, missing signature, untrusted signer, stale provenance, origin mismatch, and incompatible certification.

## Impact
- Affected specs: wasm-package-admission
- Affected code: `macaca-proto`, `macaca-app`, `macaca-sdk` fixtures
```

Create `openspec/changes/add-wasm-artifact-supply-chain-verification/design.md`:

```markdown
## Context
Current admission validates artifact digest and ABI metadata. It does not validate signatures, signer trust, source origin, or build provenance.

## Goals / Non-Goals
- Goals: signed artifact DTOs, provenance DTOs, deterministic test verification, admission integration, certification report compatibility.
- Non-Goals: production KMS integration, one Store-specific policy, raw key material logging.

## Decisions
- Use Specification for verification rules.
- Use Memento-style sanitized verification reports.
- Keep trust policy provider-neutral so Store or CI can supply trusted signer sets later.

## Risks / Trade-offs
- Crypto dependencies can increase build scope. Mitigation: start with deterministic verifier trait and test fixtures, then add real crypto only behind approved dependency review.

## Migration Plan
Existing packages without signatures remain non-industrial-ready until policy explicitly allows development mode.
```

Create `openspec/changes/add-wasm-artifact-supply-chain-verification/tasks.md`:

```markdown
## 1. Implementation
- [ ] 1.1 Add failing proto tests for signed artifact metadata serialization and sanitization.
- [ ] 1.2 Add `wasm_supply_chain.rs` DTOs for signature, signer, provenance, origin, trust policy, and verification report.
- [ ] 1.3 Add admission tests for accepted signed artifact, missing signature, digest mismatch, untrusted signer, stale provenance, origin mismatch, and incompatible certification.
- [ ] 1.4 Add `wasm_supply_chain.rs` admission specification in `macaca-app`.
- [ ] 1.5 Integrate verification into `WasmPackageAdmissionSpec`.
- [ ] 1.6 Add SDK package fixtures for signed and rejected artifacts.
- [ ] 1.7 Add sanitized logs for supply-chain verification decisions.
- [ ] 1.8 Run OpenSpec validation and targeted cargo tests.
```

Create `openspec/changes/add-wasm-artifact-supply-chain-verification/specs/wasm-package-admission/spec.md`:

```markdown
## ADDED Requirements

### Requirement: WASM Supply-Chain Admission Gate
The system SHALL verify WASM artifact digest, signature, signer trust, source origin, build provenance, ABI declaration, and certification compatibility before reporting an artifact as industrial-ready.

#### Scenario: Verified artifact is accepted
- **WHEN** a WASM artifact has a matching digest, trusted signature, accepted origin, fresh provenance, compatible ABI, and compatible certification report
- **THEN** package admission SHALL include a successful sanitized supply-chain verification report

#### Scenario: Untrusted artifact is rejected
- **WHEN** a WASM artifact is missing a signature, has a digest mismatch, has an untrusted signer, has stale provenance, has an origin mismatch, or has incompatible certification
- **THEN** package admission SHALL reject industrial readiness with stable sanitized reason codes
```

- [ ] **Step 7: Repeat proposal/design/tasks/spec creation for SDK bindgen toolchain**

Create `openspec/changes/add-wasm-guest-sdk-bindgen-toolchain/proposal.md`:

```markdown
# Change: Add WASM guest SDK bindgen toolchain

## Why
The current SDK scaffold and harness prove contracts, but third-party developers need generated bindings, local tests, package fixture generation, and certification commands to build real WASM applications.

## What Changes
- Add provider-neutral WIT bindgen planning and generated Rust guest scaffold DTOs.
- Add local mock host-import test runner surfaces.
- Add package descriptor and fixture generation from WIT and manifest inputs.
- Add SDK tests that prevent runtime/toolchain drift.

## Impact
- Affected specs: wasm-guest-toolchain
- Affected code: `macaca-sdk`, runtime guest harness fixtures
```

Create `openspec/changes/add-wasm-guest-sdk-bindgen-toolchain/design.md`:

```markdown
## Context
The SDK currently builds a static scaffold. Industrial developer workflow requires WIT-driven generated bindings and local certification feedback.

## Goals / Non-Goals
- Goals: WIT input validation, generated binding plan, Rust guest scaffold, mock host import tests, package fixture generation, local certification report.
- Non-Goals: engine-specific generated code, application-specialized scaffold behavior, hardcoded workflows.

## Decisions
- Use Builder for scaffold generation.
- Use Adapter for bindgen backend so future languages can be added.
- Use the existing runtime guest harness for local host import behavior.

## Risks / Trade-offs
- Full code generation can become large. Mitigation: start with deterministic generated source DTOs and fixture tests, then add CLI integration in a later Store/CLI phase.

## Migration Plan
Existing scaffold API remains; new bindgen builder extends it with WIT-driven generation.
```

Create `openspec/changes/add-wasm-guest-sdk-bindgen-toolchain/tasks.md`:

```markdown
## 1. Implementation
- [ ] 1.1 Add failing SDK tests for WIT input validation, binding plan generation, Rust scaffold generation, mock host import registration, fixture generation, and local certification report.
- [ ] 1.2 Add `wasm_bindgen.rs` with bindgen input, output, diagnostic, backend trait, and Rust scaffold builder.
- [ ] 1.3 Integrate generated package descriptor output with existing `WasmComponentApplicationScaffold`.
- [ ] 1.4 Reuse runtime guest harness fixture shapes for local mock host imports.
- [ ] 1.5 Add sanitized logs for bindgen planning, scaffold generation, fixture emission, and local certification.
- [ ] 1.6 Run OpenSpec validation and targeted cargo tests.
```

Create `openspec/changes/add-wasm-guest-sdk-bindgen-toolchain/specs/wasm-guest-toolchain/spec.md`:

```markdown
## ADDED Requirements

### Requirement: WIT-Driven Guest SDK Toolchain
The system SHALL provide a provider-neutral guest SDK toolchain that validates WIT inputs, generates Rust guest binding scaffolds, emits package fixtures, registers mock host imports, and runs local certification feedback without depending on a concrete runtime engine.

#### Scenario: Guest scaffold is generated
- **WHEN** a developer provides valid WIT metadata, package identity, ABI version, declared imports, and declared exports
- **THEN** the SDK SHALL generate a deterministic Rust guest scaffold and admission-ready package fixture

#### Scenario: Guest scaffold input is rejected
- **WHEN** WIT metadata is malformed, missing required imports, declares unsupported ABI, or attempts to request raw host resources
- **THEN** the SDK SHALL reject generation with sanitized diagnostics and stable reason codes
```

- [ ] **Step 8: Repeat proposal/design/tasks/spec creation for observability sinks**

Create `openspec/changes/add-wasm-production-observability-sinks/proposal.md`:

```markdown
# Change: Add WASM production observability sinks

## Why
Industrial WASM operation requires sanitized telemetry for admission, provider selection, compile, instantiate, invoke, host imports, resource decisions, lifecycle transitions, daemon health, certification, and supply-chain checks.

## What Changes
- Add runtime-host telemetry event DTOs and sink traits.
- Add in-memory test sink and tracing-compatible sink.
- Emit sanitized events from key runtime provider paths.
- Add tests proving raw payloads and secrets never enter telemetry.

## Impact
- Affected specs: wasm-observability
- Affected code: `macaca-runtime-host/src/wasm_runtime_provider`
```

Create `openspec/changes/add-wasm-production-observability-sinks/design.md`:

```markdown
## Context
Current code logs many key nodes, but there is no single production sink contract for runtime events and metrics.

## Goals / Non-Goals
- Goals: observer sink trait, sanitized event DTOs, test sink, tracing sink, emission at all WASM runtime decision points.
- Non-Goals: one vendor-specific dashboard, raw payload export, app-specific telemetry schema.

## Decisions
- Use Observer for sink fan-out.
- Use Memento-style event DTOs so telemetry is serializable and auditable.
- Keep sink failures non-fatal unless policy explicitly marks audit persistence as mandatory.

## Risks / Trade-offs
- Telemetry can leak sensitive data. Mitigation: sanitizer tests and safe-subject fields only.

## Migration Plan
Existing logs remain. Runtime provider paths progressively emit structured events to the configured sink.
```

Create `openspec/changes/add-wasm-production-observability-sinks/tasks.md`:

```markdown
## 1. Implementation
- [ ] 1.1 Add failing runtime-host tests for telemetry event emission and redaction across admission, compile, instantiate, invoke, resource, host import, lifecycle, daemon, certification, and supply-chain paths.
- [ ] 1.2 Add `telemetry.rs` event DTOs, sink trait, in-memory sink, tracing sink, and sanitizer helpers.
- [ ] 1.3 Inject telemetry sink into provider constructors using optional Arc dependencies.
- [ ] 1.4 Emit events from unavailable, default in-process, Component Model, hardened, sandbox guard, host import bridge, lifecycle support, guest harness, and certification paths.
- [ ] 1.5 Update Route C regression matrix with observability readiness rows.
- [ ] 1.6 Run OpenSpec validation and targeted cargo tests.
```

Create `openspec/changes/add-wasm-production-observability-sinks/specs/wasm-observability/spec.md`:

```markdown
## ADDED Requirements

### Requirement: WASM Runtime Observability Sinks
The system SHALL provide sanitized WASM runtime telemetry events and sink interfaces for admission, provider selection, compile, instantiate, invoke, resource decisions, host imports, lifecycle transitions, daemon health, certification, and supply-chain verification.

#### Scenario: Runtime event is emitted
- **WHEN** a WASM runtime decision point completes, fails, rejects, times out, traps, or is unavailable
- **THEN** the configured telemetry sink SHALL receive a sanitized event with trace id, event kind, safe subject, reason code, status, duration where available, and redacted diagnostics

#### Scenario: Sensitive data is redacted
- **WHEN** runtime inputs contain raw payloads, guest bytes, memory, secrets, filesystem paths, environment values, network values, prompts, or API keys
- **THEN** telemetry SHALL NOT include those raw values and SHALL include only sanitized reason codes and safe metadata
```

- [ ] **Step 9: Validate all five OpenSpec changes**

Run:

```bash
openspec validate add-wasm-component-model-execution-provider --strict
openspec validate add-wasm-hardened-out-of-process-provider --strict
openspec validate add-wasm-artifact-supply-chain-verification --strict
openspec validate add-wasm-guest-sdk-bindgen-toolchain --strict
openspec validate add-wasm-production-observability-sinks --strict
```

Expected: every command prints `Change '<change-id>' is valid`.

- [ ] **Step 10: Commit OpenSpec proposals**

Run:

```bash
git add openspec/changes/add-wasm-component-model-execution-provider \
  openspec/changes/add-wasm-hardened-out-of-process-provider \
  openspec/changes/add-wasm-artifact-supply-chain-verification \
  openspec/changes/add-wasm-guest-sdk-bindgen-toolchain \
  openspec/changes/add-wasm-production-observability-sinks
git commit -m "spec: plan industrial wasm runtime completion"
```

Expected: commit includes only the five OpenSpec change directories.

## Task 2: Implement Component Model Provider

**Files:**
- Modify: `macaca/crates/runtime/macaca-runtime-host/Cargo.toml`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/mod.rs`
- Create: `macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/component_model.rs`
- Create: `macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/component_model_adapter.rs`
- Create: `macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/component_model_tests.rs`

- [ ] **Step 1: Run impact analysis before editing provider symbols**

Run GitNexus impact analysis for:

```text
WasmApplicationRuntimeProvider
WasmExecutionSession
DefaultInProcessWasmRuntimeProvider
WasmRuntimeProviderRegistry
```

Expected: review direct callers and affected processes. If HIGH or CRITICAL risk appears, report it before editing.

- [ ] **Step 2: Add failing tests**

Add tests in `component_model_tests.rs` covering:

```rust
#[tokio::test]
async fn component_model_provider_requires_trace_context() {
    // Build a request without TraceContext and assert the provider returns a
    // missing-trace ApplicationAbiError with sanitized diagnostics.
}

#[tokio::test]
async fn component_model_provider_rejects_invalid_component_without_raw_bytes() {
    // Feed invalid bytes through the adapter and assert diagnostics do not
    // contain raw module bytes or secret markers.
}

#[tokio::test]
async fn component_model_provider_routes_host_imports_through_service_portal() {
    // Register a mock service portal bridge and assert host import commands
    // preserve trace, capability, import kind, and safe metadata.
}
```

Run:

```bash
cargo test -p macaca-runtime-host component_model --manifest-path macaca/Cargo.toml
```

Expected: tests fail because the provider modules do not exist.

- [ ] **Step 3: Implement adapter boundary**

Create `component_model_adapter.rs` with:

```rust
pub(crate) trait WasmComponentEngineAdapter: Send + Sync {
    fn validate_component(&self, artifact: &[u8]) -> Result<WasmComponentModule, WasmRuntimeHostError>;
    fn instantiate(&self, module: WasmComponentModule) -> Result<WasmComponentInstance, WasmRuntimeHostError>;
}
```

Include detailed English comments explaining why the adapter is private and how it prevents engine leakage.

- [ ] **Step 4: Implement provider/session**

Create `component_model.rs` implementing `WasmApplicationRuntimeProvider` and `WasmExecutionSession`.  The implementation must:

```rust
pub struct ComponentModelWasmRuntimeProvider {
    adapter: Arc<dyn WasmComponentEngineAdapter>,
    host_import_bridge: Option<Arc<WasmHostImportBridge>>,
}
```

Map every compile, instantiate, invoke, timeout, and trap failure to sanitized `ApplicationAbiError`.

- [ ] **Step 5: Export module and run tests**

Modify `mod.rs`:

```rust
mod component_model;
mod component_model_adapter;

pub use component_model::ComponentModelWasmRuntimeProvider;
```

Run:

```bash
cargo test -p macaca-runtime-host component_model --manifest-path macaca/Cargo.toml
```

Expected: all component model tests pass.

- [ ] **Step 6: Validate and commit**

Run:

```bash
openspec validate add-wasm-component-model-execution-provider --strict
git diff --check
```

Run GitNexus detect changes before commit. Then commit:

```bash
git add macaca/crates/runtime/macaca-runtime-host/Cargo.toml \
  macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider
git commit -m "feat: add wasm component model provider"
```

## Task 3: Implement Hardened Out-of-Process Provider

**Files:**
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/mod.rs`
- Create: `macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/hardened_transport.rs`
- Create: `macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/hardened_provider.rs`
- Create: `macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/hardened_provider_tests.rs`

- [ ] **Step 1: Run impact analysis**

Run GitNexus impact analysis for:

```text
WasmHardenedProviderEnvelope
WasmHardenedProviderResponse
WasmApplicationRuntimeProvider
```

Expected: review direct callers and affected processes before editing.

- [ ] **Step 2: Add failing hardened provider tests**

Add tests for:

```rust
#[tokio::test]
async fn hardened_provider_fails_closed_when_daemon_unavailable() {}

#[tokio::test]
async fn hardened_provider_rejects_malformed_daemon_response() {}

#[tokio::test]
async fn hardened_provider_preserves_trace_for_successful_daemon_dispatch() {}

#[tokio::test]
async fn hardened_provider_reports_backpressure_without_raw_payload() {}
```

Run:

```bash
cargo test -p macaca-runtime-host hardened_provider --manifest-path macaca/Cargo.toml
```

Expected: tests fail because transport/provider modules do not exist.

- [ ] **Step 3: Implement transport trait and test transport**

Create `hardened_transport.rs`:

```rust
#[async_trait::async_trait]
pub(crate) trait WasmHardenedTransport: Send + Sync {
    async fn health(&self, trace: TraceContext) -> WasmHardenedHealth;
    async fn dispatch(&self, envelope: WasmHardenedProviderEnvelope) -> WasmHardenedProviderResponse;
}
```

Add deterministic in-memory transport for tests.

- [ ] **Step 4: Implement provider/session**

Create `hardened_provider.rs` using the transport. It must fail closed for unavailable, unhealthy, overload, timeout, cancellation, crash, and malformed response paths.

- [ ] **Step 5: Export module and run tests**

Modify `mod.rs`:

```rust
mod hardened_provider;
mod hardened_transport;

pub use hardened_provider::HardenedOutOfProcessWasmRuntimeProvider;
```

Run:

```bash
cargo test -p macaca-runtime-host hardened_provider --manifest-path macaca/Cargo.toml
```

Expected: all hardened provider tests pass.

- [ ] **Step 6: Validate and commit**

Run:

```bash
openspec validate add-wasm-hardened-out-of-process-provider --strict
git diff --check
```

Run GitNexus detect changes before commit. Then commit:

```bash
git add macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider
git commit -m "feat: add hardened wasm runtime provider"
```

## Task 4: Implement Artifact Supply-Chain Verification

**Files:**
- Modify: `macaca/crates/foundation/macaca-proto/src/lib.rs`
- Create: `macaca/crates/foundation/macaca-proto/src/wasm_supply_chain.rs`
- Modify: `macaca/crates/application/macaca-app/src/certification/wasm_admission.rs`
- Create: `macaca/crates/application/macaca-app/src/certification/wasm_supply_chain.rs`
- Create: `macaca/crates/application/macaca-app/src/certification/wasm_supply_chain_tests.rs`
- Modify: `macaca/crates/facade/macaca-sdk/src/package_fixtures/application_platform_wasm.rs`

- [ ] **Step 1: Run impact analysis**

Run GitNexus impact analysis for:

```text
WasmPackageAdmissionSpec
WasmPackageAdmissionReport
WasmComponentArtifactDescriptor
ApplicationCertificationFixture
```

Expected: review direct callers and affected processes before editing.

- [ ] **Step 2: Add failing proto and admission tests**

Add tests for signed artifact acceptance and all rejection paths:

```rust
#[test]
fn wasm_supply_chain_report_is_sanitized() {}

#[test]
fn wasm_admission_rejects_untrusted_signer() {}

#[test]
fn wasm_admission_rejects_digest_mismatch() {}

#[test]
fn wasm_admission_accepts_verified_artifact_for_industrial_readiness() {}
```

Run:

```bash
cargo test -p macaca-proto wasm_supply_chain --manifest-path macaca/Cargo.toml
cargo test -p macaca-app wasm_supply_chain --manifest-path macaca/Cargo.toml
```

Expected: tests fail because DTOs and rules do not exist.

- [ ] **Step 3: Add provider-neutral DTOs**

Create `wasm_supply_chain.rs` in `macaca-proto` with signature, signer, provenance, origin, trust policy, verification status, reason code, and report structs. Add English comments for each public type and method.

- [ ] **Step 4: Add admission specification**

Create `wasm_supply_chain.rs` in `macaca-app/src/certification` implementing deterministic verification. It must never log raw key material or raw artifact bytes.

- [ ] **Step 5: Integrate with admission**

Modify `WasmPackageAdmissionSpec` so industrial-ready status requires successful supply-chain verification when the context policy requires it.

- [ ] **Step 6: Run tests and commit**

Run:

```bash
cargo test -p macaca-proto wasm_supply_chain --manifest-path macaca/Cargo.toml
cargo test -p macaca-app wasm_supply_chain --manifest-path macaca/Cargo.toml
openspec validate add-wasm-artifact-supply-chain-verification --strict
git diff --check
```

Run GitNexus detect changes before commit. Then commit:

```bash
git add macaca/crates/foundation/macaca-proto/src \
  macaca/crates/application/macaca-app/src/certification \
  macaca/crates/facade/macaca-sdk/src/package_fixtures
git commit -m "feat: add wasm supply chain admission"
```

## Task 5: Implement Guest SDK Bindgen Toolchain

**Files:**
- Modify: `macaca/crates/facade/macaca-sdk/src/application_kit/mod.rs`
- Modify: `macaca/crates/facade/macaca-sdk/src/application_kit/wasm.rs`
- Create: `macaca/crates/facade/macaca-sdk/src/application_kit/wasm_bindgen.rs`
- Create: `macaca/crates/facade/macaca-sdk/src/application_kit/wasm_bindgen_tests.rs`

- [ ] **Step 1: Run impact analysis**

Run GitNexus impact analysis for:

```text
WasmComponentApplicationScaffold
WasmComponentApplicationDescriptor
```

Expected: review direct callers and affected processes before editing.

- [ ] **Step 2: Add failing SDK tests**

Add tests:

```rust
#[test]
fn wasm_bindgen_generates_rust_guest_scaffold_from_wit() {}

#[test]
fn wasm_bindgen_rejects_raw_host_resource_requests() {}

#[test]
fn wasm_bindgen_emits_admission_ready_fixture() {}

#[test]
fn wasm_bindgen_report_is_sanitized() {}
```

Run:

```bash
cargo test -p macaca-sdk wasm_bindgen --manifest-path macaca/Cargo.toml
```

Expected: tests fail because bindgen module does not exist.

- [ ] **Step 3: Implement bindgen DTOs and backend trait**

Create `wasm_bindgen.rs` with input, output, diagnostic, backend trait, Rust scaffold backend, and builder structs. Comments must explain how the toolchain remains provider-neutral.

- [ ] **Step 4: Integrate scaffold builder**

Modify `application_kit/mod.rs` and `wasm.rs` to export the bindgen builder and convert generated output into existing `WasmComponentApplicationDescriptor`.

- [ ] **Step 5: Run tests and commit**

Run:

```bash
cargo test -p macaca-sdk wasm_bindgen --manifest-path macaca/Cargo.toml
openspec validate add-wasm-guest-sdk-bindgen-toolchain --strict
git diff --check
```

Run GitNexus detect changes before commit. Then commit:

```bash
git add macaca/crates/facade/macaca-sdk/src/application_kit
git commit -m "feat: add wasm guest bindgen toolchain"
```

## Task 6: Implement Production Observability Sinks

**Files:**
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/mod.rs`
- Create: `macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/telemetry.rs`
- Create: `macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/telemetry_tests.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/default_provider.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/unavailable.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/host_import_bridge.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/lifecycle_support.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/certification.rs`
- Modify: `macaca/docs/route-c-regression-matrix.md`

- [ ] **Step 1: Run impact analysis**

Run GitNexus impact analysis for:

```text
DefaultInProcessWasmRuntimeProvider
UnavailableWasmRuntimeProvider
WasmHostImportBridge
WasmCertificationHarness
```

Expected: review direct callers and affected processes before editing.

- [ ] **Step 2: Add failing telemetry tests**

Add tests:

```rust
#[tokio::test]
async fn wasm_telemetry_records_provider_selection_and_invoke() {}

#[tokio::test]
async fn wasm_telemetry_records_host_import_denial_without_payload() {}

#[tokio::test]
async fn wasm_telemetry_records_lifecycle_transition() {}

#[test]
fn wasm_telemetry_redacts_secrets_and_raw_values() {}
```

Run:

```bash
cargo test -p macaca-runtime-host wasm_telemetry --manifest-path macaca/Cargo.toml
```

Expected: tests fail because telemetry module does not exist.

- [ ] **Step 3: Implement telemetry event and sink**

Create `telemetry.rs`:

```rust
pub trait WasmRuntimeTelemetrySink: Send + Sync {
    fn record(&self, event: WasmRuntimeTelemetryEvent);
}
```

Add event kind, status, reason code, safe subject, trace id, duration, in-memory sink, tracing sink, and sanitizer helpers.

- [ ] **Step 4: Emit telemetry from runtime paths**

Inject optional `Arc<dyn WasmRuntimeTelemetrySink>` into provider constructors. Emit events from unavailable provider, default provider, Component Model provider, hardened provider, sandbox guard, host import bridge, lifecycle support, and certification harness.

- [ ] **Step 5: Update regression matrix**

Add rows to `macaca/docs/route-c-regression-matrix.md` for production WASM telemetry redaction and sink coverage.

- [ ] **Step 6: Run tests and commit**

Run:

```bash
cargo test -p macaca-runtime-host wasm_telemetry --manifest-path macaca/Cargo.toml
openspec validate add-wasm-production-observability-sinks --strict
git diff --check
```

Run GitNexus detect changes before commit. Then commit:

```bash
git add macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider \
  macaca/docs/route-c-regression-matrix.md
git commit -m "feat: add wasm runtime observability sinks"
```

## Task 7: Final Industrial Readiness Gate

**Files:**
- Modify: `macaca/docs/2026-05-13-industrial-wasm-application-runtime-audit.md`
- Modify: `macaca/docs/route-c-regression-matrix.md`
- Modify: each completed `openspec/changes/add-wasm-*/tasks.md` introduced by this plan.

- [ ] **Step 1: Add final regression tests**

Run all targeted WASM tests:

```bash
cargo test -p macaca-proto wasm --manifest-path macaca/Cargo.toml
cargo test -p macaca-app wasm --manifest-path macaca/Cargo.toml
cargo test -p macaca-sdk wasm --manifest-path macaca/Cargo.toml
cargo test -p macaca-runtime-host wasm --manifest-path macaca/Cargo.toml
```

Expected: all tests pass.

- [ ] **Step 2: Validate all OpenSpec changes**

Run:

```bash
openspec validate add-wasm-component-model-execution-provider --strict
openspec validate add-wasm-hardened-out-of-process-provider --strict
openspec validate add-wasm-artifact-supply-chain-verification --strict
openspec validate add-wasm-guest-sdk-bindgen-toolchain --strict
openspec validate add-wasm-production-observability-sinks --strict
```

Expected: every command prints `Change '<change-id>' is valid`.

- [ ] **Step 3: Update audit status**

Update `macaca/docs/2026-05-13-industrial-wasm-application-runtime-audit.md` so it records which remaining industrial gaps are now complete and which external operational work remains.

- [ ] **Step 4: Mark OpenSpec tasks complete**

After implementation and verification are complete, set each relevant task item in the five new `tasks.md` files to `- [x]`.

- [ ] **Step 5: Run final change detection and commit**

Run GitNexus detect changes before commit. Then run:

```bash
git status --short
git diff --check
git add macaca/docs/2026-05-13-industrial-wasm-application-runtime-audit.md \
  macaca/docs/route-c-regression-matrix.md \
  openspec/changes/add-wasm-component-model-execution-provider \
  openspec/changes/add-wasm-hardened-out-of-process-provider \
  openspec/changes/add-wasm-artifact-supply-chain-verification \
  openspec/changes/add-wasm-guest-sdk-bindgen-toolchain \
  openspec/changes/add-wasm-production-observability-sinks
git commit -m "docs: certify industrial wasm runtime readiness"
```

Expected: final commit records docs/spec completion state after all runtime, admission, SDK, and observability tests pass.

## Self-Review

- Spec coverage: the plan maps the audit's five remaining gaps to five OpenSpec changes and five implementation phases.
- Placeholder scan: no task relies on unresolved placeholders; exact file paths and validation commands are listed.
- Type consistency: provider, session, admission, SDK, and telemetry names match existing project naming conventions or are introduced before use.
- Risk control: each code-edit task starts with GitNexus impact analysis and ends with GitNexus change detection before commit.
