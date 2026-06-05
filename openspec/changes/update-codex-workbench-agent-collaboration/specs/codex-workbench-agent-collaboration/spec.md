## ADDED Requirements

### Requirement: Model-Decided Workbench Collaboration

The Codex WASM Workbench application SHALL delegate production task execution through coordinator, planner, coder, and reviewer agents, with task complexity and collaboration depth decided by the coordinator model from the task context rather than by hardcoded application, language, keyword, or business-domain rules.

#### Scenario: Complex task receives full collaboration

- **WHEN** a user starts a Workbench task that the coordinator model judges to require deep collaboration
- **THEN** the coordinator SHALL emit a collaboration plan explaining the model judgment
- **AND** the planner SHALL consume the coordinator output before producing a plan
- **AND** the coder SHALL consume the coordinator and planner outputs before writing artifacts
- **AND** the reviewer SHALL consume coordinator, planner, and coder outputs before producing review findings

#### Scenario: Simple task remains lightweight without hardcoded rules

- **WHEN** a user starts a Workbench task that the coordinator model judges to be simple
- **THEN** the same four application-owned agents SHALL still participate in the traceable handoff chain
- **AND** the coordinator output SHALL instruct downstream agents to keep planning and review lightweight
- **AND** Macaca OS SHALL NOT branch on task language, application name, workflow name, or business-domain keywords to decide that complexity

### Requirement: Application-Owned Collaboration Boundary

The Workbench collaboration behavior SHALL live in the application package and use generic Macaca host-command, service-call, trace, policy, and event boundaries without adding Codex-specific semantics below the application layer.

#### Scenario: Runtime executes generic commands

- **WHEN** the WASM runtime executes the Workbench component metadata
- **THEN** each collaboration step SHALL be a generic `agent_delegate` host command with bounded metadata and trace context
- **AND** prior agent outputs SHALL be passed through generic host-command result placeholders
- **AND** runtime-host, service runtime, SDK, Web shell, and frontend code SHALL NOT contain Workbench-specific complexity heuristics
