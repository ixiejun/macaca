# Device Notifications Pack Design

## Context

`pack.device.notifications.v1` exposes host notification capability through Macaca's service runtime. Mature platforms converge on explicit authorization, channel/category registration, user-visible content controls, scheduled delivery, action callbacks, and per-host restrictions. Macaca must normalize these concepts while keeping notification delivery auditable and provider-replaceable.

This pack owns host notification presentation and interaction mediation. Gateway push delivery, communication messages, and application-specific reminder workflows remain separate capabilities.

## Supplier Capability Matrix

| Platform/API | Borrowed capability | Macaca mapping |
| --- | --- | --- |
| Android Notifications | runtime permission, channels, importance, actions, foreground-service visibility | authorization state, channel descriptor, delivery policy, foreground constraints |
| Apple UserNotifications | authorization, requests, triggers, categories, actions, badges, notification center callbacks | request/schedule DTOs, categories/actions, badge commands, interaction events |
| Web Notifications/Push | browser permission, notification display, actions, service workers, push subscriptions | host provider class, push-support inspection, interaction subscription |
| Windows App Notifications | toast content, actions, activation, history/local notifications | content/action DTOs, activation event bridge, pending/history commands |
| HarmonyOS Notification Kit | slots/channels, authorization, actions, badges, distributed host mediation | channel/category mapping, host status, provider diagnostics |

## Goals

- Provide authorization inspection/request, channel/category registration, post/schedule/cancel/list notifications, badge management, interaction subscription, push-support inspection, and host status.
- Normalize content, triggers, actions, channels, categories, delivery policy, interruption class, lock-screen redaction, and callback semantics.
- Enforce permission, policy, approval, resource quotas, foreground/background rules, and redaction before host notification operations.
- Support host-native, browser, remote-host, plugin, mock, and unavailable providers through descriptors.
- Provide detailed developer documentation and provider conformance guidance.

## Non-Goals

- Do not own communication messaging, inbox storage, email/SMS, gateway push routing, remote campaign orchestration, calendar/workflow reminders, or application-specific ranking.
- Do not expose raw push tokens, raw notification bodies, secrets, credentials, or provider payloads in observability.
- Do not branch on host OS, provider name, channel name, action name, business workflow, or application id in OS-layer code.

## Ownership And Boundaries

- Pack id: `pack.device.notifications.v1`.
- Capability family: `device`.
- Backing service: device notification service.
- SDK surface: `sdk.packs.device.notifications`.
- Command namespace: `notifications.*`.
- Application framework owns manifest declarations and app-scoped permission projection.
- Service runtime owns typed dispatch, decorators, notification lifecycle, interaction event bridge, health, snapshots, and unavailable behavior.
- Runtime host owns concrete host/browser/provider adapters through approved composition roots.
- Shells render diagnostics and interaction surfaces only through SDK/service events.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `notifications.inspect_authorization` | Inspect host notification permission and effective policy | Returns permission state, prompt eligibility, host disabled reasons, channel constraints, and provider class |
| `notifications.request_authorization` | Request notification authorization | Requires foreground/user-mediated policy and returns granted/denied/provisional/limited state |
| `notifications.register_channel` | Register/update a notification channel/topic/slot | Validates importance, sound/vibration/badge policy, locale labels, and stable channel id |
| `notifications.register_category` | Register actionable notification category | Validates actions, foreground/background handling, destructive/auth-required flags, and callback scope |
| `notifications.post` | Post an immediate local notification | Requires content redaction policy, channel/category, delivery policy, resource quota, and approval when sensitive |
| `notifications.schedule` | Schedule local notification delivery | Requires trigger, expiry, timezone behavior, quota, replacement policy, and cancellation id |
| `notifications.cancel` | Cancel pending or displayed notifications | Idempotently cancels by notification id, group, channel, or tag within app scope |
| `notifications.list_pending` | List pending scheduled notifications | Returns redacted summaries and trigger metadata |
| `notifications.inspect_history` | Inspect displayed/delivered notification summaries when host supports it | Returns bounded history summaries without raw sensitive content unless policy permits |
| `notifications.set_badge` | Set badge count/text where supported | Enforces badge policy and host support |
| `notifications.clear_badge` | Clear badge state | Idempotent badge clear |
| `notifications.subscribe_interactions` | Subscribe to user interaction/action events | Emits canonical service events with redacted payloads |
| `notifications.inspect_push_support` | Inspect host push subscription/display support | Reports push/display capability without owning gateway push delivery |
| `notifications.inspect_host` | Inspect provider health and host notification status | Returns disabled/degraded/provider diagnostics |

## DTO Model

- `NotificationAuthorization`: state, prompt eligibility, provisional/limited/denied flags, quiet mode, critical entitlement, host disabled reason, and provider class.
- `NotificationChannel`: stable id, label, description, importance, sound/vibration/badge policy, grouping, locale labels, retention, and provider mapping.
- `NotificationCategory`: stable id, action descriptors, foreground/background handling, auth-required/destructive flags, and callback scope.
- `NotificationAction`: stable id, label, kind, destructive flag, authentication requirement, input field metadata, and callback policy.
- `NotificationContent`: title, body, subtitle, summary, icon/media references, data references, localization keys, redaction class, privacy class, and size class.
- `NotificationTrigger`: immediate, timestamp, interval, calendar-like, location-external-reference, app-event reference, expiry, timezone semantics, and replacement policy.
- `NotificationDeliveryPolicy`: interruption class, sound/vibration/badge flags, lock-screen redaction, quiet-hours handling, priority/importance, rate limit class, and foreground behavior.
- `NotificationRecord`: notification id, channel, category, trigger, state, posted/delivered/cancelled timestamps, redacted content summary, and provenance.
- `NotificationInteraction`: notification id, action id, response class, input redaction state, foreground/background execution request, and trace context.
- `NotificationError`: denied, unavailable, unsupported, prompt not allowed, channel missing, category missing, content too large, sensitive content blocked, quota exceeded, schedule too far, background action denied, interaction expired, host disabled, provider failure, or conflict.

## Permission, Policy, Resource, Entitlement, And Approval

Initial scopes:

- `device.notifications.read_status`: inspect authorization, host status, pending/history summaries.
- `device.notifications.request_permission`: request host authorization.
- `device.notifications.post`: immediate notifications.
- `device.notifications.schedule`: scheduled notifications.
- `device.notifications.manage`: channels, categories, cancellation, badge state.
- `device.notifications.interactions`: subscribe to user interactions.

Policy requirements:

- Notification content is sensitive by default; traces store redacted summaries, hashes, and classes.
- Sensitive lock-screen content requires explicit redaction policy.
- Critical/urgent/interruption-elevated delivery requires entitlement and approval when configured.
- Background actions are denied unless host policy and foreground/background capability allow them.
- Scheduling requires bounded count, expiry, replacement policy, and timezone semantics.
- Push token/subscription material is not exposed by this pack; push gateway integration must use gateway/communication service boundaries.

## Service Runtime And Provider Strategy

Provider Strategy categories:

- Host-native provider: OS notification center/toast APIs.
- Browser provider: Web Notifications/Push display APIs.
- Remote-host provider: notification delivery on a delegated trusted host.
- Plugin provider: enterprise/device-management notification bridge.
- Mock provider: deterministic notifications/interactions for tests/docs.
- Unavailable provider: explicit unavailable diagnostics.

Providers declare authorization states, channel/category support, action limits, scheduling limits, history support, badge support, push display support, foreground/background constraints, and health.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, lifecycle, command schemas, DTO schemas, permission scopes, effective availability, authorization state, channel/category support, scheduling limits, action limits, badge support, interaction support, policy templates, examples, diagnostics, compatibility, and documentation links.

The implementation SHALL create `docs/developer-packs/device/notifications.md` with manifest examples, scopes, authorization flow, channel/category/action setup, content redaction, posting, scheduling, cancellation, badges, interactions, push-support boundary, unavailable diagnostics, trace/audit reference, and provider conformance checklist.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `notifications.pack_declared`
- `notifications.admission_validated`
- `notifications.policy_decision`
- `notifications.authorization_requested`
- `notifications.authorization_changed`
- `notifications.channel_registered`
- `notifications.category_registered`
- `notifications.notification_posted`
- `notifications.notification_scheduled`
- `notifications.notification_cancelled`
- `notifications.interaction_received`
- `notifications.badge_updated`
- `notifications.command_failed`
- `notifications.unavailable`
- `notifications.snapshot_recorded`

Events include pack id, command name, service id, descriptor version, trace id, application/session/task/tenant ids when present, provider class, notification id hash, channel id, category id, action id, delivery class, redaction class, policy decision, latency, and resource counters. Events exclude raw bodies, raw input responses, push tokens, credentials, secrets, and unbounded provider payloads.

Snapshots include provider health, authorization state, channel/category descriptor hashes, pending count, schedule limits, interaction subscription summaries, policy template hash, unavailable diagnostics, and sanitized replay pointers.

## Design Patterns

- **Facade**: SDK exposes discovery and command builders while `SystemFacade` carries canonical service calls.
- **Command**: every operation is a typed command/result DTO.
- **Adapter**: host, browser, remote, plugin, mock, and unavailable providers map into Macaca DTOs.
- **Strategy**: provider selection, delivery policy, authorization flow, scheduling support, and unavailable behavior are descriptor-driven.
- **Decorator**: trace, policy, resource, entitlement, approval, metering, and redaction wrap every call.
- **State**: authorization, notification lifecycle, and interaction subscriptions are explicit state machines.
- **Specification**: admission validates scopes, channels, categories, content size, schedule bounds, and delivery policy.
- **Observer**: trace, audit, interaction, health, and notification lifecycle events are subscribable.
- **Memento**: snapshots record pending notifications and subscriptions without raw content.
- **Abstract Factory**: providers are created only in approved composition roots.

## Risks And Mitigations

- Risk: notifications become spam or covert exfiltration. Mitigation: authorization, rate limits, content redaction, approval, and resource budgets.
- Risk: raw content leaks in traces. Mitigation: event schemas store hashes/classes and redacted summaries only.
- Risk: action callbacks become hidden workflows. Mitigation: interactions are canonical events; application behavior remains app-owned.
- Risk: push gateway semantics leak into device pack. Mitigation: this pack inspects push/display support only; gateway delivery stays serviceized elsewhere.
- Risk: SDK helpers bypass host authorization. Mitigation: helpers only build canonical service commands and gates enforce service dispatch.
