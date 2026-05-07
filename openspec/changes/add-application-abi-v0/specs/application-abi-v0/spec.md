## ADDED Requirements

### Requirement: Macaca SHALL define Application ABI v0 contracts

Macaca SHALL define provider-neutral Application ABI v0 contracts that describe ABI version, required exports, allowed host imports, lifecycle events, application events, render requests, host commands, command results, checkpoints, trace context, metadata, and structured ABI errors.

#### Scenario: Application ABI declaration round trips through serde

- **WHEN** an Application ABI declaration is serialized and deserialized
- **THEN** the decoded declaration SHALL preserve ABI version, exports, imports, package reference, application identity, permissions, lifecycle metadata, trace metadata, and arbitrary metadata
- **AND** the declaration SHALL NOT depend on `macaca-web`, frontend code, concrete provider crates, concrete driver implementations, concrete gateway implementations, chain implementations, Store implementation, payment implementation, or business workflows

### Requirement: Macaca SHALL define required ABI exports

Application ABI v0 SHALL define exports for `app:init`, `app:start`, `app:handle_event`, `app:render`, `app:pause`, `app:resume`, `app:shutdown`, and `app:upgrade`.

#### Scenario: Missing required export is rejected

- **WHEN** an application package declares Application ABI v0 but omits a required export
- **THEN** ABI validation SHALL reject the declaration with a structured missing-export error
- **AND** the application SHALL NOT be started through ABI dispatch

#### Scenario: Future export remains structured

- **WHEN** an application declares an export that the current host does not understand
- **THEN** parsing SHALL preserve the unknown export as structured data
- **AND** execution of the unknown export SHALL return a structured unsupported-export error instead of panicking, hanging, or silently accepting unsafe behavior

### Requirement: Macaca SHALL define required host imports

Application ABI v0 SHALL define host imports for `macaca:capability/request`, `macaca:task/create_goal`, `macaca:task/query`, `macaca:trace/emit`, `macaca:ui/render`, `macaca:storage/get`, `macaca:storage/set`, `macaca:payment/create_intent`, and `macaca:service/call`.

#### Scenario: Host import command carries trace context

- **WHEN** an application invokes a host import that affects lifecycle, capability, task, trace, UI, storage, payment, or service-call behavior
- **THEN** the host command SHALL carry trace context or be rejected before dispatch with a structured missing-trace error
- **AND** the rejection SHALL be traceable and auditable

#### Scenario: Unsupported host import returns structured unavailable

- **WHEN** an application invokes a declared host import whose backing service is unavailable in Phase 05
- **THEN** the host SHALL return a structured unavailable, disabled-by-policy, unsupported, or runtime-unavailable result
- **AND** the host SHALL NOT panic, hang, fake success, or bypass policy/trace boundaries

### Requirement: Macaca SHALL expose an ApplicationHost facade

Macaca SHALL expose an `ApplicationHost` facade that is the controlled application-facing boundary for capability requests, task operations, trace emission, UI render requests, app-scoped storage, payment intent creation, and generic service calls.

#### Scenario: Application cannot directly access internal host state

- **WHEN** application ABI code requests a system capability
- **THEN** it SHALL go through `ApplicationHost` or equivalent ABI host command dispatch
- **AND** it SHALL NOT receive direct access to `Arc<AppState>`, web runtime internals, framework runner internals, concrete provider clients, concrete driver clients, or application-specific workflow routers

#### Scenario: Task create goal uses existing task path

- **WHEN** an ABI application calls `macaca:task/create_goal` with valid trace context and policy-ready metadata
- **THEN** `ApplicationHost` SHALL route the request through the existing task creation path where Phase 05 integration supports it
- **AND** the result SHALL include structured status and trace/audit metadata

#### Scenario: Trace emit uses existing trace path

- **WHEN** an ABI application calls `macaca:trace/emit` with valid trace context
- **THEN** `ApplicationHost` SHALL route the trace through the existing trace/EventLog/RunTrace path where Phase 05 integration supports it
- **AND** the emitted event SHALL be observable through existing trace consumers without requiring application-specific code

### Requirement: Macaca SHALL model Application lifecycle as a state machine

Macaca SHALL model application lifecycle transitions explicitly and reject invalid transitions with structured errors.

#### Scenario: Valid lifecycle transition emits audit data

- **WHEN** an application transitions from declared to initialized or initialized to started
- **THEN** the lifecycle state machine SHALL accept the transition
- **AND** it SHALL emit structured logs and trace/audit records containing application id, package id when available, ABI version, previous state, next state, session id when available, and trace id when available

#### Scenario: Invalid lifecycle transition is rejected

- **WHEN** an application attempts to start before initialization or resume without being paused
- **THEN** the lifecycle state machine SHALL reject the transition with a structured invalid-transition error
- **AND** the application SHALL NOT continue through the invalid lifecycle path

### Requirement: Macaca SHALL support checkpoint payloads for lifecycle recovery

Application ABI v0 SHALL support checkpoint payloads for pause, resume, shutdown, and upgrade flows without exposing internal runtime objects.

#### Scenario: Pause creates a portable checkpoint

- **WHEN** an ABI application is paused
- **THEN** the host SHALL be able to produce or preserve an `ApplicationCheckpoint` payload containing application identity, ABI version, lifecycle state, timestamp, and opaque application state
- **AND** the checkpoint SHALL NOT contain direct pointers, `Arc<AppState>`, provider clients, private keys, credentials, or non-serializable internal runtime handles

### Requirement: Macaca SHALL adapt YAML applications into ABI applications

Macaca SHALL provide a YAML application ABI adapter that maps existing YAML application manifests and package descriptors into Application ABI descriptors while preserving current YAML application behavior.

#### Scenario: YAML application maps to ABI descriptor

- **WHEN** an existing YAML application manifest is parsed
- **THEN** the adapter SHALL produce an ABI descriptor containing application id, application name, version, package/runtime reference, entry agent or entrypoint when declared, workflow references when declared, capabilities, allowed tools, required or optional services when available, required exports, and declared host imports
- **AND** the adapter SHALL NOT hardcode demo application names or application-specific workflow routing

#### Scenario: Existing YAML application still loads

- **WHEN** Route C regression scenario `RC-APP-001` runs
- **THEN** existing YAML application loading SHALL still work through the current runtime path
- **AND** the new ABI adapter SHALL NOT remove, block, or silently change current YAML application execution semantics

### Requirement: Macaca SHALL provide a non-executing WASM ABI loader stub

Macaca SHALL provide a WASM Application ABI metadata loader stub that can load package manifest metadata and ABI declaration metadata but cannot instantiate or execute WASM code in Phase 05.

#### Scenario: WASM metadata loads without execution

- **WHEN** a WASM component package declares Application ABI v0 metadata
- **THEN** the loader stub SHALL parse and expose package metadata and ABI declaration metadata
- **AND** it SHALL NOT instantiate, link, or execute WASM bytes

#### Scenario: WASM execution returns runtime unavailable

- **WHEN** execution is requested for a WASM component package before a WASM runtime exists
- **THEN** the loader stub SHALL return structured `RuntimeUnavailable` or equivalent ABI error
- **AND** it SHALL emit structured logs and trace/audit records explaining that execution is intentionally unavailable in Phase 05

### Requirement: Macaca SHALL expose SDK helpers for Application ABI v0

Macaca SHALL expose SDK helpers that let application authors build ABI declarations, lifecycle events, host commands, trace metadata, render requests, storage requests, and structured command results without depending on internal runtime crates.

#### Scenario: SDK application helper builds a host command

- **WHEN** an SDK consumer builds a `macaca:service/call` or `macaca:trace/emit` command
- **THEN** the helper SHALL produce a protocol-level ABI host command with command name, payload, trace context, metadata, and ABI version
- **AND** the helper SHALL NOT require importing `macaca-web`, frontend code, framework runner internals, concrete provider clients, or business workflow modules

### Requirement: Macaca SHALL trace and log Application ABI decisions

Macaca SHALL emit presentation-neutral trace/audit records and structured logs for ABI declaration parsing, adapter selection, lifecycle transition start/pass/reject, host import command dispatch, policy/permission boundary outcomes, task routing, trace routing, storage routing, UI/payment/service unavailable outcomes, checkpoint operations, and WASM runtime unavailable results.

#### Scenario: Rejected ABI operation is auditable

- **WHEN** ABI validation, lifecycle transition, or host import dispatch rejects an operation
- **THEN** trace/audit records SHALL include application id, package id when available, ABI version, operation/export/import name, lifecycle state when relevant, structured error code, trace id when available, session id when available, and policy/permission status when available
- **AND** logs SHALL NOT include secrets, provider credentials, private keys, payment credentials, raw encrypted package contents, or user-private payloads beyond bounded diagnostics

### Requirement: Macaca SHALL preserve Route C Phase 05 regression baselines

Macaca SHALL implement Phase 05 additively without regressing YAML application loading, `/api/chat/v2` session creation, or the existing goal pipeline.

#### Scenario: Phase 05 baseline checks pass

- **WHEN** Phase 05 verification runs
- **THEN** the implementation SHALL preserve regression matrix scenarios `RC-APP-001`, `RC-CHAT-001`, and `RC-GOAL-001`
- **AND** existing YAML application, trace, task board, resume, driver, skill/MCP, Web UI, and CLI behavior SHALL continue to compile and run through existing paths until explicitly migrated by later changes

### Requirement: Macaca SHALL document Application ABI code with detailed English comments

Macaca SHALL include detailed English comments in new Phase 05 Rust code explaining ABI contracts, lifecycle states, host facade routing, command/result models, YAML adapter behavior, WASM stub behavior, trace/audit behavior, policy-ready boundaries, and explicit non-goals.

#### Scenario: Maintainer can understand ABI invariants from comments

- **WHEN** a maintainer reads the new Application ABI modules
- **THEN** comments SHALL explain what each public type, trait, state transition, host command, adapter, and unavailable result represents
- **AND** comments SHALL explain how trace, audit, permissions, lifecycle, storage, service calls, and runtime-unavailable invariants are protected
- **AND** comments SHALL explain which future capabilities are intentionally not implemented in Phase 05
