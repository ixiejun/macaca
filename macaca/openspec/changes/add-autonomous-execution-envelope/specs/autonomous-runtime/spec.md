## ADDED Requirements

### Requirement: Autonomous Execution Envelope

Runtime Host SHALL compile heartbeat-agent and scheduled-agent-task dispatches
into a provider-neutral execution envelope before calling Agent Execution. The
envelope SHALL preserve the source instruction, source kind, instruction
priority, execution mode, completion policy, trace-safe metadata, and generic
evidence requirements. The compiler SHALL NOT require users to author typed
contracts and SHALL NOT branch on application-specific business semantics.

#### Scenario: Heartbeat dispatch requires artifact evidence
- **GIVEN** a heartbeat agent declaration carries `evidence.expected_artifact_path`
- **WHEN** runtime-host dispatches an accepted heartbeat wake to Agent Execution
- **THEN** the command includes an execution envelope with source kind `heartbeat_profile`
- **AND** its completion policy requires artifact evidence
- **AND** its instruction priority is task-over-persona

#### Scenario: Scheduled task dispatch preserves natural language
- **GIVEN** a scheduled-agent-task payload contains a natural-language prompt
- **WHEN** runtime-host resolves the payload and dispatches Agent Execution
- **THEN** the command includes an execution envelope with source kind `scheduled_agent_task`
- **AND** the original prompt is preserved as the source instruction
- **AND** no application-specific logic is required to compile the envelope

### Requirement: Envelope Rendering And Evidence

Agent Execution SHALL render the execution envelope as the highest-priority
delegated execution contract before ordinary delegated context. Successful wake
or dispatch SHALL NOT by itself imply task completion; completion SHALL continue
to require Agent Execution status and envelope-specific generic evidence
validation. A completion policy of `require_agent_result` SHALL require a
completed Agent Execution result with bounded result evidence. A completion
policy of `require_artifact` SHALL require completed Agent Execution plus
artifact evidence.

#### Scenario: Envelope is visible to the runtime agent
- **GIVEN** an Agent Execution command carries an execution envelope
- **WHEN** Agent Execution builds the runtime prompt
- **THEN** the rendered prompt identifies the envelope as the highest-priority delegated execution contract
- **AND** includes generic evidence requirements without exposing raw secrets

#### Scenario: Completion policy controls evidence gate
- **GIVEN** a scheduled or heartbeat dispatch compiled an execution envelope
- **WHEN** Agent Execution returns a completed result
- **THEN** Runtime Host evaluates the result against the envelope completion policy
- **AND** `require_agent_result` accepts bounded result evidence
- **AND** `require_artifact` rejects results without artifact evidence
