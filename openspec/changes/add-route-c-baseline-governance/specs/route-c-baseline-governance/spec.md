## ADDED Requirements

### Requirement: Macaca SHALL define Route C governance boundaries before implementation phases

Macaca SHALL provide documentation that classifies Route C capabilities into microkernel primitives, system services, plugins, optional modules, application framework, and presentation shells before later Route C implementation phases begin.

#### Scenario: Boundary document names core layers

- **WHEN** a contributor opens the Route C boundary document
- **THEN** the document SHALL identify kernel-owned primitives
- **AND** it SHALL identify service-owned replaceable capabilities
- **AND** it SHALL identify plugin and optional module boundaries
- **AND** it SHALL prohibit application-specific logic in kernel

### Requirement: Macaca SHALL define Route C regression scenarios

Macaca SHALL maintain a regression matrix that lists existing Agent OS behavior that later Route C phases must preserve.

#### Scenario: Regression matrix covers current execution chain

- **WHEN** a contributor opens the regression matrix
- **THEN** the matrix SHALL include YAML application loading
- **AND** it SHALL include `/api/chat/v2` session create/resume behavior
- **AND** it SHALL include goal planning, worker execution, review, and coordinator resume
- **AND** it SHALL include trace real-time push and historical replay
- **AND** it SHALL include task board session-scoped fetching

### Requirement: Macaca SHALL provide a reusable Route C phase template

Macaca SHALL provide a reusable implementation template that later Route C phases follow before code changes.

#### Scenario: Phase template enforces additive implementation

- **WHEN** a later Route C phase is started
- **THEN** the phase template SHALL require Superpowers brainstorm
- **AND** it SHALL require OpenSpec proposal/design/tasks/spec
- **AND** it SHALL require additive-first implementation
- **AND** it SHALL require targeted tests, integration smoke, GitNexus impact, detect_changes, and commit

### Requirement: Macaca SHALL maintain an automated no-network Route C baseline

Macaca SHALL provide an integration baseline that validates the current no-network autonomous pipeline and required governance coverage without live LLMs or external services.

#### Scenario: Baseline runs without network LLM

- **WHEN** `cargo test -p macaca-integration-tests route_c_baseline` is run
- **THEN** it SHALL execute a no-network pipeline baseline
- **AND** it SHALL fail if required Route C governance scenarios are missing from documentation

