## ADDED Requirements

### Requirement: Agent service construction SHALL expose a canonical builder

The system SHALL provide an additive builder-style API for constructing `AgentServices` while preserving the existing direct field compatibility and no-op fallback behavior.

#### Scenario: Empty services preserve no-op behavior
- **GIVEN** an `AgentServices` value is constructed without concrete services
- **WHEN** callers access memory, IPC, or persistence through facade methods
- **THEN** no-op services SHALL be returned
- **AND** those no-op services SHALL NOT write memory, send IPC, write persistence, or alter agent output

#### Scenario: Builder services are effective
- **GIVEN** concrete memory, IPC, or persistence services are provided through the builder
- **WHEN** callers access them through facade methods
- **THEN** the effective services SHALL be the provided implementations

### Requirement: Agent capabilities SHALL have a stable primitive boundary

The system SHALL expose agent capability composite types through a dedicated primitive boundary while preserving existing flattened legacy output.

#### Scenario: Flattened capability output is unchanged
- **GIVEN** a `BasicAgent` is built with legacy capabilities
- **WHEN** capabilities are flattened for the legacy `Agent::capabilities` API
- **THEN** the visible capability list SHALL match the previous behavior

#### Scenario: Capability sources are inspectable
- **GIVEN** capabilities are grouped by source
- **WHEN** callers inspect the capability set
- **THEN** source metadata SHALL be available through read-only APIs
- **AND** callers SHALL NOT need to parse flattened capability names to infer source

### Requirement: Agent lifecycle transitions SHALL be preflightable

The system SHALL expose read-only lifecycle transition preflight using the same semantics as state mutation.

#### Scenario: Valid transition preflight matches mutation
- **GIVEN** a valid transition in the current lifecycle matrix
- **WHEN** the transition is checked through preflight
- **THEN** it SHALL be accepted
- **AND** executing the same transition SHALL still succeed

#### Scenario: Invalid transition preflight matches mutation
- **GIVEN** an invalid transition in the current lifecycle matrix
- **WHEN** the transition is checked through preflight
- **THEN** it SHALL be rejected
- **AND** executing the same transition SHALL still fail without changing state

## MODIFIED Requirements

### Requirement: macaca-agent refactor remains additive and behavior-compatible

The system SHALL keep `macaca-agent` primitive boundary refactors additive and SHALL NOT change existing agent execution, service fallback, lifecycle, capability, trace, session, task, planner, worker, coordinator, or application behavior.

#### Scenario: Existing imports remain compatible
- **GIVEN** upper crates import `AgentServices`, `AgentCapabilitySet`, `BasicAgentBuilder`, or `AgentLifecyclePolicy` from `macaca_agent`
- **WHEN** primitive modules are introduced
- **THEN** existing imports SHALL continue to compile through public re-exports

#### Scenario: Legacy direct interfaces remain discoverable
- **GIVEN** an old direct constructor or helper remains available for migration compatibility
- **WHEN** the primitive boundary refactor is applied
- **THEN** the old interface SHALL be marked deprecated
- **AND** the old interface SHALL continue to delegate to the new canonical path until follow-up consumer migrations remove call sites
