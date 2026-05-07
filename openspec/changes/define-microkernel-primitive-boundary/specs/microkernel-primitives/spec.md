## ADDED Requirements

### Requirement: Macaca SHALL define kernel primitive value objects in `macaca-proto`

Macaca SHALL define microkernel primitive value objects in `macaca-proto` so upper crates can share kernel-facing identities, descriptors, trace context, policy data, resource scopes, and structured primitive errors without depending on presentation or provider crates.

#### Scenario: Primitive types are protocol-level contracts

- **WHEN** a contributor imports the kernel primitive module from `macaca-proto`
- **THEN** the module SHALL expose `KernelServiceId`, `CapabilityId`, `CapabilityDescriptor`, `ServiceScope`, `TraceContext`, `PolicyRequest`, `PolicyDecision`, `ResourceScope`, and `KernelPrimitiveError`
- **AND** representative values SHALL serialize and deserialize through serde
- **AND** the module SHALL NOT depend on `macaca-web`, `macaca-app`, `macaca-framework`, or provider-specific crates

### Requirement: Macaca SHALL expose an additive microkernel facade in `macaca-kernel`

Macaca SHALL expose an additive kernel facade that groups capability registry, service registry, policy, trace, and resource primitives behind stable traits while preserving existing runtime behavior.

#### Scenario: Facade supports capability registration and lookup

- **WHEN** a capability descriptor is registered through the kernel facade
- **THEN** the same descriptor SHALL be queryable by `CapabilityId`
- **AND** missing capabilities SHALL return a structured primitive error or explicit absence result
- **AND** existing application execution flows SHALL continue to compile without requiring migration in this phase

### Requirement: Macaca SHALL model system services as discoverable descriptors, not concrete providers

Macaca SHALL model system services through service identity and scope descriptors so the kernel can discover services without implementing LLM, driver, gateway, skill, MCP, memory, persistence, payment, Store, Web3, or EVM providers.

#### Scenario: Service registry stores provider-neutral service entries

- **WHEN** a service entry is registered with a service id and service scope
- **THEN** the service registry SHALL allow lookup by service id
- **AND** the registry SHALL NOT require or name any concrete provider, application, driver, gateway, chain, or business workflow

### Requirement: Macaca SHALL provide replaceable policy evaluation through a strategy interface

Macaca SHALL provide a `PolicyEngine` strategy interface that evaluates `PolicyRequest` values and returns structured `PolicyDecision` values.

#### Scenario: Default allow policy preserves compatibility

- **WHEN** the default compatibility policy evaluates a valid policy request
- **THEN** it SHALL return a structured allow decision
- **AND** the implementation comments SHALL state that this permissive behavior exists only for additive compatibility

#### Scenario: Deny decisions are represented as data

- **WHEN** a deny policy evaluates a request
- **THEN** it SHALL return a structured deny decision with reason data
- **AND** callers SHALL NOT need to detect denial by panic, hang, or provider-specific string parsing

### Requirement: Macaca SHALL provide resource scope mediation without taking over existing resources

Macaca SHALL provide a resource manager primitive that registers and queries resource scopes such as workspace, browser, driver process, network, and storage without changing existing resource ownership behavior in this phase.

#### Scenario: Duplicate resource registration returns structured error

- **WHEN** the same resource scope is registered twice in the same manager
- **THEN** the second registration SHALL fail with a structured primitive error
- **AND** the manager SHALL keep the first registration intact

### Requirement: Macaca SHALL provide a trace event bus boundary for primitive operations

Macaca SHALL provide a trace event bus trait so primitive operations can emit trace/audit events without binding the kernel to SSE, EventLog, frontend state, or `macaca-web`.

#### Scenario: Trace boundary remains presentation-neutral

- **WHEN** kernel primitive code emits a trace event through the trace boundary
- **THEN** the event SHALL be expressed through a trait
- **AND** the kernel SHALL NOT depend on Web UI, SSE route handlers, or application-specific trace rendering

### Requirement: Macaca SHALL expose microkernel primitive access through `macaca-sdk`

Macaca SHALL provide an additive SDK-facing entry point for applications and tooling to discover kernel capabilities and services through the microkernel facade.

#### Scenario: SDK users can depend on facade primitives without `macaca-web`

- **WHEN** an SDK consumer imports the new facade access path
- **THEN** it SHALL be able to reference the primitive facade types without depending on `macaca-web`
- **AND** existing SDK APIs SHALL remain source-compatible in this phase

### Requirement: Macaca SHALL keep Route C Phase 01 additive and regression-safe

Macaca SHALL implement Phase 01 without changing current YAML application loading, goal planning, worker execution, review, coordinator resume, or no-network pipeline behavior.

#### Scenario: Route C baseline remains valid

- **WHEN** the Route C baseline integration test is run after Phase 01 implementation
- **THEN** the no-network baseline SHALL pass
- **AND** the implementation SHALL preserve regression matrix scenarios `RC-APP-001`, `RC-GOAL-001`, and `RC-PIPE-001`

### Requirement: Macaca SHALL prohibit application and provider hardcode in microkernel primitives

Macaca SHALL keep the new microkernel primitive code generic across all applications and providers.

#### Scenario: Primitive code contains no application-specific names

- **WHEN** the new `macaca-proto` and `macaca-kernel` primitive files are reviewed
- **THEN** they SHALL NOT hardcode application names, workflow names, driver names, gateway names, provider names, chain names, or business-specific routing
- **AND** any provider-specific behavior SHALL remain outside the microkernel primitive boundary

### Requirement: Macaca SHALL document new primitive code with detailed English comments

Macaca SHALL include detailed English comments in new Phase 01 Rust code to explain the purpose, operating model, and protected invariant of each primitive and skeleton implementation.

#### Scenario: Comments explain operating principles

- **WHEN** a maintainer reads the new primitive Rust files
- **THEN** comments SHALL explain what each primitive represents
- **AND** comments SHALL explain how callers should use it
- **AND** comments SHALL explain why compatibility defaults or skeleton implementations are intentionally limited
