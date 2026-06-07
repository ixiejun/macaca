## ADDED Requirements

### Requirement: Macaca SHALL keep kernel Web3/EVM APIs only as deprecated compatibility anchors

Existing kernel Web3/EVM facades and adapter helpers SHALL remain available but SHALL be marked deprecated once runtime-host service providers and SDK focused clients exist.

#### Scenario: Existing compatibility API remains searchable

- **WHEN** existing code references kernel Web3/EVM compatibility APIs
- **THEN** the APIs SHALL remain present so migration work can find old call sites
- **AND** deprecation guidance SHALL point new production consumers to `SystemWeb3Client` or `SystemEvmClient` through `ServiceRuntime`-backed `SystemFacade`

#### Scenario: Compatibility behavior is preserved

- **WHEN** existing Web3/EVM v0 tests exercise kernel compatibility facades
- **THEN** the existing absent-safe behavior SHALL remain intact
- **AND** the change SHALL NOT delete old facade semantics during S11

### Requirement: Macaca upper consumers SHALL migrate to SDK focused clients for new production paths

New Web, CLI, Gateway, Application Framework, and agent-facing production code SHALL consume Web3/EVM through SDK focused clients instead of kernel facades or provider implementations.

#### Scenario: Web status uses service-backed client or snapshot

- **WHEN** Web displays Web3/EVM status
- **THEN** it SHALL use SDK focused clients or service snapshots
- **AND** it SHALL NOT define chain, wallet, RPC, gas, signing, contract, provider-selection, payment, or application-specific semantics

#### Scenario: Application declares capability before using Web3/EVM

- **WHEN** an application or agent requests Web3/EVM operations
- **THEN** the request SHALL flow through capability and policy admission before provider execution
- **AND** application-specific code SHALL NOT bypass SDK clients to reach provider implementations

### Requirement: Macaca SHALL document S11 ownership in Route C governance

Route C governance and serviceization allowlist SHALL document that Web3/EVM provider execution belongs to runtime-host optional services and that kernel facades are deprecated compatibility anchors.

#### Scenario: Governance records remaining migration debt

- **WHEN** S11 implementation updates governance docs
- **THEN** the docs SHALL list Web3/EVM optional service ownership, upper-consumer SDK boundaries, default unavailable behavior, mock/dev provider restrictions, and remaining deprecated kernel anchors
- **AND** any temporary dependency allowlist entry SHALL include the replacement service path
