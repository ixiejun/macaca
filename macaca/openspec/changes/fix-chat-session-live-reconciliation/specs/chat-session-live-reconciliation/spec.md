## ADDED Requirements

### Requirement: Live chat turns survive persisted session hydration

The frontend SHALL reconcile persisted session turns with newer live turns and SHALL NOT replace visible live assistant output with an older persisted snapshot.

#### Scenario: Session id hydration races with live output

- **WHEN** a chat stream has appended live assistant trace or content
- **AND** session id hydration fetches a persisted session snapshot that is less complete
- **THEN** the visible assistant turn SHALL retain the live trace or content
- **AND** the persisted snapshot SHALL still update session metadata

#### Scenario: Session stream refresh races with live output

- **WHEN** a session stream event schedules a persisted session refresh
- **AND** the fetched persisted turns are less complete than the current live turns
- **THEN** the frontend SHALL keep the more complete live assistant turn state visible

### Requirement: Plan decisions render from live session events

The frontend SHALL adapt live `plan_decision` stream events into generic coordinator trace steps without requiring an immediate persisted refresh.

#### Scenario: Plan decision arrives through session stream

- **WHEN** the frontend receives a `plan_decision` stream event for an active session
- **THEN** it SHALL append a `plan_decision` trace step to the latest assistant turn
- **AND** it SHALL preserve the event payload as opaque decision data

### Requirement: Reconciliation remains application-generic

The chat session reconciliation SHALL NOT depend on hardcoded workflow names, application names, agent names, or driver names.

#### Scenario: Different applications emit session events

- **WHEN** two applications emit different generic chat/session event payloads
- **THEN** reconciliation SHALL use only turn role, turn position, field completeness, and generic stream event type
