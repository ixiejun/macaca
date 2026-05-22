## ADDED Requirements

### Requirement: Gates Shall Reject Blocking Heartbeat Agent Dispatch

Serviceization gates SHALL reject production autonomy supervisor code that makes
Scheduler due-run dispatch wait for long-running heartbeat agent execution.
Heartbeat cadence and Scheduler dispatch may share a lifecycle-managed
supervisor, but heartbeat agent work must cross a bounded handoff boundary so
Scheduler run leasing continues.

#### Scenario: Heartbeat agent execution is awaited inside the scheduler loop

- **GIVEN** a change awaits long-running heartbeat Agent Execution in the same path that must advance Scheduler leases
- **WHEN** serviceization boundary gates inspect the autonomy supervisor
- **THEN** the gates fail with guidance to isolate heartbeat dispatch in Runtime Host

### Requirement: Gates Shall Reject Fake Autonomy Success

Serviceization gates SHALL reject autonomy dispatch code that records
scheduled-agent-task or heartbeat-agent success from a service-call success flag
or Agent Execution completed status alone. Completion must be correlated with
sanitized result evidence that can be replayed or audited. Natural-language
output hashes are audit correlation only and must not be accepted as completion
evidence by themselves.

#### Scenario: Completion status is used without evidence

- **GIVEN** a scheduled-agent-task or heartbeat-agent dispatch receives `agent.execute completed`
- **WHEN** no sanitized result evidence is present
- **THEN** the autonomy result is classified as evidence-missing instead of succeeded
- **AND** logs include a bounded reason code without raw prompts or provider payloads

#### Scenario: Output hash is treated as success evidence

- **GIVEN** a scheduled-agent-task or heartbeat-agent dispatch receives `agent.execute completed`
- **AND** the only metadata is `result_output_hash`
- **WHEN** serviceization boundary gates inspect the autonomy evidence policy
- **THEN** the gates fail with guidance to require artifact, audit, or explicit evidence references
