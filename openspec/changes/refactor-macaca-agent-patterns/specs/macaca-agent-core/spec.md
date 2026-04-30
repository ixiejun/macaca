## ADDED Requirements

### Requirement: Agent services facade preserves current behavior

The system SHALL provide facade-style accessors for agent services while preserving the existing behavior of `macaca-agent` service usage.

#### Scenario: Existing service remains accessible

- **GIVEN** an agent is constructed with an existing concrete service
- **WHEN** code accesses that service through the new `AgentServices` facade method
- **THEN** the returned service SHALL be the same effective service as before
- **AND** service calls SHALL preserve the previous side effects and outputs

#### Scenario: Missing service uses no-op behavior

- **GIVEN** an agent is constructed without an optional service
- **WHEN** code accesses that service through the new `AgentServices` facade method
- **THEN** the facade SHALL return a no-op service implementation
- **AND** the no-op service SHALL NOT write memory
- **AND** the no-op service SHALL NOT emit trace events
- **AND** the no-op service SHALL NOT change the agent response

### Requirement: BasicAgent builder remains backward compatible

The system SHALL add a builder-based construction path for `BasicAgent` without breaking the existing construction path.

#### Scenario: Existing constructor still works

- **GIVEN** existing code constructs a `BasicAgent` through the current constructor
- **WHEN** the builder refactor is applied
- **THEN** the existing constructor SHALL remain available
- **AND** it SHALL produce an agent with the same id, services, state, and capabilities as before

#### Scenario: Builder produces equivalent default agent

- **GIVEN** a `BasicAgentBuilder` receives the same inputs as the existing constructor
- **WHEN** the builder builds the agent
- **THEN** the resulting `BasicAgent` SHALL be behaviorally equivalent to the agent produced by the existing constructor

### Requirement: Agent lifecycle policy preserves state transition semantics

The system SHALL extract agent lifecycle transition rules into a policy abstraction while preserving the current state machine semantics.

#### Scenario: Valid transitions remain valid

- **GIVEN** a transition that is valid in the current `AgentStateMachine`
- **WHEN** the transition is evaluated through `AgentLifecyclePolicy`
- **THEN** the transition SHALL be accepted
- **AND** the resulting state SHALL match the previous state machine result

#### Scenario: Invalid transitions remain invalid

- **GIVEN** a transition that is invalid in the current `AgentStateMachine`
- **WHEN** the transition is evaluated through `AgentLifecyclePolicy`
- **THEN** the transition SHALL be rejected
- **AND** the current state SHALL remain unchanged as before

### Requirement: Agent capabilities support composite representation with legacy output

The system SHALL support a composite internal representation for agent capabilities while preserving the existing externally visible capability output.

#### Scenario: Legacy capability output is unchanged

- **GIVEN** an agent has a set of capabilities using the existing representation
- **WHEN** those capabilities are stored internally using the composite representation
- **THEN** flattening capabilities for the legacy API SHALL produce the same capability list as before

#### Scenario: Capability source can be represented internally

- **GIVEN** capabilities originate from different sources such as manifest, persona, skill, driver, or MCP
- **WHEN** those capabilities are added to the composite representation
- **THEN** the system SHALL preserve the source information internally
- **AND** the legacy flattened output SHALL remain compatible with existing callers

### Requirement: macaca-agent refactor remains additive and trace-safe

The system SHALL keep the `macaca-agent` refactor additive and SHALL NOT reduce Agent OS observability.

#### Scenario: Trace behavior is not changed

- **GIVEN** an agent execution path emits trace events before this refactor
- **WHEN** `AgentServices`, `BasicAgent`, state lifecycle, or capability internals are refactored
- **THEN** trace event names and payload schemas SHALL remain unchanged
- **AND** missing no-op services SHALL NOT emit fake trace events

#### Scenario: External agent execution semantics are preserved

- **GIVEN** an application uses existing coordinator, planner, or worker agent execution flows
- **WHEN** this refactor is applied
- **THEN** task creation, task claim, task review, coordinator resume, and agent execution results SHALL remain behaviorally unchanged
