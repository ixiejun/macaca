## ADDED Requirements

### Requirement: SDK SHALL Be Protocol-Client Backed Only

The SDK SHALL expose provider-neutral clients, typed commands, typed results, typed errors, `SystemFacade`, and unavailable/null-object clients. It SHALL NOT construct concrete providers, re-export provider/runtime-host/application/framework crates, or build in-process microkernel/framework calls for public API consumers.

#### Scenario: SDK public call uses protocol client
- **WHEN** a shell, gateway, plugin, application adapter, or external Rust consumer calls an SDK method for an OS capability
- **THEN** the SDK SHALL construct a provider-neutral command and call the protocol/service client path
- **AND** it SHALL NOT instantiate or re-export concrete runtime, framework, provider, application, database, wallet, chain, or transport implementations

#### Scenario: Provider re-export bridge is rejected
- **WHEN** SDK production source contains provider/runtime-host/application/framework crate alias re-exports or a shell provider bridge module
- **THEN** the SDK boundary gate SHALL fail with replacement focused-client guidance

### Requirement: SDK Public Surface SHALL Contain No Deprecated Wrappers

The SDK SHALL NOT retain deprecated aliases, compatibility wrappers, or allow-deprecated tests after terminal migration. Removed symbols SHALL be replaced by canonical focused clients or command DTOs.

#### Scenario: Deprecated SDK item is rejected
- **WHEN** SDK production or integration-test Rust source is scanned
- **THEN** zero `#[deprecated]` and zero `#[allow(deprecated)]` occurrences SHALL remain
- **AND** any caller of a removed SDK symbol SHALL be migrated before deletion

## REMOVED Requirements

### Requirement: SystemFacade SHALL delegate to clients and preserve compatibility

**Reason**: The terminal facade preserves stable response contracts, not old Rust wrappers or compatibility aliases.

**Migration**: Replace old callers with focused clients and provider-neutral DTOs, then delete deprecated wrappers and provider bridge modules.

### Requirement: S3 SHALL document SDK/SystemFacade governance

**Reason**: This phase-specific governance language keeps concrete provider migrations assigned to later phases. The terminal change has no later compatibility phase.

**Migration**: Archive phase language and document SDK as the stable protocol-client facade in the baseline specs.
