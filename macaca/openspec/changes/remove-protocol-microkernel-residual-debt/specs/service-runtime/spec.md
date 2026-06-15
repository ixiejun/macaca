## ADDED Requirements

### Requirement: ServiceRuntime SHALL Be Terminal Ownership Boundary

`ServiceRuntime` SHALL be the terminal ownership boundary for side-effecting service calls. It SHALL NOT expose deprecated public facades or migration-only managers as alternate invocation surfaces.

#### Scenario: Runtime call enters through typed service command
- **WHEN** any service capability is invoked by SDK, shell, application adapter, plugin, or host import
- **THEN** `ServiceRuntime.call` SHALL receive a typed `ServiceCommand`
- **AND** decorators SHALL enforce trace, policy, resource, entitlement, metering, audit, and redaction before provider side effects

#### Scenario: Deprecated runtime facade is rejected
- **WHEN** runtime-host production or integration-test source exposes or calls a deprecated public facade for MCP, entitlement, store, optional modules, or provider bootstrap
- **THEN** the runtime-host terminal gate SHALL fail with the replacement typed service client/provider command

### Requirement: Alert And Notification Delivery SHALL Be Serviceized

Alert delivery, webhook notification, remote notification, and unavailable alert behavior SHALL be owned by a system service provider registered through runtime-host, not by the microkernel.

#### Scenario: Webhook alert delivery runs as provider
- **WHEN** a webhook alert sink is configured
- **THEN** runtime-host SHALL register a provider-neutral alert/notification service provider
- **AND** the provider SHALL execute transport side effects only after trace and policy decorators allow the service command

#### Scenario: Alert provider is unavailable
- **WHEN** alert delivery is requested and no provider is registered or enabled
- **THEN** the service SHALL return structured unavailable
- **AND** trace/audit evidence SHALL include service id, command, trace id, and reason code without raw secrets or provider payloads

## REMOVED Requirements

### Requirement: Macaca SHALL keep S1 additive and non-migrating

**Reason**: This stage-specific requirement allowed infrastructure to coexist with direct paths. The terminal cleanup requires the service runtime to be the exclusive ownership boundary.

**Migration**: Replace this baseline rule with terminal ownership requirements and migrate all remaining direct callers before deleting their old APIs.

### Requirement: Macaca SHALL document ServiceRuntime governance

**Reason**: The referenced route-phase governance document is no longer the active terminal authority. Governance must point to stable microkernel/serviceization documents instead.

**Migration**: Update governance references to `macaca-os-architecture-governance.md`, `macaca-os-microkernel-boundaries.md`, and `macaca-os-serviceization-allowlist.md`.
