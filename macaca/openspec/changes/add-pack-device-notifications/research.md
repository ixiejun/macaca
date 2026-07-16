# Device Notifications Pack Research

## Purpose

This note records supplier/API comparison, Macaca provider-neutral mapping,
boundary decisions, existing platform inventory, and GitNexus memo evidence for
`pack.device.notifications.v1`. The notifications pack must expose
authorization, channel/category registration, local and scheduled notification
delivery, cancellation, pending/history inspection, badge updates, interaction
subscriptions, push-display support inspection, host status, and redaction
through typed service commands. It must not own email, SMS, messaging, inbox,
gateway push routing, campaign logic, or application-specific notification
ranking.

## Source Baseline

- Android Notifications and notification permission:
  <https://developer.android.com/develop/ui/views/notifications> and
  <https://developer.android.com/develop/ui/compose/notifications/notification-permission>
- Apple UserNotifications:
  <https://developer.apple.com/documentation/usernotifications> and
  <https://developer.apple.com/documentation/usernotifications/unusernotificationcenter>
- Web Notifications API and W3C Push API:
  <https://developer.mozilla.org/docs/Web/API/Notifications_API> and
  <https://www.w3.org/TR/push-api/>
- Windows App Notifications:
  <https://learn.microsoft.com/windows/apps/develop/notifications/app-notifications/>
- HarmonyOS Notification Kit:
  <https://developer.huawei.com/consumer/en/doc/harmonyos-guides/notification-overview>

## Supplier API Notes

- Android contributes runtime notification permission, channels, importance,
  actions, pending intents, foreground-service visibility, and drawer behavior.
  Macaca should normalize channels, actions, and interaction callbacks while
  keeping Android pending-intent mechanics provider-private.
- Apple UserNotifications contributes authorization states, notification
  requests, content, triggers, categories, actions, badges, sounds,
  provisional/critical behavior, and notification center callbacks. Macaca
  should model interruption classes, categories, actions, and badge capability.
- Web Notifications and Push contribute browser permissions, notification
  display, actions, service-worker events, push subscriptions, and user-agent
  restrictions. Macaca should inspect push-display support while keeping remote
  push provider routing in gateway/communication packs.
- Windows App Notifications contribute local toast content, activation,
  actions, notification history, and cloud push separation. Macaca should
  normalize history and activation as bounded records and interactions.
- HarmonyOS Notification Kit contributes authorization, slots/channels,
  actions, badges, and distributed notification mediation. Macaca should hide
  slot/channel implementation details behind provider capabilities.

## Macaca-Owned Abstractions

`pack.device.notifications.v1` should define `NotificationAuthorization`,
`NotificationChannel`, `NotificationCategory`, `NotificationAction`,
`NotificationContent`, `NotificationTrigger`, `NotificationDeliveryPolicy`,
`NotificationRecord`, `NotificationInteraction`, and `NotificationError`.

The DTOs must carry authorization state, prompt eligibility, channel/category
metadata, action limits, content redaction class, trigger type, timezone
semantics, quiet-hours policy, interruption class, lock-screen redaction,
badge support, interaction expiry, host disabled reason, bounded provider
reason codes, and replay pointers. Raw notification bodies, raw user input,
push tokens, credentials, secrets, raw provider payloads, and unbounded
interaction data are rejected.

## Boundary Decisions

- Communication notification/messaging/inbox packs own cross-user or remote
  communication semantics; device notifications own host-mediated user
  notification display and local interaction surfaces.
- Gateway push providers own remote transport, token management, routing, and
  push provider adapters unless separately serviceized.
- Foreground/background host owns lifecycle and background-action eligibility;
  notifications consume that evidence for sensitive actions and scheduling.
- Workflow schedule owns workflow timers and reminders; notifications may
  schedule host display but does not own workflow execution.
- Application lifecycle services own app state; notifications emit interaction
  events through canonical service events without embedding app workflow logic.

## Existing Macaca Platform Inventory

- `crates/foundation/macaca-proto/src/domain_pack_contract/` provides reusable
  descriptor, lifecycle, availability, diagnostics, policy, SDK metadata, and
  unavailable diagnostic structures.
- `crates/facade/macaca-sdk/src/system_facade.rs` provides the Facade pattern
  for upper layers; notification SDK helpers should only create canonical traced
  service calls.
- `crates/runtime/macaca-host-composition/src/runtime_host.rs` and
  `crates/kernel/macaca-kernel/src/domain_pack_registration.rs` provide generic
  provider registration/composition mechanics.
- Kernel policy, audit, trace, and redaction modules provide reusable
  enforcement and observability substrate, but current evidence does not prove
  notification-specific DTOs, descriptors, providers, SDK helpers, ABI, tests,
  or docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
