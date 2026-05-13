## ADDED Requirements

### Requirement: Hardened Out-of-Process Provider
The system SHALL provide a runtime-host-owned hardened WASM provider that
executes through an out-of-process daemon transport while preserving the
provider-neutral runtime contract.

#### Scenario: Daemon execution succeeds
- **WHEN** the daemon is healthy and accepts a traced execution envelope
- **THEN** the provider SHALL return a sanitized response mapped to the existing runtime command result and SHALL emit provider, daemon, and lifecycle audit events

#### Scenario: Daemon execution fails closed
- **WHEN** the daemon is unavailable, unhealthy, overloaded, timed out, cancelled, crashed, or returns a malformed response
- **THEN** the provider SHALL fail closed with a stable reason code and sanitized diagnostic

### Requirement: Hardened Provider Ownership
The system SHALL treat hardened WASM execution as a runtime-host deployment
profile and SHALL NOT introduce new Application ABI semantics, kernel-owned
daemon lifecycle, or presentation-shell provider construction.

#### Scenario: Hardened provider dependency boundary is preserved
- **WHEN** the hardened provider is enabled
- **THEN** dependency boundary checks SHALL pass without adding a new Route C allowlist exception
