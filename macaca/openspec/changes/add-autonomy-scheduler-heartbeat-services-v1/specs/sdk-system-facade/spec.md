# sdk-system-facade Specification

## ADDED Requirements

### Requirement: Scheduler Focused Client

The SystemFacade or SDK layer SHALL expose a focused scheduler client that wraps
typed `service.scheduler` commands without constructing scheduler providers,
timers, queues, stores, cron parsers, or runtime internals.

#### Scenario: Shell pauses a scheduled job

Given a shell caller has a SystemFacade scheduler client
When it pauses a scheduled job
Then the client sends a typed scheduler command through the service runtime
And the client returns the scheduler service result without implementing
scheduler lifecycle semantics in shell code

### Requirement: Heartbeat Focused Client

The SystemFacade or SDK layer SHALL expose a focused heartbeat client that wraps
typed `service.heartbeat` commands without constructing heartbeat providers,
wake queues, timers, stores, or runtime internals.

#### Scenario: Application requests a heartbeat wake

Given an application has declared heartbeat capability
When it requests a heartbeat wake through the SDK
Then the client sends a typed heartbeat command through the service runtime
And the client returns the heartbeat service result without implementing wake
coalescing or gate semantics in application code

### Requirement: Facade Preserves Structured Autonomy Errors

The SystemFacade scheduler and heartbeat clients SHALL preserve structured
unavailable, unsupported, denied, invalid-request, conflict, provider-failure,
and timeout results.

#### Scenario: Scheduler provider is unavailable

Given no scheduler provider is installed or enabled
When a facade caller registers a job
Then the client returns the scheduler service's structured unavailable result
And it does not panic, silently fallback, or fake successful scheduling

### Requirement: Facade Preserves Trace and Audit Correlation

The SystemFacade scheduler and heartbeat clients SHALL attach or propagate trace
context and return audit correlation identifiers provided by the service
runtime.

#### Scenario: Heartbeat wake is denied

Given a heartbeat wake request is denied by policy
When the facade returns the result
Then the caller can inspect the structured denied state
And the result includes trace and audit correlation safe for diagnostics
