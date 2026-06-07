## ADDED Requirements
### Requirement: Runtime harness models guest SDK facade calls
Macaca SHALL provide a runtime-scoped local harness that models guest SDK facade calls as provider-neutral Application ABI host commands.

#### Scenario: Service proxy command
- **WHEN** a test uses the harness service proxy to call a service operation
- **THEN** the harness SHALL emit an `ApplicationHostCommand` with `ServiceCall`, trace context, capability metadata, service id metadata, operation metadata, and sanitized payload metadata.

#### Scenario: No business-specific names
- **WHEN** the harness builds service, storage, render, trace, or memory/context proxy commands
- **THEN** the harness SHALL use caller-provided ids and provider-neutral labels rather than embedding provider names, gateway names, driver names, workflow names, or application-specific business names.
