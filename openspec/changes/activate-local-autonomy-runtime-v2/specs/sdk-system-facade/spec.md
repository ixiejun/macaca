# sdk-system-facade Specification

## ADDED Requirements

### Requirement: Facade Observes Production-Active Autonomy Providers

The SystemFacade and focused SDK autonomy clients SHALL preserve the same
public command contract in disabled and enabled autonomy modes while returning
active local provider results when runtime-host explicitly enables local
autonomy.

#### Scenario: Disabled mode returns unavailable

Given local autonomy activation is disabled
When an application or shell calls the Scheduler or Heartbeat client
Then the client returns the structured unavailable service result
And it does not construct providers, timers, stores, queues, or supervisor
loops.

#### Scenario: Enabled mode reaches local provider

Given local autonomy activation is enabled
When an application or shell calls the Scheduler or Heartbeat client
Then the client sends the typed command through ServiceRuntime
And the command reaches the active local provider
And the client returns provider-neutral result DTOs with trace and audit
correlation.

### Requirement: Facade Does Not Own Autonomy Runtime Semantics

The SDK and SystemFacade SHALL NOT own local autonomy activation, background
tick loops, due-run materialization, lease acquisition, heartbeat coalescing,
gate evaluation, provider construction, or dispatch strategy selection.

#### Scenario: Application registers scheduled work

Given an application has declared scheduler capability
When it registers scheduled work through the SDK
Then the SDK creates only typed provider-neutral command DTOs
And runtime-host, Scheduler, Heartbeat, and the autonomy supervisor own the
runtime behavior behind service boundaries.
