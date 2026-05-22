## MODIFIED Requirements

### Requirement: Lifecycle-managed Local Autonomy Supervisor

The system SHALL provide a lifecycle-managed autonomy supervisor that starts,
ticks, and stops only when local autonomy is enabled. Scheduler due-run dispatch
and Heartbeat native cadence SHALL remain separate lanes owned by Runtime Host
coordination, and long-running heartbeat agent execution SHALL NOT block
Scheduler due-run leasing or dispatch.

#### Scenario: Heartbeat agent dispatch runs longer than a scheduler tick

- **GIVEN** local autonomy is enabled
- **AND** a native heartbeat profile accepts a wake for a manifest-declared agent
- **AND** Agent Execution takes longer than one Scheduler tick interval
- **WHEN** the Heartbeat lane dispatches the agent work
- **THEN** the heartbeat tick records a bounded dispatch handoff
- **AND** the supervisor remains able to run Scheduler ticks while the agent execution continues
- **AND** no Scheduler job, due-run lease, or run history state is owned by Heartbeat

### Requirement: Autonomy Completion Requires Result Evidence

Runtime Host SHALL classify scheduled-agent-task and heartbeat-agent work as
successful only when Agent Execution returns a completed status together with
sanitized, replayable result evidence. A service-call success or
`agent.execute completed` status without evidence SHALL be treated as a
retryable or failed autonomy outcome, not as final success. A bounded output
hash MAY be used for audit correlation, but SHALL NOT be sufficient completion
evidence unless it is accompanied by durable artifact or audit evidence.

#### Scenario: Agent execution completes without evidence

- **GIVEN** a Scheduler run dispatches a scheduled-agent-task target
- **WHEN** Agent Execution returns a completed status without result evidence
- **THEN** Runtime Host records the dispatch as evidence-missing instead of succeeded
- **AND** the Scheduled Agent Task summary does not claim final success
- **AND** logs include the trace id and safe reason code without raw prompt content

#### Scenario: Agent execution only returns output hash

- **GIVEN** a heartbeat or scheduled-agent dispatch invokes Agent Execution
- **WHEN** Agent Execution returns a completed status with `result_output_hash` but no durable artifact or audit evidence
- **THEN** Runtime Host records the dispatch as evidence-missing instead of succeeded
- **AND** the bounded output hash remains available only as audit correlation metadata

#### Scenario: Agent execution completes with sanitized evidence

- **GIVEN** a heartbeat or scheduled-agent dispatch invokes Agent Execution
- **WHEN** Agent Execution returns a completed status with sanitized evidence metadata
- **THEN** Runtime Host may classify the dispatch as succeeded
- **AND** the audit chain can correlate the dispatch trace to the evidence reference

#### Scenario: Expected artifact path is declared

- **GIVEN** a heartbeat or scheduled-agent dispatch includes `evidence.expected_artifact_path` metadata
- **WHEN** Agent Execution observes a successful generic file-write tool result for a different path
- **THEN** the observed tool result is not emitted as completion evidence
- **AND** the dispatch remains evidence-missing without parsing prompt text or hardcoding application paths

#### Scenario: Expected artifact remains stale

- **GIVEN** a heartbeat or scheduled-agent dispatch includes `evidence.expected_artifact_path` metadata
- **AND** the expected artifact exists before Agent Execution starts
- **WHEN** Agent Execution completes without creating or changing that artifact
- **THEN** Runtime Host records the dispatch as evidence-missing instead of succeeded
- **AND** stale artifact mtime or size SHALL NOT satisfy completion evidence
