# autonomous-runtime Specification

## ADDED Requirements

### Requirement: Explicit Local Autonomy Activation

The system SHALL keep local autonomous runtime execution disabled by default
and SHALL activate production-capable local Scheduler, Heartbeat, and
Supervisor behavior only through explicit provider-neutral runtime-host
configuration.

#### Scenario: Default startup remains fail-closed

Given runtime-host starts without local autonomy activation
When it composes autonomy services
Then it registers unavailable Scheduler and Heartbeat providers
And it does not start an autonomy supervisor loop
And application-facing calls return structured unavailable results with trace
and audit correlation.

#### Scenario: Local autonomy is explicitly enabled

Given runtime-host starts with provider-neutral local autonomy activation
When it composes autonomy services
Then it registers local Scheduler and Heartbeat providers through the service
runtime
And it starts a lifecycle-managed autonomy supervisor
And application-facing scheduler and heartbeat calls reach active local
providers through SystemFacade and service runtime boundaries.

### Requirement: Runtime-Host Composition Ownership

Runtime-host SHALL be the only approved owner for constructing local autonomy
providers and the autonomy supervisor. The microkernel, SDK, Web, CLI,
frontend, applications, and application-specific code SHALL NOT construct these
providers or own the background autonomy loop.

#### Scenario: Provider construction remains in runtime-host

Given a change enables local autonomy
When dependency-boundary gates evaluate the change
Then local Scheduler, local Heartbeat, and supervisor construction appear only
under approved runtime-host composition surfaces
And no kernel, SDK, shell, frontend, or application code constructs concrete
autonomy providers.

### Requirement: Autonomy Supervisor Lifecycle

The system SHALL provide a lifecycle-managed autonomy supervisor that starts,
ticks, idles, cancels, and stops through runtime-host lifecycle control when
local autonomy is enabled.

#### Scenario: Supervisor starts in local mode

Given local autonomy is enabled
When runtime-host starts autonomy services
Then the supervisor starts with bounded tick interval, maximum leases per tick,
dispatch timeout, shutdown grace, and sanitized retention limits
And it records structured logs for start, tick, idle, cancellation, timeout,
and stop events.

#### Scenario: Supervisor stops cleanly

Given the autonomy supervisor is running
When runtime-host shuts down autonomy services
Then the supervisor stops accepting new ticks
And it cancels or drains in-flight dispatch within shutdown grace
And it records sanitized audit and log evidence for the shutdown outcome.

### Requirement: Scheduler Tick Dispatch

The autonomy supervisor SHALL use Scheduler service boundaries to materialize
or observe due runs, acquire leases, dispatch generic target commands, and
record run transitions. It SHALL NOT parse cron expressions or inspect
application business payloads.

#### Scenario: Due run dispatch succeeds

Given local autonomy is enabled and an active scheduled run is due
When the supervisor tick executes
Then it acquires a bounded run lease through Scheduler
And it dispatches the target through a provider-neutral dispatch strategy
And it transitions the run to succeeded with trace and audit evidence after the
target command reports success.

#### Scenario: Due run dispatch fails

Given a leased scheduled run dispatches to a generic target command
When the target command fails or times out
Then the supervisor records the safe failure class
And Scheduler applies retry, skip, expired, or failed transition policy
And logs and snapshots omit raw target payloads and unbounded output.

### Requirement: Provider-Neutral Dispatch Strategies

The autonomy supervisor SHALL dispatch only provider-neutral target categories
through replaceable dispatch strategies and approved service, application,
task/execution, heartbeat, or plugin boundaries.

#### Scenario: Service command target dispatches

Given a scheduled run targets a generic service command
When the supervisor dispatches the run
Then it routes the command through ServiceRuntime with trace and policy
decorators
And it does not branch on application, workflow, provider, driver, model,
gateway, chain, payment, or business-domain names.

#### Scenario: Heartbeat wake target dispatches

Given a scheduled run targets a heartbeat wake command
When the supervisor dispatches the run
Then it calls `service.heartbeat` with a typed wake intent
And Heartbeat owns coalescing, gates, and wake lifecycle decisions.

### Requirement: Heartbeat Recovery and Scheduled Wake Lane

The local autonomy runtime SHALL integrate Heartbeat as a system wake and
recovery mechanism by sending provider-neutral scheduled and recovery wake
intents through `service.heartbeat`.

#### Scenario: Recovery wake is emitted

Given local autonomy is enabled and recovery wakes are configured
When runtime-host starts after downtime
Then the supervisor emits a Heartbeat `Recovery` wake intent
And Heartbeat returns accepted, coalesced, gated, delayed, skipped, or failed
result evidence without shell-owned wake logic.

#### Scenario: Scheduled heartbeat tick is emitted

Given local autonomy is enabled with heartbeat tick configuration
When the heartbeat tick interval elapses
Then the supervisor sends a `ScheduledTick` wake intent to Heartbeat
And Heartbeat evaluates coalescing and gates before any generic target
dispatch occurs.

### Requirement: Sanitized Observability

The local autonomy runtime SHALL emit sanitized trace, audit, and log evidence
for every local autonomy activation, supervisor tick, lease attempt, dispatch
attempt, heartbeat wake, state transition, retry decision, skip decision,
failure, timeout, and shutdown event.

#### Scenario: Snapshot is inspected

Given local autonomy is enabled
When a caller inspects autonomy, scheduler, or heartbeat diagnostics
Then the response contains bounded service ids, state names, safe reason codes,
trace identifiers, audit identifiers, timestamps, and counters
And it omits raw secrets, prompts, manifests, package bytes, WASM bytes,
credentials, private keys, raw signatures, raw provider payloads, and
unbounded application output.

### Requirement: Generic Configuration Only

Autonomy runtime configuration SHALL express generic provider mode, lifecycle,
tick, lease, timeout, shutdown, recovery, and retention controls. It SHALL NOT
encode application-specific, workflow-specific, model-specific,
driver-specific, gateway-specific, chain-specific, payment-specific, or
business-domain behavior.

#### Scenario: Configuration is loaded

Given runtime-host loads autonomy configuration
When local autonomy mode is selected
Then the configuration controls only provider-neutral runtime behavior
And no OS-layer code branches on application, workflow, provider, driver,
model, gateway, chain, payment, or business-domain names.
