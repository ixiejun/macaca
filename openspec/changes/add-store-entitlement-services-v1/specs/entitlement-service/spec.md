## ADDED Requirements

### Requirement: Macaca SHALL expose Entitlement as a provider-neutral system service

Macaca SHALL expose entitlement query, upsert, revoke, install authorization, start authorization, capability-call authorization, audit query, metering record, and snapshot operations through an Entitlement Service contract.

#### Scenario: Entitlement service authorizes paid package start

- **WHEN** a paid package start command includes package id, developer id, commerce metadata, operation, and trace context
- **THEN** Entitlement Service SHALL resolve the effective entitlement record
- **AND** it SHALL return an allow decision only when the entitlement state and policy allow the operation
- **AND** it SHALL append an auditable decision record

#### Scenario: Entitlement service denies invalid entitlement

- **WHEN** the effective entitlement is missing, expired, revoked, region-blocked, usage-exceeded, or unavailable
- **THEN** Entitlement Service SHALL return structured deny or unavailable state
- **AND** it SHALL log package id, developer id, operation, state, reason code, and trace id
- **AND** it SHALL NOT panic, hang, or silently allow the paid operation

### Requirement: Macaca SHALL preserve deterministic entitlement state precedence

Entitlement Service SHALL preserve Phase 08 deterministic entitlement precedence where revoked and other denial states override valid entitlement records according to the configured repository policy.

#### Scenario: Revoked entitlement overrides valid entitlement

- **WHEN** multiple entitlement records exist for one package and one effective record is revoked
- **THEN** Entitlement Service SHALL resolve the denied state according to repository precedence
- **AND** paid install/start/call authorization SHALL be denied unless a later explicit policy restores validity

### Requirement: Entitlement service SHALL emit metering and audit records for paid capability calls

Entitlement Service SHALL record metering and audit events for paid capability calls when commerce metadata requires metering.

#### Scenario: Metered capability call emits event

- **WHEN** a metered capability call is authorized
- **THEN** Entitlement Service SHALL record package id, developer id, application id when available, session id when available, capability id, operation, quantity, unit, decision state, timestamp, and trace id
- **AND** the metering record SHALL be compatible with existing trace/audit replay paths

### Requirement: Entitlement service SHALL protect encrypted package authorization

Entitlement Service SHALL provide an authorization path suitable for encrypted package decrypt/load hooks.

#### Scenario: Encrypted package decrypt is denied without entitlement

- **WHEN** encrypted package metadata is present and Entitlement Service denies or is unavailable for a paid/encrypted package
- **THEN** decrypt hooks SHALL NOT execute
- **AND** the caller SHALL receive structured deny or unavailable state

### Requirement: Entitlement service SHALL provide sanitized snapshots and audit pages

Entitlement Service SHALL expose snapshots and audit pages suitable for Web, CLI, SDK, diagnostics, and recovery.

#### Scenario: Entitlement snapshot is safe to display

- **WHEN** a snapshot or audit page is requested
- **THEN** the response SHALL include bounded identifiers, state counts, operation names, timestamps, trace ids, reason codes, and sanitized diagnostics
- **AND** it SHALL NOT include license secrets, credentials, API keys, private keys, encrypted payloads, raw package bytes, prompt bodies, or raw manifest bodies
