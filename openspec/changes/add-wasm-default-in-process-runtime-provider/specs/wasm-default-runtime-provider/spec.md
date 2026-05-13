## ADDED Requirements

### Requirement: Default in-process WASM provider executes minimal exports

Macaca SHALL provide a default in-process WASM runtime provider that can compile, instantiate, and invoke minimal WASM module exports through the provider-neutral runtime provider contract.

#### Scenario: Provider executes a traced export
- **GIVEN** a session request has trace context, application id, ability id, profile, and a metadata-only artifact reference
- **AND** the artifact bytes compile and instantiate
- **WHEN** the host dispatches an invoke command for an exported function
- **THEN** the provider SHALL return a provider-neutral successful `ApplicationHostCommandResult`
- **AND** logs SHALL include trace id, application id, ability id, runtime kind, cache state, and artifact hash prefix.

#### Scenario: Provider remains replaceable
- **WHEN** SDK, Application Framework, Web, CLI, or proto callers inspect runtime state
- **THEN** they SHALL NOT see concrete engine types, concrete engine errors, raw WASM bytes, raw guest payloads, raw manifests, secrets, env values, API keys, prompts, private keys, stdout/stderr, or memory dumps.

### Requirement: Unavailable fallback remains fail-closed

Macaca SHALL keep the unavailable provider available when the default provider cannot be constructed or cannot execute.

#### Scenario: Default provider cannot execute
- **WHEN** default provider construction or availability fails
- **THEN** callers SHALL receive structured unavailable or rejected results with stable reason codes
- **AND** the system SHALL NOT silently report success.
