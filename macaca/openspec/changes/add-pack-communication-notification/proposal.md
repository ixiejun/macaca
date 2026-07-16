# Change: Add Industrial Communication Notification Pack

## Why

Applications need notifications as a real operating-system capability, not as a
demo helper or catalog label. A production notification pack must cover local
display, push routing, scheduled delivery, in-app delivery, action callbacks,
dismissal/acknowledgement, delivery inspection, provider replacement, consent,
quota, replay, and developer documentation.

The pack must also preserve Macaca OS boundaries. Notifications are user-visible
and often cross host, device, network, identity, and consent boundaries, so every
operation must flow through typed service commands, policy decorators, resource
checks, entitlement checks, sanitized trace/audit events, and structured
unavailable behavior.

## Supplier And Platform API Research

This proposal uses official platform and vendor documentation as supplier-grade
input and maps shared concepts into Macaca abstractions:

- Apple UserNotifications defines notification authorization, categories,
  custom actions, local scheduling, remote APNs delivery, and foreground/focus
  behavior. Macaca maps this to consent state, action registration, scheduled
  notification requests, push provider routing, and host capability diagnostics.
  Sources: Apple UserNotifications, permission, actionable notification, APNs
  request documentation.
- Android Notifications defines notification channels, runtime notification
  permission, posting/updating/canceling, grouping, actions through intents,
  time-sensitive notification behavior, badges, and conversation-specific
  affordances. Macaca maps this to channel-like policy profiles, host permission
  state, replace/update semantics, action callbacks, urgency, grouping, badge
  metadata, and host surface capability reporting.
- Firebase Cloud Messaging separates notification messages from data messages,
  supports token/topic/condition targeting, platform-specific overrides, HTTP v1
  send requests, foreground/background receive behavior, delivery analytics, and
  scale guidance. Macaca maps this to target descriptors, payload classes,
  platform override policy, push delivery handles, delivery inspection, bounded
  payload limits, and provider quota diagnostics.
- Web Notifications and Push APIs require permission, service worker
  registration, push subscriptions with endpoints and keys, notification
  actions, secure-context behavior, and background handling through service
  workers. Macaca maps this to host support checks, subscription handles,
  secure-context diagnostics, action callback routing, and secret-reference-only
  endpoint storage.
- Windows App Notifications/Toast APIs support local app notifications,
  interactive content, action activation, notification management, and host
  limitations. Macaca maps this to local host provider capability, action input
  DTOs, activation events, provider limitations, and structured unsupported
  states.

The Macaca contract deliberately avoids copying provider-specific API names into
OS semantics. Provider-specific fields are kept in declarative, bounded
`provider_options` maps owned by provider adapters, while the pack-level DTOs
remain stable, typed, auditable, and portable.

## What Changes

- Add provider-neutral `pack.communication.notification.v1` as an industrial
  service-backed pack.
- Define notification DTOs for message content, targets, delivery channel,
  priority/urgency, schedule, expiry, collapse key, grouping, badge metadata,
  actions, action input, acknowledgement, cancellation, delivery status, and
  provider capabilities.
- Define commands for `notification.publish`, `notification.schedule`,
  `notification.cancel`, `notification.list_notifications`,
  `notification.register_action`, `notification.unregister_action`,
  `notification.acknowledge`, `notification.dismiss`, `notification.update`,
  `notification.inspect_delivery`, `notification.register_subscription`, and
  `notification.revoke_subscription`.
- Define permission scopes for publish, schedule, action callback,
  subscription management, delivery inspection, and host notification surfaces.
- Require policy, consent, entitlement, quota, idempotency, redaction,
  unavailable behavior, snapshot, replay, and delivery audit for every command.
- Require a detailed developer guide under
  `docs/developer-packs/communication/notification.md` before this pack can be
  marked complete.

## Impact

- Affected specs: `pack-communication-notification`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected future code: provider-neutral proto DTOs, pack descriptors,
  manifest/admission validators, SDK discovery metadata, focused SDK clients,
  service runtime decorators, notification service providers, unavailable/mock
  providers, trace/audit schema, replay tests, and dependency-boundary gates.
- Non-goals: no application-specific notification workflow, no provider-name
  routing in OS layers, no raw device token exposure, no raw push credential
  exposure, no concrete provider construction in kernel/SDK/shells, and no fake
  success when providers or host capabilities are absent.

## References

- Apple UserNotifications: https://developer.apple.com/documentation/usernotifications
- Apple notification permission:
  https://developer.apple.com/documentation/usernotifications/asking-permission-to-use-notifications
- Apple actionable notifications:
  https://developer.apple.com/documentation/usernotifications/declaring-your-actionable-notification-types
- Apple APNs requests:
  https://developer.apple.com/documentation/usernotifications/sending-notification-requests-to-apns
- Android notifications:
  https://developer.android.com/develop/ui/views/notifications
- Firebase Cloud Messaging:
  https://firebase.google.com/docs/cloud-messaging
- FCM message types:
  https://firebase.google.com/docs/cloud-messaging/customize-messages/set-message-type
- MDN Notifications API:
  https://developer.mozilla.org/en-US/docs/Web/API/Notifications_API
- MDN Push API: https://developer.mozilla.org/en-US/docs/Web/API/Push_API
- Windows App Notifications:
  https://learn.microsoft.com/en-us/windows/apps/develop/notifications/app-notifications/
