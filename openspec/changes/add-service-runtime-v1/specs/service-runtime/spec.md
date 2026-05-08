## ADDED Requirements

### Requirement: Macaca SHALL provide a host-owned ServiceRuntime facade

Macaca SHALL provide a `ServiceRuntime` facade owned by `macaca-runtime-host` that coordinates provider-neutral system service registration, lifecycle, calls, cleanup, events, and snapshots without making `macaca-kernel`, `macaca-web`, or `macaca-cli` own provider orchestration.

#### Scenario: ServiceRuntime registers a provider-neutral service

- **WHEN** a service provider factory returns a valid provider-neutral service descriptor and service instance
- **THEN** the runtime SHALL register the service under its descriptor id
- **AND** the runtime SHALL register a local service bus handler for that service id
- **AND** the runtime SHALL record a Registered lifecycle state
- **AND** the runtime SHALL emit a structured registration event

#### Scenario: Duplicate service registration is rejected

- **WHEN** a provider factory attempts to register a service id that is already registered
- **THEN** the runtime SHALL reject the registration with a structured error
- **AND** the runtime SHALL emit a structured failure event

### Requirement: Macaca SHALL create services through descriptor-driven provider factories

Macaca SHALL expose a descriptor-driven `ServiceProviderFactory` abstraction so built-in and future plugin-backed providers can be created without app, workflow, provider, model, driver, gateway, chain, or business hardcoding.

#### Scenario: Factory creates a service from neutral context

- **WHEN** the runtime invokes a provider factory
- **THEN** the factory SHALL receive provider-neutral runtime context
- **AND** the factory SHALL return an `Arc<dyn SystemService>` or structured error
- **AND** the factory SHALL NOT require callers to branch on concrete provider categories

### Requirement: Macaca SHALL manage service lifecycle through explicit runtime state

Macaca SHALL manage registered services through explicit lifecycle states for start, call, stop, cleanup, health, and failure.

#### Scenario: Service starts successfully

- **WHEN** a registered service is started with trace context
- **THEN** the runtime SHALL transition the service through Starting to Running
- **AND** the runtime SHALL call the underlying service start operation
- **AND** the runtime SHALL emit structured lifecycle events and logs

#### Scenario: Service stop and cleanup complete

- **WHEN** a running or stopped service is stopped and cleaned up
- **THEN** the runtime SHALL transition through Stopping, Stopped, CleaningUp, and CleanedUp as applicable
- **AND** the runtime SHALL emit structured lifecycle events and logs

#### Scenario: Provider operation fails

- **WHEN** a provider start, call, stop, cleanup, or health operation fails
- **THEN** the runtime SHALL record Failed state with a reason
- **AND** the runtime SHALL return a structured error
- **AND** the runtime SHALL emit a structured failure event

### Requirement: Macaca SHALL dispatch runtime service calls through the service bus

Macaca SHALL dispatch ServiceRuntime calls through `macaca-ipc` service bus routing rather than exposing direct provider calls to external callers.

#### Scenario: Runtime calls a service through the bus

- **WHEN** a caller invokes a registered running service with a traced `ServiceCommand`
- **THEN** the runtime SHALL build a service envelope
- **AND** the runtime SHALL evaluate runtime decorators before dispatch
- **AND** the runtime SHALL dispatch through the service bus
- **AND** the service bus SHALL route to the registered service handler
- **AND** the runtime SHALL return the structured service reply or structured error

#### Scenario: Unknown service call is rejected

- **WHEN** a caller invokes a service id not known to the runtime
- **THEN** the runtime SHALL reject the call with a structured unknown-service error
- **AND** the runtime SHALL emit a structured rejection event

### Requirement: Macaca SHALL enforce trace and policy decorators before service dispatch

Macaca SHALL use an ordered runtime decorator chain to enforce trace-required and policy-required admission before any service bus dispatch.

#### Scenario: Missing trace is rejected before dispatch

- **WHEN** a runtime service call does not contain trace context
- **THEN** the trace-required runtime decorator SHALL reject the call before service bus dispatch
- **AND** the underlying service SHALL NOT receive the command
- **AND** the runtime SHALL log and emit a structured rejection event

#### Scenario: Policy denial is rejected before dispatch

- **WHEN** the runtime policy strategy denies a service call
- **THEN** the policy decorator SHALL reject the call before service bus dispatch
- **AND** the underlying service SHALL NOT receive the command
- **AND** the runtime SHALL log and emit a structured policy-denial event

### Requirement: Macaca SHALL expose pluggable runtime policy and future decorator extension points

Macaca SHALL model runtime policy as a replaceable Strategy and SHALL provide extension points for future resource, entitlement, and metering decorators without requiring provider-specific code in S1.

#### Scenario: Runtime uses an allow policy

- **WHEN** the runtime is configured with an allow policy strategy
- **THEN** traced service calls SHALL proceed to service bus dispatch
- **AND** the runtime SHALL log the allow decision

#### Scenario: Runtime includes future decorator extension points

- **WHEN** maintainers add resource, entitlement, or metering enforcement in later phases
- **THEN** they SHALL be able to add decorators without changing provider factories or presentation shells

### Requirement: Macaca SHALL emit audit-friendly ServiceRuntime events and logs

Macaca SHALL emit structured runtime events and logs at key execution nodes for service lifecycle, calls, rejections, failures, policy decisions, and snapshots.

#### Scenario: Runtime emits lifecycle and call events

- **WHEN** a service is registered, started, called, stopped, cleaned up, rejected, or failed
- **THEN** the runtime SHALL emit events including service id, operation, lifecycle state, health when available, timestamp, trace id when available, and structured payload
- **AND** the runtime SHALL write structured logs for key execution nodes

### Requirement: Macaca SHALL expose deterministic runtime snapshots

Macaca SHALL expose deterministic `ServiceRuntimeSnapshot` data for diagnostics and future service inspector surfaces.

#### Scenario: Snapshot lists services deterministically

- **WHEN** a snapshot is requested
- **THEN** the runtime SHALL return service snapshots sorted by service id
- **AND** each service snapshot SHALL include descriptor, runtime lifecycle state, health, and failure reason when present

### Requirement: Macaca SHALL keep S1 additive and non-migrating

S1 SHALL add runtime infrastructure without migrating concrete providers, removing existing direct dependencies, or changing user-visible flows.

#### Scenario: Existing flows remain unchanged

- **WHEN** S1 is implemented
- **THEN** YAML application loading, `/api/chat/v2`, trace, task board, resume, driver, skill/MCP, Web UI, and CLI behavior SHALL continue through existing paths
- **AND** S1 SHALL NOT remove any S0 allowlist row unless the implementation genuinely removes that dependency debt
- **AND** S1 SHALL NOT introduce new kernel-to-provider, presentation-to-provider, or provider-to-presentation dependency violations

### Requirement: Macaca SHALL document ServiceRuntime governance

Macaca SHALL update Route C architecture governance documentation to describe the host-owned ServiceRuntime and its trace/policy/decorator requirements.

#### Scenario: Governance doc explains ServiceRuntime ownership

- **WHEN** maintainers read `macaca/docs/route-c-architecture-governance.md`
- **THEN** it SHALL state that `ServiceRuntime` is host-owned orchestration rather than kernel provider ownership
- **AND** it SHALL state that runtime service calls must pass trace and policy decorators
- **AND** it SHALL state that concrete provider migrations occur in later S phases
