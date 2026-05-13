## ADDED Requirements

### Requirement: WASM service imports route through ServiceRuntime
Macaca SHALL route WASM service imports through host-owned `ServiceRuntime` and SHALL NOT let guests call concrete providers, registries, or backends directly.

#### Scenario: ServiceRuntime dispatch succeeds
- **GIVEN** a service import has trace context, target service id, operation, capability metadata, and bounded payload
- **WHEN** the target service is registered and policy allows the call
- **THEN** the bridge SHALL call `ServiceRuntime::call`
- **AND** return a provider-neutral application host command result.

#### Scenario: Service is unavailable
- **WHEN** the target service is not registered
- **THEN** the bridge SHALL return structured unavailable with reason code `service_unavailable`.
