## ADDED Requirements

### Requirement: Macaca SHALL expose EVM capability through an optional EVM Service

Macaca SHALL expose provider-neutral EVM Service commands for availability, contract deploy admission, contract call admission, contract read admission, gas estimate, receipt query, event subscription admission, and EVM service snapshot.

#### Scenario: EVM service command set is available through ServiceRuntime

- **WHEN** the EVM Service is registered and started
- **THEN** callers SHALL be able to dispatch availability, contract deploy, contract call, contract read, gas estimate, receipt query, event subscription, and snapshot commands through the system service path
- **AND** the service descriptor SHALL remain independent of concrete EVM engine names, RPC provider names, chain names, app names, workflow names, gateway names, driver names, model names, or business-specific routing

#### Scenario: EVM service remains optional

- **WHEN** EVM Service is unavailable, disabled, or not backed by a real provider
- **THEN** ordinary applications that do not declare EVM capability SHALL continue to start and run
- **AND** EVM-specific mutating commands SHALL fail closed with structured diagnostics

### Requirement: Macaca SHALL require trace and admission before mutating EVM commands execute

EVM Service SHALL validate trace context, capability scope, provider availability, policy status, command bounds, and redaction rules before any mutating EVM contract deploy or call reaches provider execution.

#### Scenario: Contract deploy without trace is rejected

- **WHEN** a contract deploy command is submitted without `TraceContext`
- **THEN** EVM Service SHALL reject the command before provider execution
- **AND** the rejection SHALL be structured and logged with bounded service and command identifiers

#### Scenario: Contract call without capability is rejected

- **WHEN** contract call is requested without an admitted EVM capability scope
- **THEN** EVM Service SHALL reject the command before provider execution
- **AND** it SHALL NOT create a fake transaction receipt or provider artifact

### Requirement: Macaca SHALL provide unavailable and mock EVM providers

EVM Service SHALL provide an unavailable provider for absent-safe base OS behavior and a mock/dev provider for deterministic tests and development.

#### Scenario: Unavailable EVM provider fails closed

- **WHEN** a mutating EVM deploy or call command is sent to the unavailable provider
- **THEN** the provider SHALL return structured unavailable diagnostics before any adapter execution
- **AND** it SHALL NOT claim real EVM execution, settlement, receipt, or chain proof capability

#### Scenario: Mock EVM provider is visibly non-real-chain

- **WHEN** the mock/dev EVM provider handles a command
- **THEN** its provider descriptor, logs, snapshots, and results SHALL mark the provider as mock-only or development-only
- **AND** they SHALL indicate that no real chain execution, settlement, or execution proof occurred

### Requirement: Macaca SHALL protect EVM payloads from trace and memento leakage

EVM Service SHALL keep raw ABI payloads, raw contract bytecode, raw signed transactions, private keys, wallet secrets, provider credentials, and unbounded provider responses out of DTOs intended for logs, trace, snapshots, and mementos.

#### Scenario: Snapshot redacts EVM provider payloads

- **WHEN** a caller requests an EVM Service snapshot after contract operations
- **THEN** the snapshot SHALL include only bounded operation summaries, statuses, reason codes, and artifact digests or references
- **AND** it SHALL NOT expose raw ABI payloads, raw bytecode, raw signed transactions, secrets, or raw provider responses
