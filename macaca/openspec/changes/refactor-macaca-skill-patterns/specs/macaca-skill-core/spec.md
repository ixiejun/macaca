## ADDED Requirements

### Requirement: Additive skill exposure policy chain
The system SHALL provide a skill exposure policy chain that evaluates existing skill metadata gates without changing visible/filter behavior.

#### Scenario: Existing metadata gates are preserved
- **GIVEN** a skill requires a missing env var
- **WHEN** a snapshot is built through the refactored runtime
- **THEN** the skill is filtered with reason `missing_env`

### Requirement: Additive skill source factory
The system SHALL provide source factory primitives that produce workspace, application, user, bundled, and extra skill sources in the documented precedence order.

#### Scenario: Workspace source wins by precedence
- **GIVEN** duplicate skill names in workspace and application sources
- **WHEN** a snapshot is built
- **THEN** the workspace skill is selected

### Requirement: Executable skill registry snapshot
The system SHALL provide registry snapshot and reload primitives for executable skill definitions.

#### Scenario: Registry reload preserves executable skills
- **GIVEN** a registry containing two executable skill definitions
- **WHEN** the registry is snapshotted and reloaded into a new registry
- **THEN** both skill definitions are available by name

### Requirement: Skill tool adapter
The system SHALL provide an adapter/proxy boundary for executable skill tool calls while preserving existing shell/script/MCP behavior.

#### Scenario: Shell skill still executes through tool command executor
- **GIVEN** a shell executable skill
- **WHEN** it is exposed as a tool through the adapter
- **THEN** the tool command executor returns stdout, stderr, exit_code, and command fields as before

### Requirement: Skill runtime lifecycle handle
The system SHALL provide a runtime handle that represents skill lifecycle state without taking MCP lifecycle ownership away from the Agent OS MCP runtime.

#### Scenario: Provisioned skill handle records target client
- **GIVEN** a skill is provisioned to a client
- **WHEN** the additive handle API is used
- **THEN** the returned handle records the skill id, client id, target path, and `Provisioned` state
