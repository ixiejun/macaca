# service-runtime Specification

## ADDED Requirements

### Requirement: Runtime Host Activates Local Autonomy Providers

The service runtime SHALL support runtime-host registration of either
unavailable autonomy providers or explicitly enabled local autonomy providers
without changing caller contracts.

#### Scenario: Runtime host registers unavailable providers by default

Given local autonomy activation is disabled
When runtime-host composes autonomy services
Then the service runtime registers unavailable Scheduler and Heartbeat
providers
And service calls return structured unavailable results instead of panics,
silent fallback, or fake success.

#### Scenario: Runtime host registers local providers when enabled

Given local autonomy activation is enabled
When runtime-host composes autonomy services
Then the service runtime registers local Scheduler and Heartbeat providers
through approved provider factories
And subsequent Scheduler and Heartbeat commands reach active providers through
standard trace, policy, resource, entitlement when applicable, metering when
applicable, and audit decorators.

### Requirement: Runtime Service Dispatch Supports Autonomy Supervisor

The service runtime SHALL allow the autonomy supervisor to dispatch
provider-neutral commands through standard service boundaries while preserving
trace, policy, resource, and sanitized audit decorators.

#### Scenario: Supervisor dispatches service command

Given the autonomy supervisor leases a scheduled run targeting a service
command
When it dispatches the command through ServiceRuntime
Then the same decorator chain used by external callers evaluates the command
And the supervisor receives a structured result that can be recorded in
Scheduler run state.
