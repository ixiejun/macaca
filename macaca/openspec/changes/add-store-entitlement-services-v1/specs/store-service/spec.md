## ADDED Requirements

### Requirement: Macaca SHALL expose Store as a provider-neutral system service

Macaca SHALL expose package inspect, resolve, install, status, and snapshot operations through a Store Service contract that is independent of concrete Store vendors, payment providers, package repositories, app names, workflow names, driver names, gateway names, model names, chain names, or business-specific routing.

#### Scenario: Store service returns sanitized package metadata

- **WHEN** a shell or SDK client inspects packages through Store Service
- **THEN** the response SHALL include provider-neutral package ids, versions, runtime kinds, license metadata, source status, install status, and diagnostics
- **AND** the response SHALL NOT include raw package bytes, raw manifest bodies, encrypted package bytes, credentials, API keys, private keys, or license secrets

#### Scenario: Store service is unavailable

- **WHEN** Store Service is missing, disabled, or unavailable
- **THEN** package inspect/status operations SHALL return structured unavailable state
- **AND** free/open package runtime paths SHALL remain eligible for existing local execution paths
- **AND** paid install operations SHALL NOT be silently treated as successful

### Requirement: Macaca SHALL route package install decisions through Store and Entitlement boundaries

Macaca SHALL use Store Service for package install orchestration and Entitlement Service for paid authorization decisions.

#### Scenario: Free or open package install remains allowed

- **WHEN** a package license is free/open and the package does not require Store entitlement
- **THEN** Store Service SHALL allow metadata-level install/status decisions without requiring a paid entitlement record
- **AND** the decision SHALL be traceable with package id, developer id, operation, status, and trace id

#### Scenario: Paid package install requires entitlement

- **WHEN** a paid, subscription, or metered package is installed through Store Service
- **THEN** Store Service SHALL delegate authorization to Entitlement Service or an equivalent service-backed entitlement authorizer
- **AND** missing, expired, revoked, region-blocked, usage-exceeded, or unavailable entitlement SHALL produce structured deny/unavailable state

### Requirement: Store service SHALL provide sanitized snapshots

Store Service SHALL expose snapshots that are safe for Web, CLI, SDK, diagnostics, and recovery.

#### Scenario: Store snapshot is safe to display

- **WHEN** Web or CLI requests a Store Service snapshot
- **THEN** the snapshot SHALL include counts, provider health, source ids, installed package metadata, last sync time when available, and sanitized diagnostics
- **AND** the snapshot SHALL NOT include package bodies, encrypted payloads, credentials, secrets, private keys, prompt bodies, or raw manifests

### Requirement: Store service operations SHALL be traceable and logged

Store Service mutating operations SHALL require trace context and SHALL emit structured logs at key execution nodes.

#### Scenario: Store install emits auditable logs

- **WHEN** a package install command is accepted, denied, unavailable, or fails
- **THEN** logs SHALL include service id, command name, trace id, package id, developer id, operation, status, and reason code
- **AND** logs SHALL omit raw package data and secrets
