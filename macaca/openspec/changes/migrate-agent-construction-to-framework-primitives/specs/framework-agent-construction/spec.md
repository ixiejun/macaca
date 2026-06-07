## ADDED Requirements

### Requirement: Traced agent construction SHALL be based on macaca-agent core abstractions

The system SHALL provide a framework-level traced agent construction primitive that is built on top of the `macaca-agent` core abstractions instead of keeping the core construction flow owned by `macaca-web`.

#### Scenario: Web delegates construction to framework primitive

- **GIVEN** `macaca-web` needs to construct a coordinator, planner, or worker agent
- **WHEN** the construction path is migrated
- **THEN** the core traced agent assembly flow SHALL be executed through a framework-level construction primitive
- **AND** that primitive SHALL consume the `macaca-agent` service, capability, and lifecycle abstractions instead of re-defining parallel structures
- **AND** `macaca-web` SHALL only provide runtime adapters, session context, OS resources, and trace sinks

#### Scenario: Legacy web builder names remain compatible during migration

- **GIVEN** existing code calls `build_coordinator`, `build_worker_agent`, or other current traced builder facades
- **WHEN** the migration is applied
- **THEN** those facades SHALL remain callable during the migration window
- **AND** they SHALL delegate to the framework-level construction primitive

#### Scenario: AgentServices facade remains the service binding surface

- **GIVEN** the runtime has optional memory, IPC, or persistence resources available for an agent
- **WHEN** a traced agent build request is assembled
- **THEN** those resources SHALL be bound through `AgentServices`
- **AND** downstream construction code SHALL consume the `AgentServices` facade rather than raw optional service wiring

### Requirement: Agent build intent SHALL be explicit and role-compatible

The system SHALL model agent construction through explicit build intents so planner, coordinator, and worker differences are expressed declaratively instead of through scattered web-only helper logic.

#### Scenario: Planner decomposition uses explicit build intent

- **GIVEN** the planner is being constructed for goal decomposition
- **WHEN** the build request is assembled
- **THEN** the request SHALL carry an explicit decomposition intent
- **AND** the resulting prompt parts, tool visibility, trace context, and goal linkage SHALL remain compatible with the current behavior

#### Scenario: Worker execution uses explicit build intent

- **GIVEN** a worker is being constructed to execute a claimed task
- **WHEN** the build request is assembled
- **THEN** the request SHALL carry an explicit worker execution intent
- **AND** worker lifecycle tool suppression and task trace binding SHALL remain compatible with the current behavior

#### Scenario: Capability composition remains legacy-compatible

- **GIVEN** an agent build path currently produces a legacy externally visible capability list
- **WHEN** capabilities are assembled through the migrated construction primitive
- **THEN** the internal representation SHALL support `AgentCapabilitySet` or an equivalent `macaca-agent` capability abstraction
- **AND** the legacy externally visible capability output SHALL remain compatible with the current behavior

### Requirement: Task-side execution SHALL depend on stable framework contracts

The system SHALL let task-side orchestration depend on a stable execution contract instead of depending on web-specific builder helper names or implementation details.

#### Scenario: Task loop launches planner execution through contract

- **GIVEN** PlanLoop or its runtime consumer needs planner execution
- **WHEN** the execution is requested
- **THEN** the task-side code SHALL express the execution through a stable launcher or intent contract
- **AND** it SHALL NOT require knowledge of web-internal builder helper naming

#### Scenario: Task loop launches worker execution through contract

- **GIVEN** WorkerLoop or its runtime consumer needs worker execution
- **WHEN** the execution is requested
- **THEN** the task-side code SHALL express the execution through the same stable launcher or intent contract
- **AND** the resulting task status transitions and wake-up behavior SHALL remain compatible with the current behavior

#### Scenario: Lifecycle policy remains compatible in migrated build paths

- **GIVEN** the migrated build path needs lifecycle semantics for coordinator, planner, or worker agents
- **WHEN** the execution contract is exercised
- **THEN** the build path SHALL rely on the `macaca-agent` lifecycle abstraction or a compatible contract derived from it
- **AND** externally visible lifecycle behavior SHALL remain compatible with the current behavior

### Requirement: Trace and tool visibility SHALL remain compatible across the migration

The system SHALL preserve existing trace behavior and tool visibility while migrating construction ownership.

#### Scenario: Live trace behavior remains unchanged

- **GIVEN** coordinator, planner, and worker agents currently emit live trace events through traced builders
- **WHEN** agent construction is moved behind framework-level primitives
- **THEN** the same classes of trace events SHALL continue to be emitted
- **AND** live SSE delivery SHALL remain compatible with current frontend expectations

#### Scenario: Historical trace recovery remains unchanged

- **GIVEN** the browser refreshes and reloads a session after trace events were persisted
- **WHEN** the new framework-level construction primitive is used
- **THEN** EventLog payload shapes required for historical trace reconstruction SHALL remain compatible
- **AND** frontend history restoration SHALL continue to work without new event translation rules

#### Scenario: Tool visibility remains unchanged per intent

- **GIVEN** coordinator, planner decomposition, planner review, and worker execution each currently see a specific allowed tool set
- **WHEN** construction ownership is migrated
- **THEN** each intent SHALL continue to receive the same effective tool visibility rules as before
