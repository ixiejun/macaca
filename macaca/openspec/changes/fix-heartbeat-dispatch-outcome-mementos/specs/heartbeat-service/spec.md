## MODIFIED Requirements
### Requirement: Heartbeat Service records replayable wake and dispatch mementos
The Heartbeat Service SHALL own heartbeat wake state, gate decisions, native
profile cadence state, and bounded run mementos. For accepted wakes that are
handed to an external runtime dispatch boundary, the Heartbeat Service SHALL NOT
mark the run terminally succeeded until a traced completion command reports the
dispatch outcome. Completion metadata SHALL be sanitized and SHALL NOT contain
raw prompts, raw provider output, manifests, package bytes, credentials, or
unbounded payloads.

#### Scenario: Accepted wake waits for dispatch completion
- **WHEN** a native heartbeat wake is accepted for delegated agent execution
- **THEN** the Heartbeat run SHALL enter a non-terminal dispatch-boundary state
- **AND** the run SHALL remain non-terminal until a traced completion command is recorded

#### Scenario: Failed dispatch is visible in run history
- **WHEN** Runtime Host reports a heartbeat dispatch failure for a run
- **THEN** the Heartbeat run memento SHALL record state `Failed`
- **AND** the memento metadata SHALL include a stable sanitized reason code

#### Scenario: Successful dispatch is visible in run history
- **WHEN** Runtime Host reports a heartbeat dispatch success with verified evidence
- **THEN** the Heartbeat run memento SHALL record state `Succeeded`
- **AND** the memento metadata SHALL include sanitized execution policy and evidence keys when available
