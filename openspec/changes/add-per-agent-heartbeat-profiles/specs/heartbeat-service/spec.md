## MODIFIED Requirements

### Requirement: Heartbeat Profiles

The Heartbeat Service SHALL expose provider-neutral heartbeat profiles that may
represent global, application, session, or application-agent scopes. Each
profile SHALL own its fixed interval cadence, optional cooldown policy,
enablement, bounded metadata, run mementos, and safe summary state. Missing
cooldown policy SHALL fall back to the provider default.

#### Scenario: Agent heartbeat profile is inspected
- **GIVEN** an application declares two heartbeat agents
- **WHEN** runtime-host registers native Heartbeat profiles
- **THEN** Heartbeat exposes two distinct profile summaries
- **AND** each summary has a distinct profile id and wake scope key
- **AND** each summary exposes its fixed interval and cooldown policy without raw manifest content

#### Scenario: Agent heartbeat profile is edited
- **GIVEN** an operator edits one agent heartbeat profile through a traced Heartbeat command
- **WHEN** the command changes fixed interval or cooldown policy
- **THEN** Heartbeat updates only that profile
- **AND** returns a mutation result with trace and audit identifiers
- **AND** other agent profiles for the same application are unchanged
