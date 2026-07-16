## ADDED Requirements

### Requirement: Macaca SHALL provide Device Foreground/Background Host as a serviceized industrial pack

Macaca SHALL provide `pack.device.foreground_background_host.v1` as a provider-neutral industrial pack for host visibility/lifecycle inspection, foreground session management, background lease management, lifecycle event subscription, policy inspection, revocation, throttling/suspension diagnostics, and host status. The pack SHALL be declared by applications, resolved by admission/catalog services, and invoked only through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.device.foreground_background_host.v1` as required and the host lifecycle service is registered, healthy, entitled, policy-admissible, host-enabled, and command-compatible
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, permission scopes, lifecycle state, supported foreground presentations, supported background lease classes, throttling metadata, policy template, availability, health, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing provider secrets, credentials, raw provider payloads, host-private identifiers, or unbounded lifecycle logs

#### Scenario: Required declaration is unavailable or denied
- **WHEN** an application declares `pack.device.foreground_background_host.v1` as required but provider, command support, permission, entitlement, resource, host support, foreground presentation, background lease class, or lifecycle state is unavailable or denied
- **THEN** admission SHALL block readiness with structured unavailable, unsupported, foreground-required, background-denied, entitlement-required, presentation-required, throttled, suspended, or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back to another provider, or fake background execution

#### Scenario: Optional declaration is degraded
- **WHEN** an application declares `pack.device.foreground_background_host.v1` as optional and the pack is unavailable, throttled, suspended, or command-limited
- **THEN** admission SHALL produce an explicit degraded effective capability report with bounded reason codes
- **AND** SDK command helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: Device Foreground/Background Host SHALL expose provider-neutral lifecycle commands

`pack.device.foreground_background_host.v1` SHALL expose typed commands for `host_lifecycle.inspect_state`, `host_lifecycle.subscribe_events`, `host_lifecycle.open_foreground_session`, `host_lifecycle.close_foreground_session`, `host_lifecycle.request_background_lease`, `host_lifecycle.release_background_lease`, `host_lifecycle.inspect_policy`, `host_lifecycle.revoke`, and `host_lifecycle.inspect_host`.

#### Scenario: State inspection returns current host lifecycle
- **WHEN** a declared and policy-allowed caller invokes `host_lifecycle.inspect_state`
- **THEN** Macaca SHALL route the command through SDK/facade helpers into service runtime and the active host lifecycle provider
- **AND** the result SHALL include visibility state, execution state, suspension state, throttle state, lock/screen state when available, reason, timestamp, and provider class

#### Scenario: Event subscription emits canonical transitions
- **WHEN** a caller invokes `host_lifecycle.subscribe_events`
- **THEN** Macaca SHALL subscribe through the service runtime event path
- **AND** it SHALL emit canonical foreground, background, hidden, suspended, resumed, revoked, throttled, and terminated transition events with bounded metadata

#### Scenario: Foreground session requires presentation
- **WHEN** a caller invokes `host_lifecycle.open_foreground_session`
- **THEN** Macaca SHALL require purpose, capability type, max duration, dependent capabilities, resource budget, and presentation requirement when host policy requires it
- **AND** missing presentation SHALL return presentation-required diagnostics before provider dispatch

#### Scenario: Foreground session close is idempotent
- **WHEN** a caller invokes `host_lifecycle.close_foreground_session`
- **THEN** Macaca SHALL release presentation and resources, mark the session closed, and emit sanitized audit evidence
- **AND** repeated close calls SHALL return idempotent closed diagnostics

#### Scenario: Background lease requires entitlement
- **WHEN** a caller invokes `host_lifecycle.request_background_lease`
- **THEN** Macaca SHALL require lease class, purpose, trigger, max duration, resource budget, dependent capabilities, entitlement, and approval when configured
- **AND** missing entitlement or denied background class SHALL return typed diagnostics before provider dispatch

#### Scenario: Background lease release cleans resources
- **WHEN** a caller invokes `host_lifecycle.release_background_lease`
- **THEN** Macaca SHALL release provider resources, mark the lease released, and emit sanitized audit evidence
- **AND** repeated release calls SHALL return idempotent released diagnostics

#### Scenario: Policy inspection explains dependent capability rules
- **WHEN** a caller invokes `host_lifecycle.inspect_policy`
- **THEN** Macaca SHALL return allowed foreground classes, allowed background lease classes, throttling rules, max durations, required presentations, dependent capability rules, and denial reasons
- **AND** device packs SHALL be able to reference this policy evidence instead of duplicating lifecycle logic

#### Scenario: Revocation closes sessions and leases
- **WHEN** a caller invokes `host_lifecycle.revoke`
- **THEN** Macaca SHALL revoke active sessions/leases by scope, release resources, and emit sanitized audit evidence
- **AND** subsequent commands using revoked sessions or leases SHALL return revoked diagnostics

#### Scenario: Host inspection reports support
- **WHEN** a caller invokes `host_lifecycle.inspect_host`
- **THEN** Macaca SHALL return host support, disabled reason, active session summaries, active lease summaries, supported commands, provider class, and diagnostics
- **AND** unsupported background execution SHALL be explicit rather than fake success

### Requirement: Device Foreground/Background Host DTOs SHALL model lifecycle, sessions, leases, throttling, and policy

The pack SHALL define provider-neutral DTOs for lifecycle state, foreground sessions, background leases, lifecycle events, lifecycle policy, presentation requirements, throttle state, snapshots, and structured errors. Provider adapters SHALL translate host-specific lifecycle APIs into these DTOs and SHALL preserve enough evidence for dependent capability decisions.

#### Scenario: Lifecycle state is explicit
- **WHEN** host state is inspected or changes
- **THEN** `HostLifecycleState` SHALL include visibility, execution, suspension, throttle, lock/screen state when available, reason, timestamp, and provider class
- **AND** hidden, suspended, throttled, and unavailable states SHALL not be collapsed into generic failure

#### Scenario: Foreground session records user-visible requirement
- **WHEN** a foreground session is created
- **THEN** `ForegroundSession` SHALL include purpose, capability type, presentation requirement, max duration, state, approval id, resource reservation, dependent capabilities, and revocation state
- **AND** audit evidence SHALL prove whether required presentation was active

#### Scenario: Background lease records entitlement and expiry
- **WHEN** a background lease is granted
- **THEN** `BackgroundLease` SHALL include lease class, purpose, trigger, max duration, entitlement, approval id, resource budget, expiration, state, and revocation state
- **AND** expiry SHALL be explicit and replayable

#### Scenario: Structured errors are stable across providers
- **WHEN** providers return foreground required, background denied, entitlement required, presentation required, lease expired, lease revoked, throttled, suspended, quota, or provider failure states
- **THEN** Macaca SHALL map them to stable `HostLifecycleError` variants
- **AND** provider-specific diagnostics SHALL be sanitized and bounded

### Requirement: Device Foreground/Background Host SHALL enforce permission, policy, resource, entitlement, approval, and revocation

Every command in `pack.device.foreground_background_host.v1` SHALL run through permission, policy, resource, entitlement, approval, metering, and redaction decorators before provider side effects or host calls.

#### Scenario: Missing permission denies before provider dispatch
- **WHEN** an application invokes a command without required scope such as `device.host_lifecycle.read`, `device.host_lifecycle.events`, `device.host_lifecycle.foreground`, `device.host_lifecycle.background`, or `device.host_lifecycle.revoke`
- **THEN** Macaca SHALL return a typed denied result before invoking the concrete provider
- **AND** the audit event SHALL include the bounded missing-scope code

#### Scenario: Background is denied by default
- **WHEN** a caller requests a background lease without entitlement or allowed lease class
- **THEN** Macaca SHALL return background-denied or entitlement-required diagnostics before provider dispatch
- **AND** no fake background execution SHALL be reported

#### Scenario: Throttling is explicit
- **WHEN** the host throttles CPU, network, timers, wake eligibility, or background execution
- **THEN** Macaca SHALL expose `HostThrottleState` in results and events
- **AND** dependent capability calls SHALL receive policy evidence instead of silently failing

#### Scenario: Lease/session quota blocks resource pressure
- **WHEN** active sessions, active leases, duration, event subscriptions, CPU, network, timer budget, or retained snapshots exceed policy
- **THEN** Macaca SHALL return quota-exceeded diagnostics before provider dispatch
- **AND** resource counters SHALL be emitted in sanitized trace evidence

#### Scenario: Revocation invalidates dependent capability use
- **WHEN** a foreground session or background lease is revoked
- **THEN** Macaca SHALL emit revocation evidence that dependent device packs can consume
- **AND** subsequent dependent capability commands SHALL be denied unless a new valid session/lease exists

### Requirement: Device Foreground/Background Host SHALL preserve canonical service runtime execution

All callable operations SHALL traverse the canonical Macaca service path: application declaration, admission/effective capability projection, SDK/facade command construction, service runtime dispatch, decorators, provider adapter, structured result, trace/audit evidence, lifecycle events, and replayable snapshot. SDK helpers SHALL NOT construct providers or create alternate execution paths.

#### Scenario: Command succeeds through the canonical path
- **WHEN** a declared and policy-allowed command is invoked
- **THEN** Macaca SHALL route it through SDK/facade helpers into service runtime dispatch and the active host lifecycle provider adapter
- **AND** trace evidence SHALL show declaration, admission, policy, entitlement, resource, provider selection, session/lease state, command result, and replay pointer events

#### Scenario: Provider is absent
- **WHEN** no provider is registered for `pack.device.foreground_background_host.v1`
- **THEN** the unavailable provider SHALL return structured unavailable diagnostics
- **AND** SDK discovery SHALL report unavailable state while preserving the same provider-neutral command/result contract

#### Scenario: Provider supports only a subset
- **WHEN** the active provider supports visibility inspection but not background leases or foreground presentation
- **THEN** SDK discovery SHALL mark unsupported commands/features as non-callable
- **AND** direct invocation SHALL return typed unsupported diagnostics without falling through to application-specific logic

#### Scenario: Provider is replaced
- **WHEN** a host-native, browser, remote-host, plugin, mock, or unavailable provider is selected
- **THEN** callers SHALL observe the same provider-neutral DTO contract
- **AND** OS-layer code SHALL identify only provider class, descriptor version, lifecycle state, and capability metadata in traces rather than branching on provider names

### Requirement: Device Foreground/Background Host SHALL expose industrial SDK discovery and developer documentation

SDK discovery for `pack.device.foreground_background_host.v1` SHALL expose pack metadata, lifecycle, command schemas, DTO schemas, permission scopes, effective availability, host lifecycle state, foreground presentation support, background lease class support, throttling metadata, policy templates, examples, diagnostics, compatibility, and documentation links. The implementation SHALL provide detailed developer documentation under `docs/developer-packs/device/foreground-background-host.md`.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.device.foreground_background_host.v1`
- **THEN** it SHALL return command namespace `host_lifecycle.*`, supported commands, required scopes, current lifecycle state, supported foreground presentations, supported background lease classes, throttling metadata, examples, lifecycle, health, diagnostics, compatibility metadata, and documentation URL
- **AND** examples SHALL use generic synthetic lifecycle data rather than host-specific names, application workflows, or provider-name routing

#### Scenario: Documentation covers app developer usage
- **WHEN** a developer opens `docs/developer-packs/device/foreground-background-host.md`
- **THEN** the guide SHALL explain manifest declarations, required versus optional behavior, scopes, command DTOs, result DTOs, lifecycle states, foreground sessions, background leases, throttling/suspension, dependent capability integration, revocation, unavailable diagnostics, trace/audit behavior, and replay workflow
- **AND** it SHALL include minimal app-facing examples using synthetic lifecycle data and canonical SDK calls

#### Scenario: Documentation covers provider authors
- **WHEN** a provider author reads the guide
- **THEN** it SHALL document descriptor fields, host adapter responsibilities, lifecycle/session/lease state machines, conformance tests, unsupported behavior, redaction rules, health/snapshot behavior, and replacement strategy
- **AND** it SHALL forbid host-specific business routing in provider-neutral layers

### Requirement: Device Foreground/Background Host observability SHALL be sanitized, replayable, and auditable

The pack SHALL emit sanitized trace, audit, health, lifecycle, session, lease, snapshot, and replay evidence for declaration, admission, policy, state changes, foreground sessions, background leases, revocation, throttling, command failures, unavailable state, and snapshot recording.

#### Scenario: Successful command emits bounded evidence
- **WHEN** a host lifecycle command succeeds
- **THEN** Macaca SHALL emit sanitized events containing pack id, command name, service id, descriptor version, trace id, application/session/task/tenant ids when available, provider class, lifecycle state, session/lease id hash, lease class, presentation class, policy decision, duration class, reason code, latency, and resource counters
- **AND** it SHALL exclude raw provider payloads, secrets, credentials, prompts, package bytes, host-private identifiers, and unbounded lifecycle logs

#### Scenario: Snapshot records lifecycle summaries
- **WHEN** the service runtime records a host lifecycle snapshot
- **THEN** the snapshot SHALL include provider health, current state, policy hash, active session summaries, active lease summaries, throttling state, unavailable diagnostics, and sanitized replay pointers
- **AND** it SHALL exclude raw provider payloads, credentials, host-private identifiers, and unbounded output

#### Scenario: Replay verifies lifecycle evidence
- **WHEN** a session or task is replayed after refresh or restart
- **THEN** Macaca SHALL reconstruct the lifecycle command, event, session, lease, and revocation chain from bounded trace/audit evidence
- **AND** replay diagnostics SHALL prove the commands used the canonical service runtime path

### Requirement: Device Foreground/Background Host implementation SHALL preserve Macaca architecture boundaries

The `pack.device.foreground_background_host.v1` implementation SHALL keep concrete host/browser/remote providers behind service/runtime provider adapters. The microkernel, SDK, shells, and generic application framework SHALL remain provider-neutral and free of application-specific, provider-specific, host-specific, service-type-specific, or workflow-specific routing branches.

#### Scenario: Boundary gates scan imports
- **WHEN** dependency-boundary gates scan the implementation
- **THEN** they SHALL find no concrete lifecycle provider, foreground service API, background task API, browser lifecycle API, or remote lifecycle client in the microkernel, SDK, shells, or generic application framework
- **AND** provider construction SHALL appear only in approved runtime composition roots or plugin/remote provider registration paths

#### Scenario: No-direct-provider-call gate scans commands
- **WHEN** no-direct-provider-call gates scan host lifecycle commands
- **THEN** every callable operation SHALL be reachable only through descriptor-owned service registrations and typed service runtime dispatch
- **AND** SDK helpers SHALL only build canonical service commands

#### Scenario: Pack remains separate from workflow and device capabilities
- **WHEN** architecture review compares lifecycle-related packs
- **THEN** foreground/background host SHALL own host lifecycle state, foreground sessions, background leases, presentation requirements, throttling, suspension, and dependent capability policy evidence
- **AND** workflow scheduling, task execution, camera, sensors, files, notifications, process supervision, and application-specific background logic SHALL remain owned by their respective packs or services
