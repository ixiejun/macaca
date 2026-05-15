## MODIFIED Requirements

### Requirement: Session runtime activity visibility

The system SHALL keep session-visible runtime progress and app-declared agent activity consistent for both framework sessions and agentless WASM host-dispatch sessions.

#### Scenario: Agentless WASM host dispatch is running

- **WHEN** a WASM application session starts through the agentless host-dispatch path
- **THEN** the app entry agent activity is marked `Working`
- **AND** the activity update does not depend on an application-specific name, provider name, symbol, prompt, or domain payload.

#### Scenario: Agentless WASM host dispatch reaches a terminal result

- **WHEN** host dispatch returns a terminal result
- **THEN** the app entry agent activity is marked `Idle`
- **AND** the session snapshot remains the source for the terminal success, unavailable, unsupported, denied, or rejected status details.

#### Scenario: Agentless WASM host dispatch fails

- **WHEN** host dispatch returns an execution error
- **THEN** the app entry agent activity is marked `Error`
- **AND** the error detail is bounded to the structured dispatch error message.

#### Scenario: Delegated agent work starts from any chat runtime

- **WHEN** the application executor emits a delegated task start event for an app-declared agent
- **THEN** that target agent activity is marked `Working`
- **AND** the activity update is derived from generic executor event identity rather than an application-specific branch.

#### Scenario: Delegated agent work reaches a terminal event

- **WHEN** the application executor emits delegated task completion or failure
- **THEN** that target agent activity is marked `Idle` for successful completion
- **AND** that target agent activity is marked `Error` for failed completion or task failure.
