## ADDED Requirements

### Requirement: Macaca SHALL expose Web3 capability through an optional Web3 Service

Macaca SHALL expose provider-neutral Web3 Service commands for availability, wallet list, signing request admission, transaction preparation, chain query, and Web3 service snapshot.

#### Scenario: Web3 service command set is available through ServiceRuntime

- **WHEN** the Web3 Service is registered and started
- **THEN** callers SHALL be able to dispatch availability, wallet list, signing request, transaction preparation, chain query, and snapshot commands through the system service path
- **AND** the service descriptor SHALL remain independent of concrete chain names, wallet names, RPC provider names, app names, workflow names, gateway names, driver names, model names, or business-specific routing

#### Scenario: Web3 service remains optional

- **WHEN** Web3 Service is unavailable, disabled, or not backed by a real provider
- **THEN** ordinary applications that do not declare Web3 capability SHALL continue to start and run
- **AND** Web3-specific mutating commands SHALL fail closed with structured diagnostics

### Requirement: Macaca SHALL require trace and admission before mutating Web3 commands execute

Web3 Service SHALL validate trace context, capability scope, provider availability, policy status, command bounds, and redaction rules before any mutating Web3 command reaches provider execution.

#### Scenario: Signing request without trace is rejected

- **WHEN** a signing request is submitted without `TraceContext`
- **THEN** Web3 Service SHALL reject the command before provider execution
- **AND** the rejection SHALL be structured and logged with bounded service and command identifiers

#### Scenario: Transaction preparation without capability is rejected

- **WHEN** transaction preparation is requested without an admitted Web3 capability scope
- **THEN** Web3 Service SHALL reject the command before provider execution
- **AND** it SHALL NOT create a fake transaction or provider artifact

### Requirement: Macaca SHALL provide unavailable and mock Web3 providers

Web3 Service SHALL provide an unavailable provider for absent-safe base OS behavior and a mock/dev provider for deterministic tests and development.

#### Scenario: Unavailable Web3 provider fails closed

- **WHEN** a mutating Web3 command is sent to the unavailable provider
- **THEN** the provider SHALL return structured unavailable diagnostics before any adapter execution
- **AND** it SHALL NOT claim real wallet, real signing, real transaction, or real chain capability

#### Scenario: Mock Web3 provider is visibly non-real-chain

- **WHEN** the mock/dev Web3 provider handles a command
- **THEN** its provider descriptor, logs, snapshots, and results SHALL mark the provider as mock-only or development-only
- **AND** they SHALL indicate that no real chain transaction, settlement, or execution proof occurred

### Requirement: Macaca SHALL keep Web3 provider descriptors provider-neutral

Web3 provider descriptors SHALL describe capability, availability, trust level, mock/dev status, redaction guarantees, and audit mode without hardcoding vendor, chain, application, workflow, or business-specific control flow.

#### Scenario: Provider descriptor is returned in snapshot

- **WHEN** a caller requests a Web3 Service snapshot
- **THEN** the snapshot SHALL include bounded provider descriptor diagnostics
- **AND** it SHALL NOT expose private keys, wallet secrets, provider credentials, raw RPC credentials, raw signed transactions, or unbounded provider payloads
