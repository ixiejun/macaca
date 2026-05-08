## ADDED Requirements

### Requirement: Macaca SHALL define provider-neutral optional Web3 contracts

Macaca SHALL define provider-neutral protocol contracts for Web3 wallet identity, chain identity, address values, capability kinds, module availability, signing requests, signing policies, signing decisions, signing proofs, transaction requests, transaction receipts, chain queries, chain query responses, and structured Web3 errors.

#### Scenario: Web3 fixtures round trip through serde

- **WHEN** availability, signing, transaction, chain query, receipt, proof, and error fixtures are serialized and deserialized
- **THEN** decoded contracts SHALL preserve wallet id, chain id, address, capability, operation, request id, amount/value fields, receipt/proof fields, status, timestamps, and metadata
- **AND** unknown/custom chain ids, wallet ids, asset codes, method names, network identifiers, and metadata SHALL remain structured without panic

#### Scenario: Web3 contracts remain provider-neutral

- **WHEN** Web3 contracts are consumed by kernel policy, IPC, app capability metadata, web thin-shell code, or mock adapters
- **THEN** the contracts SHALL NOT depend on concrete chain names, node providers, wallet providers, token names, app names, workflow names, gateway names, driver names, model names, payment providers, or business-specific routing

### Requirement: Macaca SHALL keep Web3 as an optional module

Macaca SHALL represent absent Web3 through structured availability and unavailable errors, and SHALL NOT require Web3 for base OS startup, ordinary application loading, task execution, A2A Payment v0, or trace replay.

#### Scenario: Web3 absent returns structured unavailable

- **WHEN** no Web3 module is installed or registered
- **THEN** Web3 availability SHALL return `unavailable`
- **AND** signing, transaction, and chain-query requests SHALL return structured unavailable errors
- **AND** ordinary non-Web3 application and task flows SHALL continue unaffected

#### Scenario: Base OS regression remains intact

- **WHEN** Route C baseline checks run without a Web3 node, wallet, RPC endpoint, EVM runtime, or external network
- **THEN** `RC-APP-001` and `RC-TRACE-001` SHALL remain valid
- **AND** existing YAML applications and trace/event flows SHALL continue to compile and run through current paths until explicitly migrated

### Requirement: Macaca SHALL enforce policy before Web3 signing or transaction execution

Macaca SHALL evaluate module availability, explicit approval, permission scope, fee/network policy, and region/compliance policy before signing or transaction adapters are allowed to execute.

#### Scenario: Signing request without approval is denied before adapter execution

- **WHEN** a signing request requires explicit approval and approval is missing
- **THEN** signing SHALL be denied with a structured policy error
- **AND** no signing adapter SHALL execute
- **AND** the denial SHALL be logged and auditable

#### Scenario: Region-blocked Web3 call is denied consistently

- **WHEN** Web3 availability or policy marks the current scope as `region_blocked`
- **THEN** signing, transaction, and chain-query requests SHALL return structured policy-denied errors
- **AND** the denied call SHALL emit a trace/audit-compatible event

#### Scenario: Mock Web3 execution still requires policy approval

- **WHEN** a mock signing or transaction adapter is used for deterministic tests
- **THEN** the adapter SHALL execute only after policy approval
- **AND** the mock path SHALL require no real private key, node, wallet, RPC endpoint, EVM runtime, browser, frontend server, or external network

### Requirement: Macaca SHALL provide pluggable Web3 adapter and proxy boundaries

Macaca SHALL define replaceable adapter/proxy boundaries for wallet, signing, transaction, and chain-query providers without embedding concrete provider behavior in kernel, app runtime, or web shell.

#### Scenario: Local and remote providers share one service surface

- **WHEN** a future local node provider or remote RPC provider implements the Web3 adapter contract
- **THEN** callers SHALL use the same facade/service contract
- **AND** transport-specific details SHALL remain behind the adapter/proxy boundary

#### Scenario: Missing provider does not panic

- **WHEN** a requested Web3 capability has no provider implementation
- **THEN** Macaca SHALL return structured unavailable
- **AND** the unavailable result SHALL be logged and traceable

### Requirement: Macaca SHALL emit trace and audit events for Web3 lifecycle actions

Macaca SHALL emit structured logs and trace/audit-compatible events for availability checks, signing requests, signing decisions, transaction requests, transaction receipts, chain queries, unavailable results, policy denials, and failures.

#### Scenario: Web3 event contains auditable identity and scope

- **WHEN** a Web3 lifecycle event is emitted
- **THEN** the event payload SHALL include wallet id, chain id, capability, operation, status, request id, transaction id or receipt id when available, session/task scope when available, timestamp, and error code when present
- **AND** the event SHALL be compatible with existing trace/event log paths

#### Scenario: Web3 logs and events exclude sensitive material

- **WHEN** Web3 availability, signing, transaction, chain-query, unavailable, denial, or failure logs/events are emitted
- **THEN** logs and events SHALL NOT include private keys, seed phrases, credentials, raw encrypted payloads, provider secrets, or unredacted raw signatures
- **AND** logs SHALL retain bounded identifiers needed for audit

### Requirement: Macaca SHALL expose optional Web3 data through IPC, app, and web thin-shell boundaries

Macaca SHALL expose Web3 availability and request/denial metadata through IPC, application capability metadata, and web thin-shell surfaces without defining provider-specific Web3 semantics in presentation or application framework code.

#### Scenario: Application declares optional Web3 capability

- **WHEN** an application or package declares a Web3 capability request
- **THEN** the declaration SHALL be represented as metadata and policy input
- **AND** application runtime SHALL NOT instantiate a Web3 provider directly

#### Scenario: Web thin shell displays unavailable or denied state as data

- **WHEN** Web3 is unavailable or denied by policy
- **THEN** `macaca-web` SHALL be able to expose the status as shell data
- **AND** `macaca-web` SHALL NOT define Web3 signing, transaction, chain-query, provider, or policy semantics

### Requirement: Macaca SHALL provide detailed English comments and structured logs for Optional Web3 Node v0 code

All new Phase 10 Rust code SHALL include detailed English comments and structured logs for key execution nodes.

#### Scenario: Maintainer can audit optional Web3 decisions from code and logs

- **WHEN** a maintainer reads new Web3 modules and observes runtime logs/events
- **THEN** comments SHALL explain public type/function purpose, optional-module behavior, adapter boundaries, policy rules, trace payloads, and non-goals
- **AND** logs SHALL capture availability, signing, transaction, chain-query, unavailable, denial, and failure nodes without sensitive material
