## ADDED Requirements

### Requirement: Macaca SHALL define provider-neutral optional EVM/DApp contracts

Macaca SHALL define provider-neutral protocol contracts for EVM/DApp chain identity, contract addresses, ABI references, deploy commands, call commands, read commands, event subscriptions, gas estimates, transaction receipt lookups, module availability, contract events, and structured EVM errors.

#### Scenario: EVM fixtures round trip through serde

- **WHEN** deploy, call, read, subscription, gas estimate, receipt, availability, event, and error fixtures are serialized and deserialized
- **THEN** decoded contracts SHALL preserve chain id, request id, contract address when present, ABI reference, function reference, operation kind, gas policy, result status, timestamps, and metadata
- **AND** unknown/custom chain ids, ABI refs, function refs, account ids, contract ids, DApp metadata, and gas metadata SHALL remain structured without panic

#### Scenario: EVM contracts remain provider-neutral

- **WHEN** EVM contracts are consumed by kernel policy, application capability metadata, SDK facade code, trace/audit event code, or mock adapters
- **THEN** the contracts SHALL NOT depend on concrete chain names, node providers, wallet providers, token names, contract names, app names, workflow names, gateway names, driver names, model names, payment providers, or business-specific routing

### Requirement: Macaca SHALL keep EVM/DApp as an optional Web3 submodule

Macaca SHALL represent absent EVM through structured availability and unavailable errors, and SHALL NOT require EVM for base OS startup, ordinary application loading, Web3 v0 behavior, A2A Payment v0, task execution, or trace replay.

#### Scenario: EVM absent returns structured unavailable

- **WHEN** no EVM module is installed or registered
- **THEN** EVM availability SHALL return `unavailable`
- **AND** deploy, call, read, subscribe, estimate-gas, and receipt lookup requests SHALL return structured unavailable errors
- **AND** ordinary non-EVM application and task flows SHALL continue unaffected

#### Scenario: Base OS regression remains intact

- **WHEN** Route C baseline checks run without an EVM module, Substrate node, Frontier runtime, RPC endpoint, wallet, browser provider, or external network
- **THEN** `RC-APP-001` and `RC-TRACE-001` SHALL remain valid
- **AND** existing YAML applications, Web3 absence-safe behavior, and trace/event flows SHALL continue to compile and run through current paths

### Requirement: Macaca SHALL enforce policy before EVM deploy or state-changing call execution

Macaca SHALL evaluate module availability, signing policy, payment policy, gas policy, permission scope, and region/compliance policy before contract deploy or state-changing call adapters are allowed to execute.

#### Scenario: Deploy without required approval is denied before adapter execution

- **WHEN** a contract deploy request requires signing, payment, gas, or compliance approval and approval is missing
- **THEN** deploy SHALL be denied with a structured policy error
- **AND** no EVM adapter SHALL execute
- **AND** the denial SHALL be logged and auditable

#### Scenario: State-changing call cannot bypass signing/payment/gas policy

- **WHEN** a state-changing contract call is submitted
- **THEN** Macaca SHALL evaluate signing, payment, gas, permission, and compliance policy before adapter execution
- **AND** rejected policy SHALL return a structured EVM error
- **AND** rejected policy SHALL emit a trace/audit-compatible event

#### Scenario: Read-only operations still pass availability and compliance checks

- **WHEN** read, estimate-gas, receipt lookup, or subscription commands are submitted
- **THEN** Macaca SHALL evaluate module availability, permission scope, and compliance policy before adapter execution
- **AND** denied requests SHALL return structured policy errors without executing the adapter

### Requirement: Macaca SHALL provide pluggable EVM adapter and SDK facade boundaries

Macaca SHALL define replaceable adapter/facade boundaries for EVM providers and DApp callers without embedding provider behavior in kernel, application runtime, SDK, or web shell code.

#### Scenario: Future providers share one EVM service surface

- **WHEN** a future Substrate/Frontier adapter, EVM RPC adapter, local sandbox adapter, enterprise proxy, or third-party plugin implements the EVM adapter contract
- **THEN** callers SHALL use the same facade/service contract
- **AND** provider-specific transport, ABI encoding, receipt normalization, and subscription details SHALL remain behind the adapter/proxy boundary

#### Scenario: SDK facade constructs commands without owning providers

- **WHEN** SDK code exposes deploy, call, read, subscribe, estimate-gas, or receipt helper APIs
- **THEN** the SDK SHALL construct provider-neutral commands and delegate to the optional service boundary
- **AND** SDK code SHALL NOT instantiate concrete providers or bypass policy, trace, or audit paths

### Requirement: Macaca SHALL provide deterministic mock EVM behavior for tests only

Macaca SHALL provide a mock-only EVM adapter for deterministic no-network tests, and mock outputs SHALL be marked as simulated rather than real chain evidence.

#### Scenario: Mock deploy and call execute only after policy approval

- **WHEN** the mock EVM adapter receives deploy or state-changing call commands
- **THEN** the adapter SHALL execute only after required policy approval
- **AND** deploy SHALL return a simulated contract address
- **AND** state-changing call SHALL return simulated transaction/receipt data
- **AND** result metadata SHALL explicitly mark simulated provenance

#### Scenario: Mock read and gas estimate require no external network

- **WHEN** mock read, gas estimate, receipt lookup, or subscription behavior is exercised in tests
- **THEN** the mock path SHALL require no real EVM, Substrate node, Frontier runtime, RPC endpoint, wallet, browser provider, frontend server, or external network
- **AND** returned values SHALL be bounded and deterministic

### Requirement: Macaca SHALL emit trace and audit events for EVM lifecycle actions

Macaca SHALL emit structured logs and trace/audit-compatible events for EVM availability checks, deploy requests/results, call requests/results, read requests/results, subscription requests/events, gas estimates, receipt lookups, unavailable results, policy denials, and failures.

#### Scenario: EVM event contains auditable identity and scope

- **WHEN** an EVM lifecycle event is emitted
- **THEN** the event payload SHALL include chain id, operation, status, request id, contract address when available, transaction id or receipt id when available, session/task scope when available, timestamp, and error code when present
- **AND** the event SHALL be compatible with existing trace/event log paths

#### Scenario: EVM logs and events exclude sensitive material

- **WHEN** EVM availability, deploy, call, read, subscription, gas, receipt, unavailable, denial, or failure logs/events are emitted
- **THEN** logs and events SHALL NOT include private keys, seed phrases, credentials, raw encrypted payloads, provider secrets, raw unbounded ABI arguments, or unredacted signatures
- **AND** logs SHALL retain bounded identifiers needed for audit

### Requirement: Macaca SHALL expose DApp capability metadata without provider coupling

Macaca SHALL allow application/package metadata to declare optional DApp/EVM capability requirements such as `web3.evm`, and SHALL treat those declarations as metadata and policy input rather than provider instantiation instructions.

#### Scenario: Application declares optional EVM capability

- **WHEN** an application or package declares a DApp/EVM capability request
- **THEN** the declaration SHALL be represented as metadata and policy input
- **AND** application runtime SHALL NOT instantiate an EVM provider directly
- **AND** absence or denial SHALL be represented as structured availability data

#### Scenario: Web shell remains thin for DApp/EVM status

- **WHEN** EVM is unavailable or denied by policy
- **THEN** web-facing code MAY expose the status as shell data
- **AND** web-facing code SHALL NOT define EVM signing, payment, gas, contract execution, provider, or policy semantics

### Requirement: Macaca SHALL document Substrate/Frontier adapter ownership boundaries

Macaca SHALL document future Substrate/Frontier/EVM adapter ownership so provider-specific implementation can be added later without violating microkernel boundaries.

#### Scenario: Adapter boundary document separates responsibilities

- **WHEN** maintainers read the EVM adapter boundary document
- **THEN** it SHALL state that provider adapters own Substrate/Frontier/RPC mapping, provider-specific errors, ABI invocation encoding, receipt normalization, and subscription transport
- **AND** it SHALL state that kernel/service boundaries own registry, policy, availability, trace, and audit coordination only
- **AND** it SHALL state that application/SDK layers own command construction only and web shell owns display/approval surfaces only

### Requirement: Macaca SHALL provide detailed English comments and structured logs for Optional EVM/DApp v0 code

All new Phase 11 Rust code SHALL include detailed English comments and structured logs for key execution nodes.

#### Scenario: Maintainer can audit optional EVM decisions from code and logs

- **WHEN** a maintainer reads new EVM/DApp modules and observes runtime logs/events
- **THEN** comments SHALL explain public type/function purpose, optional-module behavior, adapter boundaries, policy rules, trace payloads, mock provenance, and non-goals
- **AND** logs SHALL capture availability, deploy, call, read, subscription, gas, receipt, unavailable, denial, and failure nodes without sensitive material
