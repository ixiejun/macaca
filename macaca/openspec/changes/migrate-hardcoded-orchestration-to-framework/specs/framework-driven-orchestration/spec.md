## ADDED Requirements

### Requirement: Framework-Driven Agent Execution

The system SHALL use `macaca-framework` as the default execution substrate for application agents instead of maintaining parallel business execution paths through legacy orchestration code.

#### Scenario: New agent execution uses framework path
- **GIVEN** an application agent is invoked by the system
- **WHEN** the execution path is selected
- **THEN** the agent runs through framework-backed execution primitives
- **AND** the system does not require a separate legacy business execution path for that invocation

### Requirement: Capability-Driven Tool Policy

The system SHALL bind tools to agents through capability/config-driven policy rather than hardcoded agent-name branches.

#### Scenario: Tool access is derived from policy, not role name
- **GIVEN** an application defines agent capabilities or policy metadata
- **WHEN** the system builds that agent's toolset
- **THEN** tool availability is derived from declared policy
- **AND** the system does not require hardcoded checks for specific agent names

### Requirement: Application-Specific Orchestration Is Not Embedded in OS Substrate

The OS substrate SHALL not embed application-specific orchestration discipline such as fixed role ordering for a specific app.

#### Scenario: App-specific dependency rules live outside OS substrate
- **GIVEN** an application requires a specific task ordering discipline
- **WHEN** that discipline is enforced
- **THEN** it is expressed through application workflow/policy or declarative dependency metadata
- **AND** the core task substrate remains reusable for other applications with different roles
