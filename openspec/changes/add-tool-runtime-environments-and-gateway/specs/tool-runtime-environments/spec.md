## ADDED Requirements

### Requirement: Tool Runtime Environments Shall Be Provider-Backed

Macaca SHALL model tool runtime environments as provider-backed capabilities with health, cleanup, resource policy, artifact roots, process handles, network policy, filesystem policy, and secret injection policy.

#### Scenario: Environment is unavailable

- **GIVEN** a tool requires a sandbox environment
- **AND** no sandbox provider is available
- **WHEN** the tool plan or invocation is evaluated
- **THEN** Macaca SHALL return a structured unavailable diagnostic
- **AND** it SHALL NOT crash, hang, silently fall back, or fake success.

#### Scenario: Session environment is cleaned up

- **GIVEN** a session-scoped environment has active process handles and artifact roots
- **WHEN** the session ends or cleanup is requested
- **THEN** Macaca SHALL release the environment resources
- **AND** cleanup status SHALL be recorded as sanitized audit evidence.

### Requirement: Environment Policy Shall Run Before Side Effects

Macaca SHALL apply resource, filesystem, network, secret injection, entitlement, and metering policy before a tool uses an environment.

#### Scenario: Network egress is denied

- **GIVEN** a tool requests an environment with network egress
- **AND** policy denies network egress for the session
- **WHEN** invocation is evaluated
- **THEN** the tool SHALL fail with a structured denied result before environment use
- **AND** the audit event SHALL include a stable reason code without raw tool input.

### Requirement: Managed Gateway Shall Be Optional And Audited

Macaca SHALL support managed gateway providers as optional tool providers.

#### Scenario: Gateway routes a web extraction tool

- **GIVEN** a gateway provider registers a web extraction descriptor
- **WHEN** policy selects the gateway route
- **THEN** invocation SHALL pass through service policy, metering, and audit
- **AND** provider-specific names SHALL remain descriptor/config data rather than OS control-flow branches.

#### Scenario: Gateway is absent

- **GIVEN** a toolset includes gateway-backed media tools
- **AND** no gateway provider is configured
- **WHEN** the tool plan is built
- **THEN** those tools SHALL be hidden or summarized with structured unavailable diagnostics
- **AND** other available tools SHALL still be planned.

### Requirement: Environment And Gateway Logs Shall Be Sanitized

Runtime environment and managed gateway operations SHALL emit structured logs and audit records with bounded sanitized fields.

#### Scenario: Gateway invocation completes

- **WHEN** a gateway-backed invocation completes
- **THEN** logs or audit SHALL include trace id, provider id, tool id, status, latency, metering ref, input hash, output hash, and artifact refs when present
- **AND** logs SHALL NOT include raw secrets, env values, headers, raw provider payloads, or unbounded output.
