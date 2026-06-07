## ADDED Requirements

### Requirement: Web3/EVM services SHALL emit bounded trace and audit records

Web3/EVM services SHALL emit structured logs and trace/audit records for service lifecycle, provider selection, availability query, admission decision, command rejection, command completion, snapshot query, and failure nodes.

#### Scenario: Web3 command rejection is traceable

- **WHEN** Web3 Service rejects a signing request or transaction preparation command
- **THEN** it SHALL emit a bounded trace/audit record with service id, command name, trace id when available, status, and reason code
- **AND** it SHALL NOT include private keys, wallet secrets, raw signatures, raw signed transactions, provider credentials, raw RPC credentials, or unbounded user input

#### Scenario: EVM command completion is traceable

- **WHEN** EVM Service completes a mock/dev contract operation
- **THEN** it SHALL emit a bounded trace/audit record with service id, command name, trace id, provider class, operation status, and mock/dev diagnostics
- **AND** it SHALL NOT include raw ABI payloads, raw contract bytecode, raw signed transactions, provider credentials, raw provider responses, or unbounded user input

### Requirement: Web3/EVM services SHALL fail closed when trace is required but missing

Mutating Web3/EVM commands SHALL require `TraceContext` and SHALL be rejected before provider execution when trace is missing or invalid.

#### Scenario: Mutating command without trace is denied before provider dispatch

- **WHEN** signing request, transaction preparation, contract deploy, or contract call is submitted without valid `TraceContext`
- **THEN** the corresponding service SHALL reject the command before provider execution
- **AND** the rejection SHALL be logged with bounded diagnostics

### Requirement: Web3/EVM services SHALL redact snapshots and mementos

Web3/EVM service snapshots and operation mementos SHALL contain only bounded provider-neutral summaries, statuses, reason codes, diagnostics, and artifact digests or references.

#### Scenario: Snapshot omits secret and raw provider fields

- **WHEN** a caller requests Web3 or EVM service snapshot
- **THEN** the snapshot SHALL omit private keys, mnemonics, wallet secrets, raw signatures, raw signed transactions, raw RPC credentials, provider credentials, raw ABI payloads, raw bytecode, raw provider responses, prompt bodies, package bytes, and encrypted payload
- **AND** the snapshot SHALL preserve enough bounded diagnostics for audit and troubleshooting

### Requirement: Mock/dev providers SHALL be auditable as non-real-chain

Mock/dev Web3/EVM providers SHALL make non-real-chain behavior explicit in logs, trace/audit records, snapshots, provider descriptors, and returned diagnostics.

#### Scenario: Mock result cannot be mistaken for real execution

- **WHEN** a mock/dev Web3 or EVM provider returns a successful result
- **THEN** the result and trace/audit record SHALL identify the provider as mock-only or development-only
- **AND** they SHALL state or encode that no real chain signing, transaction broadcast, contract execution, settlement, or proof occurred
