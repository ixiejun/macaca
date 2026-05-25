## ADDED Requirements

### Requirement: Governance Ledger Strategy

The Autonomy Evolution Control Plane SHALL define a replaceable Store/EventLog
governance ledger Strategy for sanitized evolution records, and local JSONL
storage SHALL be treated only as a development provider behind that Strategy.

#### Scenario: Provider restart replays records

- **GIVEN** a local development ledger provider with appended evolution records
- **WHEN** a new provider instance opens the same ledger path
- **THEN** replay SHALL return the same sanitized records in sequence order
- **AND** consumers SHALL NOT depend on local JSONL-specific APIs.

### Requirement: Versioned Migration And Malformed Record Handling

The governance ledger SHALL version records, migrate supported older schema
versions, and skip malformed records with bounded diagnostics instead of
crashing or faking success.

#### Scenario: Malformed record is skipped

- **GIVEN** a ledger file containing one malformed line and one valid record
- **WHEN** replay runs
- **THEN** the result SHALL include the valid record
- **AND** diagnostics SHALL report a skipped malformed record.

### Requirement: Compaction And Concurrency

The governance ledger SHALL preserve append ordering under concurrent appends
and SHALL compact retained records without changing their sequence numbers.

#### Scenario: Concurrent appends remain ordered

- **GIVEN** multiple concurrent append operations for the same ledger
- **WHEN** replay runs
- **THEN** records SHALL be returned in monotonic sequence order.

### Requirement: Sanitized Snapshots

The governance ledger SHALL expose bounded snapshots that exclude raw prompts,
raw provider payloads, manifests, package bytes, credentials, private keys, raw
signatures, and unbounded output.

#### Scenario: Forbidden text is sanitized

- **GIVEN** an append command containing forbidden raw payload markers in refs
- **WHEN** the record is appended and replayed
- **THEN** replay and snapshot output SHALL contain sanitized bounded refs only.
