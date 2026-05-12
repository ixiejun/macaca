## ADDED Requirements

### Requirement: Provider-neutral WASM runtime contract

Macaca SHALL define a provider-neutral WASM Application Runtime Provider contract for descriptor discovery, availability reporting, execution profile declaration, session creation, and host import dispatch.

#### Scenario: Public contract remains engine-neutral
- **WHEN** Application Framework, SDK, Runtime Host, or a future provider consumes the WASM runtime provider contract
- **THEN** the public DTOs and trait signatures SHALL NOT expose concrete engine types, concrete provider names, app names, workflow names, driver names, gateway names, or business-specific routing.

#### Scenario: Provider creates a traced session
- **GIVEN** a session request includes trace context, application id, ability id, artifact reference, and execution profile
- **WHEN** the runtime provider creates a session
- **THEN** the session SHALL preserve trace id, runtime kind, descriptor, profile, resource envelope, and sanitized metadata.

### Requirement: Missing provider fails closed

Macaca SHALL provide an unavailable WASM runtime provider that represents absent optional runtime support as structured unavailable results.

#### Scenario: Unavailable provider rejects execution safely
- **WHEN** a WASM runtime provider is unavailable and a host command is dispatched
- **THEN** it SHALL return structured runtime-unavailable with trace id, runtime kind, reason code, and sanitized diagnostics
- **AND** it SHALL NOT compile, instantiate, or execute guest code.

#### Scenario: Missing trace is rejected
- **WHEN** a session request or host command lacks trace context
- **THEN** the runtime provider SHALL reject the request or return fail-closed without executing guest code.
