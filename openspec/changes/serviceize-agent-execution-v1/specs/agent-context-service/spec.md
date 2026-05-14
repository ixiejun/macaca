## ADDED Requirements

### Requirement: Macaca SHALL provide an agent context construction service

Macaca SHALL provide `service.agent_context` to build trusted system context for agent execution, including persona, manifest semantics, capabilities, workspace guides, skill snapshots, MCP/tool catalog context, memory/context recall, and tool policy context.

#### Scenario: Context is built for a delegated agent

- **GIVEN** `service.agent_execution` needs to run a target agent
- **WHEN** it requests context construction
- **THEN** `service.agent_context` SHALL build the trusted system context for that app, session, task, target agent, and execution intent
- **AND** it SHALL emit a sanitized `AgentContextSnapshot`
- **AND** it SHALL NOT rely on Web shell code as the semantic owner of context rules.

### Requirement: Agent context SHALL include skill snapshot evidence

Macaca SHALL build and expose deterministic skill snapshot evidence for agent execution when Skill service capabilities are enabled or visible.

#### Scenario: Skill runtime is available

- **GIVEN** a target agent has visible skills
- **WHEN** `service.agent_context` builds context
- **THEN** it SHALL emit `skill_catalog_built`
- **AND** it SHALL emit `skill_snapshot_created`
- **AND** the snapshot SHALL include visible and filtered skill evidence without secrets or unbounded content.

#### Scenario: Skill runtime is unavailable

- **GIVEN** Skill service is absent or disabled
- **WHEN** `service.agent_context` builds context
- **THEN** it SHALL return structured unavailable or degraded context evidence
- **AND** it SHALL NOT crash, silently fake skill availability, or block unrelated context sources.

### Requirement: Agent context SHALL be provider-neutral and replayable

Macaca SHALL make context construction replayable through bounded snapshots and structured source metadata rather than shell-local mutable state.

#### Scenario: Session is refreshed

- **GIVEN** a browser or shell refreshes an existing session
- **WHEN** trace and event history are queried
- **THEN** the agent context snapshot reference SHALL allow audit replay to identify persona, skills, tool policy, and context sources used for the run.
