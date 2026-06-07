## ADDED Requirements

### Requirement: Macaca SDK SHALL expose focused Web3 and EVM clients

Macaca SDK SHALL expose `SystemWeb3Client` and `SystemEvmClient` as focused clients for upper consumers that need Web3 or EVM capability.

#### Scenario: Upper consumer obtains focused clients through SystemFacade

- **WHEN** Web, CLI, Gateway, Application Framework, or agent-facing code needs Web3 or EVM capability
- **THEN** it SHALL obtain `SystemWeb3Client` or `SystemEvmClient` through `SystemFacade` accessors
- **AND** it SHALL NOT construct runtime-host Web3/EVM providers or kernel Web3/EVM compatibility facades directly

#### Scenario: Focused clients hide provider implementation

- **WHEN** a caller uses `SystemWeb3Client` or `SystemEvmClient`
- **THEN** the caller SHALL interact with provider-neutral service commands and responses
- **AND** the caller SHALL NOT depend on concrete chain, wallet, RPC, EVM engine, or provider implementation types

### Requirement: SDK clients SHALL provide unavailable behavior

`SystemWeb3Client` and `SystemEvmClient` SHALL provide unavailable implementations that preserve absent-safe base OS behavior and fail closed for mutating commands.

#### Scenario: Web3 unavailable client rejects mutating command

- **WHEN** a signing request or transaction preparation is called through an unavailable `SystemWeb3Client`
- **THEN** the client SHALL return structured unavailable diagnostics
- **AND** it SHALL NOT claim wallet, signing, transaction, or chain capability

#### Scenario: EVM unavailable client rejects mutating command

- **WHEN** contract deploy or contract call is called through an unavailable `SystemEvmClient`
- **THEN** the client SHALL return structured unavailable diagnostics
- **AND** it SHALL NOT claim contract execution, chain receipt, or chain proof capability

### Requirement: SDK clients SHALL preserve trace and diagnostics

SDK Web3/EVM clients SHALL require or propagate trace context for mutating commands and SHALL preserve unavailable, disabled, policy-denied, mock/dev, and provider-error diagnostics.

#### Scenario: SDK preserves mock provider diagnostics

- **WHEN** a mock/dev Web3 or EVM provider returns a successful mock result
- **THEN** the SDK client SHALL expose diagnostics showing that the result is mock-only or development-only
- **AND** it SHALL NOT present the result as real chain execution, settlement, signing, or proof
