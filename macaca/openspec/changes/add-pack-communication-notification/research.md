# Communication Notification Pack Research

## Purpose

This note records supplier/API research and Macaca platform inventory for
`pack.communication.notification.v1`. Notification support must provide
provider-neutral local, push, in-app, host, scheduled, actionable, subscription,
and delivery-inspection operations through the service runtime. It must not make
kernel, SDK, shell, or application framework code depend on APNs, FCM, Web Push,
Android, Windows, or provider-specific notification payloads.

## Source Baseline

- Apple UserNotifications and APNs:
  <https://developer.apple.com/documentation/usernotifications>,
  <https://developer.apple.com/documentation/usernotifications/asking-permission-to-use-notifications>,
  and <https://developer.apple.com/documentation/usernotifications/sending-notification-requests-to-apns>
- Android Notifications and runtime permission:
  <https://developer.android.com/develop/ui/views/notifications>
  and <https://developer.android.com/develop/ui/compose/notifications/notification-permission>
- Firebase Cloud Messaging:
  <https://firebase.google.com/docs/cloud-messaging/android/get-started>
  and <https://firebase.google.com/docs/cloud-messaging/web/get-started>
- Web Notifications and Push:
  <https://developer.mozilla.org/en-US/docs/Web/API/Notifications_API>,
  <https://developer.mozilla.org/en-US/docs/Web/API/Push_API>, and
  <https://www.w3.org/TR/push-api/>
- Windows App Notifications:
  <https://learn.microsoft.com/en-us/windows/apps/develop/notifications/app-notifications/>

## Supplier API Notes

Apple UserNotifications and APNs contribute authorization, local notification,
remote push, scheduling, categories, and action concepts:

- Authorization state can change at runtime, so Macaca must model consent and
  host capability as dynamic effective capability, not as static manifest truth.
- Local notification requests, triggers, categories, and actions map to
  message, schedule, action-definition, and action-event DTOs.
- APNs device tokens, payloads, push types, and credentials must become
  subscription handles and secret references behind provider adapters.

Android Notifications and FCM contribute runtime permission, channels,
grouping, actions, update/cancel, token/topic targeting, and foreground/
background delivery constraints:

- Notification channels, permission state, priority/importance, grouping, badge,
  actions, and update/cancel behavior map to policy profile, delivery channel,
  action definition, group key, and delivery handle abstractions.
- FCM tokens, topics, condition targets, notification/data payload classes,
  platform overrides, foreground/background receive behavior, and delivery
  metrics map to target refs, subscription handles, payload class, provider
  capability, and delivery inspection.

Web Notifications and Push contribute browser permission and service-worker
subscription concepts:

- Notification permission and secure-context/service-worker requirements map to
  host support and consent diagnostics.
- Push subscriptions expose endpoint and key material. Macaca must store only
  opaque subscription handles and secret references, never raw endpoints or
  keys in SDK or observability surfaces.
- Push messages delivered to service workers map to action/event bridges and
  delivery-status updates with replayable evidence.

Windows App Notifications contribute local toast/app notification concepts:

- Windows notifications may show UI outside the app window, launch the app, or
  trigger background actions. Macaca should model action callbacks and
  activation events without making notification UI semantics an OS concern.
- Toast/action XML and platform limitations must remain provider details.
- Notification management and response to user interaction map to list,
  update/cancel, acknowledge, dismiss, action-event, and delivery inspection
  commands.

## Macaca-Owned Abstractions

`pack.communication.notification.v1` should define these provider-neutral
concepts:

- `NotificationMessage`: bounded title/body/summary, locale, sensitivity,
  category, thread/group key, collapse key, urgency, expiry, media handles, and
  provider option reference.
- `NotificationTarget`: app/session/user/tenant scoped target, device handle,
  subscription handle, topic handle, group handle, condition expression, or
  in-app surface handle. Raw device tokens and push endpoints are forbidden.
- `NotificationDeliveryChannel`: local, push, in_app, host, provider_remote, or
  explicit policy-selected channel with trace metadata.
- `NotificationSchedule`: immediate, wall-clock time, monotonic delay,
  recurrence reference, timezone, deadline, expiry, and drift policy.
- `NotificationActionDefinition`: action id, label, semantic role, destructive
  flag, foreground/authentication requirement, input schema, callback route, and
  policy profile.
- `NotificationActionEvent`: delivery handle, action id, bounded input, context,
  app/session/task refs, trace id, and replay pointer.
- `NotificationSubscriptionHandle`: opaque subscription identity,
  secret-reference bindings, host/provider class, consent state, expiry, and
  health.
- `NotificationDeliveryStatus`: accepted, queued, scheduled, sent, delivered,
  displayed, clicked, dismissed, acknowledged, expired, canceled, failed,
  unsupported, unavailable, partial, and unknown.
- `NotificationProviderCapability`: supported channels, target classes,
  scheduling, update/cancel, actions, subscription management, delivery
  inspection depth, payload/action limits, quota, rate limits, host limits, and
  health.

## Existing Macaca Platform Inventory

Current repository capabilities that can back notification service providers:

- Domain-pack descriptors and service descriptors already provide pack identity,
  command metadata, lifecycle state, health, and snapshot shape patterns.
- `macaca-kernel::service_call` enforces trace-required service execution and is
  the only acceptable execution path for notification side effects.
- `macaca-sdk::SystemFacade` and focused clients provide the Facade/Strategy
  pattern for SDK helpers and unavailable diagnostics.
- Existing unavailable/null-object clients provide examples for absent optional
  service behavior without silent fallback or fake success.
- Scheduler service command DTOs and trace validation can inform scheduled
  notification command boundaries while preserving notification as a service,
  not a kernel scheduler feature.
- Trace service descriptors, service-call trace emission, and audit-oriented
  persistence patterns can host sanitized publish, schedule, action, delivery,
  subscription, health, snapshot, and unavailable events.
- Runtime/framework permission command objects show a pattern for executable
  policy specifications before external or attention-affecting side effects.

No current evidence proves notification-specific DTOs, providers, admission,
SDK/WASM ABI, developer docs, or redaction gates; those remain unchecked tasks.

## Rejected Boundary Leakage

Macaca must reject:

- APNs payloads, device tokens, push types as OS semantics, FCM token/topic
  payloads, Web Push endpoints/keys, service-worker objects, Android channel
  objects, Windows toast XML, raw provider responses, and provider SDK models as
  stable SDK contracts.
- Application-specific notification copy, marketing campaign logic, reminder
  business workflows, or UI presentation rules in OS layers.
- Shell-owned notification delivery repair, fallback routing, action
  authorization, or host capability decisions.
- Raw tokens, endpoints, push keys, credentials, private keys, provider
  payloads, raw body content beyond bounded redaction, prompts, manifests, WASM
  bytes, package bytes, or unbounded provider diagnostics in observability
  surfaces.

All operations must enter through typed notification service commands with trace
context, consent/policy/resource/entitlement checks, approval where required,
structured result envelopes, sanitized audit, unavailable provider behavior,
idempotency, replay evidence, and provider replacement support.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
