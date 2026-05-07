## ADDED Requirements

### Requirement: Macaca SHALL define provider-neutral commerce and entitlement contracts

Macaca SHALL define provider-neutral protocol contracts for commerce metadata, entitlement identity/state, subscription plan metadata, metering records, and structured commerce errors.

#### Scenario: Commerce fixtures round trip through serde

- **WHEN** free/open/paid/subscription/metered commerce fixtures are serialized and deserialized
- **THEN** decoded contracts SHALL preserve license, entitlement, subscription, metering, and metadata fields
- **AND** unknown/custom license values SHALL remain structured without panic

#### Scenario: Commerce contracts remain provider-neutral

- **WHEN** commerce contracts are consumed by runtime guard and persistence layers
- **THEN** the contracts SHALL NOT depend on concrete store/payment provider names, chain names, or business-specific app/workflow routing

### Requirement: Macaca SHALL provide entitlement persistence contract with deterministic precedence

Macaca SHALL provide an entitlement persistence contract that supports upsert/query/audit and deterministic state precedence.

#### Scenario: Revoked entitlement overrides valid entitlement

- **WHEN** a previously valid entitlement is updated to revoked
- **THEN** entitlement queries SHALL resolve to revoked
- **AND** subsequent authorization checks SHALL deny paid operations unless policy explicitly restores validity

#### Scenario: Entitlement writes and reads are traceable

- **WHEN** entitlement snapshots or decision records are written/read
- **THEN** structured logs or trace/audit records SHALL include entitlement id, package/developer scope, operation, and timestamp

### Requirement: Macaca SHALL enforce runtime entitlement guard for commercial operations

Macaca SHALL enforce install/start/call authorization for commercial package/runtime paths through a runtime-host entitlement facade.

#### Scenario: Free or open package remains runnable

- **WHEN** a package is marked free/open and does not require store entitlement
- **THEN** install/start authorization SHALL allow execution through existing paths
- **AND** no paid-only entitlement denial SHALL be applied

#### Scenario: Paid package without entitlement is rejected

- **WHEN** a package is marked paid/subscription/metered and no valid entitlement is found
- **THEN** authorization SHALL reject install/start/call with structured deny reason
- **AND** rejection SHALL be logged and auditable

#### Scenario: Paid package with valid entitlement is authorized

- **WHEN** a paid/subscription/metered package has valid entitlement under current policy
- **THEN** authorization SHALL allow install/start/call
- **AND** allow decision SHALL be traceable with structured status

### Requirement: Macaca SHALL support encrypted skill loading hooks gated by entitlement

Macaca SHALL provide encrypted skill loading hooks where entitlement authorization is required before decrypt/load.

#### Scenario: Encrypted skill without entitlement is denied

- **WHEN** encrypted skill metadata is present and valid entitlement is missing/invalid
- **THEN** load SHALL be rejected with structured deny error
- **AND** decrypt hook SHALL NOT execute

#### Scenario: Encrypted skill with valid entitlement reaches decrypt hook

- **WHEN** encrypted skill metadata is present and entitlement is valid
- **THEN** load pipeline SHALL invoke decrypt hook abstraction
- **AND** decrypt failures SHALL return structured errors instead of panic or hang

### Requirement: Macaca SHALL emit metering events for paid capability calls

Macaca SHALL emit metering events for paid capability calls through existing trace/audit infrastructure.

#### Scenario: Metering event contains auditable identity context

- **WHEN** a paid capability call is authorized and executed
- **THEN** a metering event SHALL include app id, package id/version, developer id, session id when available, capability id, operation, decision status, and timestamp
- **AND** the event SHALL be compatible with existing trace/event log paths

### Requirement: Macaca SHALL preserve Route C regressions for Phase 08

Store/Entitlement v0 SHALL be additive and SHALL preserve baseline app, skill/MCP, and trace behaviors.

#### Scenario: Phase 08 regression checks pass

- **WHEN** Phase 08 verification runs
- **THEN** the implementation SHALL preserve `RC-APP-001`, `RC-SKILL-001`, and `RC-TRACE-001`
- **AND** existing YAML applications and `/api/chat/v2` flows SHALL continue to compile and run through current paths until explicitly migrated

### Requirement: Macaca SHALL provide detailed English comments and structured logs for Store/Entitlement v0 code

All new Phase 08 Rust code SHALL include detailed English comments and structured logs for key execution nodes.

#### Scenario: Maintainer can audit entitlement decisions from code and logs

- **WHEN** a maintainer reads new Store/Entitlement modules and observes runtime logs/events
- **THEN** comments SHALL explain public type/function purpose, decision rules, and runtime behavior
- **AND** logs SHALL capture validation/allow/deny/metering/decrypt nodes without secrets or private credentials
