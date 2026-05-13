## ADDED Requirements

### Requirement: WASM Runtime Observability Sinks
The system SHALL provide sanitized WASM runtime telemetry events and sink
interfaces for admission, provider selection, compile, instantiate, invoke,
resource decisions, host imports, lifecycle transitions, daemon health,
certification, and supply-chain verification.

#### Scenario: Runtime event is emitted
- **WHEN** a WASM runtime decision point completes, fails, rejects, times out, traps, or is unavailable
- **THEN** the configured telemetry sink SHALL receive a sanitized event with trace id, event kind, safe subject, reason code, status, duration where available, and redacted diagnostics

#### Scenario: Sensitive data is redacted
- **WHEN** runtime inputs contain raw payloads, guest bytes, memory, secrets, filesystem paths, environment values, network values, prompts, or API keys
- **THEN** telemetry SHALL NOT include those raw values and SHALL include only sanitized reason codes and safe metadata

### Requirement: Observability Governance Boundary
The system SHALL keep WASM observability as a provider-neutral Observer boundary
and SHALL NOT require Web, CLI, kernel, proto, app, or SDK layers to construct
provider-specific telemetry backends.

#### Scenario: Telemetry does not widen dependency boundaries
- **WHEN** WASM observability sinks are configured
- **THEN** Route C dependency checks SHALL pass without adding a new allowlist exception
