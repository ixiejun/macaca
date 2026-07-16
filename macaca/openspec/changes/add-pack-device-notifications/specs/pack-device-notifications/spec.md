## ADDED Requirements

### Requirement: Macaca SHALL provide Device Notifications as a serviceized industrial pack

Macaca SHALL provide `pack.device.notifications.v1` as a provider-neutral industrial pack for authorization inspection/request, channel registration, category/action registration, immediate posting, scheduled delivery, cancellation, pending/history inspection, badge management, interaction subscription, push-support inspection, and host notification status. The pack SHALL be declared by applications, resolved by admission/catalog services, and invoked only through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.device.notifications.v1` as required and the device notification service is registered, healthy, entitled, policy-admissible, host-enabled, authorized or promptable, and command-compatible
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, permission scopes, authorization state, channel/category support, scheduling limits, interaction support, policy template, availability, health, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing provider secrets, credentials, push tokens, raw notification bodies, raw provider payloads, or unbounded interaction data

#### Scenario: Required declaration is unavailable or disabled
- **WHEN** an application declares `pack.device.notifications.v1` as required but provider, command support, permission, entitlement, resource, host support, authorization, foreground state, or host notification setting is absent
- **THEN** admission SHALL block readiness with structured unavailable, disabled, prompt-not-allowed, foreground-required, or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back to another provider, or fake success

#### Scenario: Optional declaration is degraded
- **WHEN** an application declares `pack.device.notifications.v1` as optional and the pack is unavailable, disabled, unauthorized, or command-limited
- **THEN** admission SHALL produce an explicit degraded effective capability report with bounded reason codes
- **AND** SDK command helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: Device Notifications SHALL expose supplier-grade provider-neutral commands

`pack.device.notifications.v1` SHALL expose typed commands for `notifications.inspect_authorization`, `notifications.request_authorization`, `notifications.register_channel`, `notifications.register_category`, `notifications.post`, `notifications.schedule`, `notifications.cancel`, `notifications.list_pending`, `notifications.inspect_history`, `notifications.set_badge`, `notifications.clear_badge`, `notifications.subscribe_interactions`, `notifications.inspect_push_support`, and `notifications.inspect_host`.

#### Scenario: Authorization inspection reports effective state
- **WHEN** a declared and policy-allowed caller invokes `notifications.inspect_authorization`
- **THEN** Macaca SHALL route the command through SDK/facade helpers into service runtime and the active notification provider
- **AND** the result SHALL include permission state, prompt eligibility, provisional/limited/denied flags, quiet mode, critical entitlement, host disabled reason, and provider class

#### Scenario: Authorization request is user mediated
- **WHEN** a caller invokes `notifications.request_authorization`
- **THEN** Macaca SHALL require foreground/user-mediated policy and requested option classes
- **AND** it SHALL return granted, denied, provisional, limited, prompt-not-allowed, or host-disabled state with trace evidence

#### Scenario: Channel registration validates delivery policy
- **WHEN** a caller invokes `notifications.register_channel`
- **THEN** Macaca SHALL validate stable channel id, labels, importance, sound/vibration/badge policy, grouping, locale labels, and retention
- **AND** unsupported channel features SHALL return typed unsupported diagnostics

#### Scenario: Category registration validates actions
- **WHEN** a caller invokes `notifications.register_category`
- **THEN** Macaca SHALL validate action descriptors, foreground/background handling, destructive/auth-required flags, input metadata, and callback scope
- **AND** action ids SHALL remain app-scoped and provider-neutral

#### Scenario: Immediate post applies redaction and delivery policy
- **WHEN** a caller invokes `notifications.post`
- **THEN** Macaca SHALL require channel/category where policy requires it, content redaction class, delivery policy, resource budget, and authorization state
- **AND** sensitive or oversized content SHALL be denied before provider dispatch

#### Scenario: Scheduled notification is bounded
- **WHEN** a caller invokes `notifications.schedule`
- **THEN** Macaca SHALL require trigger, expiry, timezone semantics, replacement policy, cancellation id, schedule quota, and delivery policy
- **AND** schedule-too-far or quota violations SHALL return typed diagnostics before provider dispatch

#### Scenario: Cancellation is idempotent and scoped
- **WHEN** a caller invokes `notifications.cancel`
- **THEN** Macaca SHALL cancel pending or displayed notifications by notification id, group, channel, or tag within application scope
- **AND** repeated cancellation SHALL return idempotent cancelled/not-found diagnostics without provider leakage

#### Scenario: Pending and history inspection is redacted
- **WHEN** a caller invokes `notifications.list_pending` or `notifications.inspect_history`
- **THEN** Macaca SHALL return bounded redacted summaries, trigger metadata, state, and identifiers
- **AND** raw sensitive title/body/action input SHALL not be returned unless policy explicitly permits bounded disclosure

#### Scenario: Badge commands respect host support
- **WHEN** a caller invokes `notifications.set_badge` or `notifications.clear_badge`
- **THEN** Macaca SHALL enforce badge permission, host support, policy, and quota
- **AND** unsupported badge behavior SHALL return typed unsupported diagnostics

#### Scenario: Interactions are canonical events
- **WHEN** a caller invokes `notifications.subscribe_interactions`
- **THEN** Macaca SHALL subscribe to user notification actions through the service runtime event path
- **AND** interaction events SHALL include notification id hash, action id, response class, redaction state, foreground/background request, and trace context

#### Scenario: Push support inspection does not own gateway delivery
- **WHEN** a caller invokes `notifications.inspect_push_support`
- **THEN** Macaca SHALL report host push/display support, service-worker/native support class, authorization state, and unavailable diagnostics
- **AND** it SHALL not expose raw push tokens or own gateway push routing

### Requirement: Device Notifications DTOs SHALL model notification lifecycle, content policy, and interactions safely

The pack SHALL define provider-neutral DTOs for authorization, channels, categories, actions, content, triggers, delivery policy, records, interactions, and structured errors. Provider adapters SHALL translate host-specific notification APIs into these DTOs and SHALL redact sensitive content by default.

#### Scenario: Content carries privacy metadata
- **WHEN** notification content is submitted
- **THEN** `NotificationContent` SHALL include title/body references or bounded text, localization keys, media references, redaction class, privacy class, and size class
- **AND** raw content SHALL not enter traces, audits, or snapshots by default

#### Scenario: Delivery policy controls interruption
- **WHEN** a notification is posted or scheduled
- **THEN** `NotificationDeliveryPolicy` SHALL include interruption class, sound/vibration/badge flags, lock-screen redaction, quiet-hours handling, priority/importance, rate limit class, and foreground behavior
- **AND** unsupported or disallowed interruption classes SHALL return denied or unsupported diagnostics

#### Scenario: Interaction records callback scope
- **WHEN** a user activates a notification action
- **THEN** `NotificationInteraction` SHALL include notification id, action id, response class, input redaction state, foreground/background execution request, and trace context
- **AND** background actions SHALL not execute unless policy and host capability allow them

#### Scenario: Structured errors are stable across providers
- **WHEN** providers return prompt not allowed, channel missing, category missing, content too large, sensitive content blocked, quota, schedule too far, background action denied, interaction expired, host disabled, or provider failure states
- **THEN** Macaca SHALL map them to stable `NotificationError` variants
- **AND** provider-specific diagnostics SHALL be sanitized and bounded

### Requirement: Device Notifications SHALL enforce permission, policy, resource, entitlement, approval, and redaction

Every command in `pack.device.notifications.v1` SHALL run through permission, policy, resource, entitlement, approval, metering, and redaction decorators before provider side effects or host calls.

#### Scenario: Missing permission denies before provider dispatch
- **WHEN** an application invokes a command without required scope such as `device.notifications.read_status`, `device.notifications.request_permission`, `device.notifications.post`, `device.notifications.schedule`, `device.notifications.manage`, or `device.notifications.interactions`
- **THEN** Macaca SHALL return a typed denied result before invoking the concrete provider
- **AND** the audit event SHALL include the bounded missing-scope code

#### Scenario: Sensitive lock-screen content is blocked
- **WHEN** notification content is privacy-sensitive and lock-screen redaction policy is missing or disallowed
- **THEN** Macaca SHALL return sensitive-content-blocked diagnostics before provider dispatch
- **AND** raw content SHALL not be recorded in trace/audit evidence

#### Scenario: Critical delivery requires entitlement
- **WHEN** a caller requests critical, urgent, or interruption-elevated delivery
- **THEN** Macaca SHALL require entitlement and approval when configured
- **AND** missing entitlement or approval SHALL return denied diagnostics before provider dispatch

#### Scenario: Background action is denied by default
- **WHEN** a notification action requests background execution
- **THEN** Macaca SHALL deny the action unless host policy and foreground/background capability explicitly allow it
- **AND** interaction audit evidence SHALL record the bounded denial reason

#### Scenario: Schedule quota blocks excessive pending notifications
- **WHEN** pending notification count, schedule horizon, action count, content size, or interaction subscription count exceeds budget
- **THEN** Macaca SHALL return quota-exceeded diagnostics before provider dispatch
- **AND** resource counters SHALL be emitted in sanitized trace evidence

### Requirement: Device Notifications SHALL preserve canonical service runtime execution

All callable operations SHALL traverse the canonical Macaca service path: application declaration, admission/effective capability projection, SDK/facade command construction, service runtime dispatch, decorators, provider adapter, structured result, trace/audit evidence, interaction events, and replayable snapshot. SDK helpers SHALL NOT construct providers or create alternate execution paths.

#### Scenario: Command succeeds through the canonical path
- **WHEN** a declared and policy-allowed command is invoked
- **THEN** Macaca SHALL route it through SDK/facade helpers into service runtime dispatch and the active notification provider adapter
- **AND** trace evidence SHALL show declaration, admission, policy, entitlement, resource, provider selection, command result, notification lifecycle state, and replay pointer events

#### Scenario: Provider is absent
- **WHEN** no provider is registered for `pack.device.notifications.v1`
- **THEN** the unavailable provider SHALL return structured unavailable diagnostics
- **AND** SDK discovery SHALL report unavailable state while preserving the same provider-neutral command/result contract

#### Scenario: Provider supports only a subset
- **WHEN** the active provider supports local post but not scheduling, history, badges, push support, or action input
- **THEN** SDK discovery SHALL mark unsupported commands/features as non-callable
- **AND** direct invocation SHALL return typed unsupported diagnostics without falling through to application-specific logic

#### Scenario: Provider is replaced
- **WHEN** a host-native, browser, remote-host, plugin, mock, or unavailable provider is selected
- **THEN** callers SHALL observe the same provider-neutral DTO contract
- **AND** OS-layer code SHALL identify only provider class, descriptor version, authorization state, and capability metadata in traces rather than branching on provider names

### Requirement: Device Notifications SHALL expose industrial SDK discovery and developer documentation

SDK discovery for `pack.device.notifications.v1` SHALL expose pack metadata, lifecycle, command schemas, DTO schemas, permission scopes, effective availability, authorization state, channel/category support, schedule limits, badge support, interaction support, push-support boundary, policy templates, examples, diagnostics, compatibility, and documentation links. The implementation SHALL provide detailed developer documentation under `docs/developer-packs/device/notifications.md`.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.device.notifications.v1`
- **THEN** it SHALL return command namespace `notifications.*`, supported commands, required scopes, authorization state, channel/category support, schedule limits, badge support, interaction support, policy templates, examples, lifecycle, health, diagnostics, compatibility metadata, and documentation URL
- **AND** examples SHALL use generic synthetic content rather than application-specific workflows or provider-name routing

#### Scenario: Documentation covers app developer usage
- **WHEN** a developer opens `docs/developer-packs/device/notifications.md`
- **THEN** the guide SHALL explain manifest declarations, required versus optional behavior, scopes, command DTOs, result DTOs, authorization, channels, categories, actions, content redaction, delivery policy, scheduling, cancellation, badges, interactions, push-support boundary, unavailable diagnostics, trace/audit behavior, and replay workflow
- **AND** it SHALL include minimal app-facing examples that use synthetic notification content and canonical SDK calls

#### Scenario: Documentation covers provider authors
- **WHEN** a provider author reads the guide
- **THEN** it SHALL document descriptor fields, host adapter responsibilities, authorization/notification/interaction state machines, conformance tests, unsupported behavior, redaction rules, health/snapshot behavior, and replacement strategy
- **AND** it SHALL forbid application-specific business routing in provider-neutral layers

### Requirement: Device Notifications observability SHALL be sanitized, replayable, and auditable

The pack SHALL emit sanitized trace, audit, health, notification lifecycle, interaction, snapshot, and replay evidence for declaration, admission, policy, authorization, channel/category registration, posting, scheduling, cancellation, interaction, badge updates, command failures, unavailable state, and snapshot recording.

#### Scenario: Successful command emits bounded evidence
- **WHEN** a notification command succeeds
- **THEN** Macaca SHALL emit sanitized events containing pack id, command name, service id, descriptor version, trace id, application/session/task/tenant ids when available, provider class, notification id hash, channel id, category id, action id, delivery class, redaction class, policy decision, latency, and resource counters
- **AND** it SHALL exclude raw notification bodies, raw action input, push tokens, raw provider payloads, secrets, credentials, and unbounded interaction data

#### Scenario: Interaction event is redacted
- **WHEN** a notification interaction is received
- **THEN** Macaca SHALL emit canonical interaction evidence with notification id hash, action id, response class, input redaction state, foreground/background request, and trace context
- **AND** raw text input SHALL be omitted or bounded according to policy

#### Scenario: Snapshot records pending summaries
- **WHEN** the service runtime records a notification snapshot
- **THEN** the snapshot SHALL include provider health, authorization state, channel/category descriptor hashes, pending count, schedule limits, interaction subscription summaries, policy template hash, unavailable diagnostics, and sanitized replay pointers
- **AND** it SHALL exclude raw notification content, raw provider payloads, push tokens, credentials, and unbounded output

#### Scenario: Replay verifies notification lifecycle
- **WHEN** a session or task is replayed after refresh or restart
- **THEN** Macaca SHALL reconstruct the notification command and interaction chain from bounded trace/audit evidence
- **AND** replay diagnostics SHALL prove the commands used the canonical service runtime path without raw notification bodies

### Requirement: Device Notifications implementation SHALL preserve Macaca architecture boundaries

The `pack.device.notifications.v1` implementation SHALL keep concrete host/browser/remote providers behind service/runtime provider adapters. The microkernel, SDK, shells, and generic application framework SHALL remain provider-neutral and free of application-specific, provider-specific, host-specific, channel-specific, or workflow-specific routing branches.

#### Scenario: Boundary gates scan imports
- **WHEN** dependency-boundary gates scan the implementation
- **THEN** they SHALL find no concrete notification provider, host notification API, browser notification API, push gateway provider, or remote notification client in the microkernel, SDK, shells, or generic application framework
- **AND** provider construction SHALL appear only in approved runtime composition roots or plugin/remote provider registration paths

#### Scenario: No-direct-provider-call gate scans commands
- **WHEN** no-direct-provider-call gates scan notification commands
- **THEN** every callable operation SHALL be reachable only through descriptor-owned service registrations and typed service runtime dispatch
- **AND** SDK helpers SHALL only build canonical service commands

#### Scenario: Pack remains separate from communication and workflow capabilities
- **WHEN** architecture review compares notification-related packs
- **THEN** device notifications SHALL own host notification display, authorization, channels/categories/actions, scheduling at host notification level, badges, and interaction mediation
- **AND** communication notification, messaging, inbox, gateway push delivery, workflow schedule, and application-specific reminder logic SHALL remain owned by their respective packs or services
