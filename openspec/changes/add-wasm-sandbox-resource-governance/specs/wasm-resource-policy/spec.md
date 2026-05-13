## ADDED Requirements

### Requirement: WASM resource policy is deterministic
Macaca SHALL define a provider-neutral WASM resource policy that captures memory, table, fuel, epoch, wall-clock, host import, payload, and concurrency limits without exposing a concrete engine.

#### Scenario: Policy merge is stable
- **GIVEN** platform defaults, deployment profile values, manifest requests, and explicit policy overrides
- **WHEN** Macaca merges the policies
- **THEN** the result SHALL be deterministic
- **AND** stricter numeric limits SHALL win unless an explicit override is supplied.

### Requirement: Runtime resource exhaustion is fail-closed
Macaca SHALL map resource limit violations into provider-neutral `resource_exhausted`, `timeout`, or `policy_denied` reason codes.

#### Scenario: Payload exceeds policy
- **WHEN** a WASM command payload exceeds the active policy limit
- **THEN** runtime dispatch SHALL return a structured failure with a stable reason code
- **AND** logs and reports SHALL NOT include the raw payload.
