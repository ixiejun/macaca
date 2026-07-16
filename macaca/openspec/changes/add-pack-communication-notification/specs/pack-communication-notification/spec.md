## ADDED Requirements

### Requirement: Macaca SHALL provide Communication Notification Pack as a serviceized capability

Macaca SHALL provide `pack.communication.notification.v1` as a provider-neutral
industrial pack for local, push, in-app, scheduled, interactive, dismissible,
and inspectable notifications. Applications SHALL declare the pack in manifests,
admission SHALL resolve it into effective capabilities, and all operations SHALL
run through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.communication.notification.v1` as required and a notification service provider is registered, healthy, entitled, host-supported, consent-compatible, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, permission scopes, policy templates, provider capability metadata, health, diagnostics, documentation links, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing raw tokens, raw endpoints, credentials, raw provider payloads, or provider secrets

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.communication.notification.v1` as required but provider, permission, consent, entitlement, host support, resource budget, or policy support is absent
- **THEN** admission SHALL block readiness with structured unavailable, denied, unsupported, or quota diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.communication.notification.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report
- **AND** SDK command helpers SHALL return Null Object unavailable diagnostics instead of creating callable service calls

### Requirement: Notification commands SHALL use typed canonical service calls

Every `pack.communication.notification.v1` operation SHALL be represented as a
typed command/result DTO and SHALL traverse the canonical service runtime path
with trace, policy, consent, resource, entitlement, approval, health, snapshot,
idempotency, replay, and structured error behavior.

#### Scenario: Immediate notification is published
- **WHEN** a declared and policy-allowed `notification.publish` command is invoked with a valid `NotificationMessage`, `NotificationTarget`, delivery channel, and idempotency key
- **THEN** Macaca SHALL route the command through SDK/facade helpers into the service runtime and notification service provider
- **AND** it SHALL return a typed delivery handle and emit sanitized admission, policy, service-call, provider-call, result, and replay events

#### Scenario: Notification is scheduled
- **WHEN** a declared and policy-allowed `notification.schedule` command is invoked with schedule metadata, timezone or monotonic delay policy, expiry, target, and idempotency key
- **THEN** Macaca SHALL create a typed schedule handle or return a structured denied, unsupported, quota, or unavailable result before provider side effects
- **AND** replay metadata SHALL make the scheduled operation trace-addressable after restart

#### Scenario: Notification is updated or canceled
- **WHEN** `notification.update` or `notification.cancel` is invoked for a prior delivery or schedule handle
- **THEN** Macaca SHALL validate handle scope, provider capability, policy, and idempotency before invoking the provider
- **AND** it SHALL distinguish updated, canceled, already_terminal, unsupported, unavailable, and partial outcomes

#### Scenario: Notification action is received
- **WHEN** a provider reports that a user selected an action or submitted action input
- **THEN** Macaca SHALL convert the provider event into a typed `NotificationActionEvent` scoped to the application, session, task, tenant, and trace context
- **AND** action delivery SHALL pass through policy and application capability checks before reaching the application

#### Scenario: Command is denied before side effects
- **WHEN** policy, permission, consent, entitlement, approval, or resource checks reject a notification command
- **THEN** Macaca SHALL return a typed denied or quota result before invoking the concrete provider
- **AND** audit evidence SHALL include a bounded reason code without raw notification content, user input, provider payloads, tokens, endpoints, or credentials

### Requirement: Notification DTOs SHALL be provider-neutral and secure by default

`pack.communication.notification.v1` SHALL define portable DTOs for message
content, targets, delivery channels, schedules, actions, action events,
subscriptions, delivery handles, delivery status, provider capability, and
diagnostics. Provider-specific fields SHALL be bounded extension data owned by
provider adapters and SHALL NOT become OS-layer routing branches.

#### Scenario: Developer builds a notification message
- **WHEN** SDK schemas expose `NotificationMessage`
- **THEN** the schema SHALL include bounded title/body/summary fields, locale, category, group key, collapse key, badge metadata, urgency, expiry, media handles, content sensitivity, and bounded data
- **AND** the schema SHALL reject unbounded content and raw provider payloads

#### Scenario: Developer registers a push subscription
- **WHEN** `notification.register_subscription` receives device, browser, or provider subscription data
- **THEN** the command SHALL accept only opaque handles, capability metadata, and secret references for sensitive values
- **AND** raw device tokens, raw push endpoints, raw keys, credentials, and raw provider payloads SHALL be rejected or redacted before storage, trace, audit, and snapshots

#### Scenario: Provider reports capability limits
- **WHEN** SDK discovery inspects the active notification provider
- **THEN** Macaca SHALL report supported channels, target classes, scheduling support, action support, update/cancel support, delivery inspection depth, payload limits, action limits, rate limits, host limitations, lifecycle, and health
- **AND** callers SHALL use this metadata rather than provider-name branches

### Requirement: Notification Pack SHALL enforce permission, consent, policy, and quota

`pack.communication.notification.v1` SHALL define permission scopes for publish,
schedule, update, cancel, action registration, action receipt, subscription
management, delivery inspection, and host notification surfaces. Policy SHALL
run before side effects and SHALL account for consent, target scope, content
sensitivity, host support, provider health, entitlement, quotas, and approval.

#### Scenario: Missing permission blocks command
- **WHEN** an application invokes a notification command without the required permission scope
- **THEN** Macaca SHALL return a typed denied result and SHALL NOT invoke the provider
- **AND** trace/audit evidence SHALL identify the missing scope by stable code

#### Scenario: Host notification support is disabled
- **WHEN** a command requires host notification surfaces but the host or user consent state does not allow notifications
- **THEN** Macaca SHALL return a structured unavailable or denied result with host capability diagnostics
- **AND** optional declarations SHALL degrade explicitly while required declarations SHALL block readiness

#### Scenario: Delivery inspection requires separate permission
- **WHEN** an application invokes `notification.inspect_delivery`
- **THEN** Macaca SHALL require `notification.delivery.inspect` permission and return only redacted provider evidence, delivery status, retry state, confidence, and bounded reason codes
- **AND** raw provider receipts or raw user/device identifiers SHALL NOT be exposed

### Requirement: Notification Pack SHALL expose industrial metadata and developer documentation

`pack.communication.notification.v1` SHALL expose descriptor metadata for command
schemas, permission scopes, policy templates, resource budgets, SDK examples,
lifecycle state, compatibility, health probes, snapshots, unavailable
diagnostics, redaction profiles, and developer documentation.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.communication.notification.v1`
- **THEN** it SHALL return command namespace `notification.*`, supported commands, permissions, policy templates, examples, lifecycle, availability, provider capability metadata, health, diagnostics, compatibility, redaction profile, and documentation links
- **AND** examples SHALL use generic handles and synthetic data rather than application-specific workflows, provider names, credentials, or business routing

#### Scenario: Developer documentation is published
- **WHEN** the pack implementation is marked complete
- **THEN** `docs/developer-packs/communication/notification.md` SHALL document manifest declaration, permissions, DTOs, result handling, idempotency, scheduling, actions, subscription security, delivery inspection, provider replacement, unavailable diagnostics, trace/audit interpretation, and operational limits
- **AND** SDK discovery metadata and the industrial catalog index SHALL link to that guide

### Requirement: Notification Pack SHALL be traceable, auditable, replayable, and sanitized

`pack.communication.notification.v1` SHALL emit sanitized trace/audit events and
bounded snapshots for declaration, admission, consent, policy, resource,
entitlement, approval, service calls, provider calls, action callbacks, delivery
status changes, subscription changes, health, unavailable states, and replay.

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a notification pack snapshot
- **THEN** the snapshot SHALL include descriptor version, provider class, health, command availability, consent capability state, subscription handle counts, action definitions, policy template hash, resource counters, delivery status aggregates, and sanitized replay pointers
- **AND** it SHALL exclude raw secrets, credentials, raw tokens, raw endpoints, raw keys, raw notification payloads, raw provider responses, manifests, package bytes, private keys, signatures, and unbounded output

#### Scenario: Delivery status changes
- **WHEN** a notification changes from accepted to queued, scheduled, sent, delivered, displayed, clicked, dismissed, acknowledged, expired, canceled, failed, partial, unsupported, unavailable, or unknown
- **THEN** Macaca SHALL emit a sanitized `notification_pack_delivery_status_changed` event with stable handles, bounded status code, confidence metadata, and replay pointer
- **AND** consumers SHALL NOT infer stronger delivery guarantees than the provider capability report supports

### Requirement: Notification implementation SHALL preserve Macaca boundaries

The `pack.communication.notification.v1` implementation SHALL remain owned by
notification service providers behind the service runtime. The microkernel, SDK,
shells, and generic application framework SHALL remain provider-neutral and free
of application-specific or provider-specific routing branches.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete notification provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.communication.notification.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, capability hash, and result codes rather than provider-specific business branches
