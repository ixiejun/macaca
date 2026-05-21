# service-runtime Specification

## ADDED Requirements

### Requirement: Runtime Service Registration for Autonomy Scheduler and Heartbeat

The service runtime SHALL support registration of provider-neutral scheduler and
heartbeat service descriptors through runtime-host composition.

#### Scenario: Runtime host installs built-in unavailable providers

Given runtime-host boots without concrete scheduler or heartbeat providers
When it composes the service runtime
Then it may register built-in unavailable providers for `service.scheduler` and
`service.heartbeat`
And those providers fail closed with structured unavailable results
And provider absence remains visible through health and snapshot commands

### Requirement: Runtime Decorators Wrap Scheduler and Heartbeat Calls

Scheduler and heartbeat service calls SHALL pass through standard runtime
decorators for trace, policy, resource, entitlement when applicable, metering
when applicable, and sanitized audit.

#### Scenario: Scheduler command is dispatched through service runtime

Given a caller sends a scheduler command through the facade
When the service runtime dispatches the command
Then trace context is attached or propagated before provider invocation
And policy is evaluated before side effects
And sanitized audit evidence is recorded for the command outcome

### Requirement: Runtime Does Not Hardcode Application Semantics

Runtime registration and dispatch for scheduler and heartbeat SHALL NOT branch
on application, workflow, provider, driver, model, gateway, chain, payment, or
business-domain names.

#### Scenario: New application uses scheduler capability

Given a newly installed application declares scheduler capability
When it registers a provider-neutral scheduled job
Then the service runtime routes the command by service capability and policy
And it requires no OS source-code branch for that application
