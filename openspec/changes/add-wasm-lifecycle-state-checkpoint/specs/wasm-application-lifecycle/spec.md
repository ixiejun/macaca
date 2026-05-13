## ADDED Requirements
### Requirement: WASM lifecycle transitions are explicit and fail closed
Macaca SHALL represent WASM application lifecycle movement through provider-neutral states and typed transition commands validated by a centralized state machine.

#### Scenario: Valid lifecycle transition
- **WHEN** a traced WASM session requests an allowed transition such as `validate` to `compile` or `start` to `handle_event`
- **THEN** the runtime SHALL update the lifecycle state and return a transition result containing from-state, to-state, trace id, reason code, and sanitized metadata.

#### Scenario: Invalid lifecycle transition
- **WHEN** a traced WASM session requests a transition that is not allowed by the state graph
- **THEN** the runtime SHALL fail closed with a structured result whose reason code is `invalid_transition`.

#### Scenario: Missing trace
- **WHEN** a WASM lifecycle command is missing trace context
- **THEN** the runtime SHALL reject the command before changing state.
