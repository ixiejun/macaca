# Device Notifications Pack

`pack.device.notifications.v1` provides provider-neutral host notification
authorization, channel/category/action registration, posting, scheduling,
cancellation, pending/history inspection, badge updates, interaction
subscription, push-support inspection, and host notification status.

The pack handles local/user-visible host notifications. Gateway push delivery,
communication messaging, inbox processing, workflow scheduling, and
application-specific reminder logic remain separate capabilities.

## Manifest Declaration

```toml
[service_contract]
optional_packs = ["pack.device.notifications.v1"]
```

Unavailable optional declarations report
`device_notifications_provider_not_installed`.

## Commands

- `notifications.inspect_authorization` and `request_authorization`: inspect or
  request host-owned authorization.
- `notifications.register_channel` and `register_category`: declare channel,
  category, and action metadata.
- `notifications.post`, `schedule`, `cancel`, `list_pending`, and
  `inspect_history`: manage notification lifecycle.
- `notifications.set_badge` and `clear_badge`: manage bounded badge updates.
- `notifications.subscribe_interactions`: receive redacted interaction events.
- `notifications.inspect_push_support` and `inspect_host`: report host support
  and diagnostics without exposing push tokens.

## DTOs And Results

Core DTOs include `NotificationAuthorization`, `NotificationChannel`,
`NotificationCategory`, `NotificationAction`, `NotificationContent`,
`NotificationTrigger`, `NotificationDeliveryPolicy`, `NotificationRecord`,
`NotificationInteraction`, and `NotificationError`. Result statuses include
success, partial, denied, unavailable, unsupported, prompt-not-allowed,
channel-missing, category-missing, content-too-large,
sensitive-content-blocked, quota-exceeded, schedule-too-far,
background-action-denied, interaction-expired, host-disabled,
provider-failure, and conflict.

## Provider Mapping

Android Notifications, Apple UserNotifications, Web Notifications API, W3C Push
API support inspection, Windows App Notifications, and HarmonyOS Notification
Kit map into authorization state, channels, categories, actions, content
references, triggers, delivery policy, records, interactions, badge capability,
and host status. Raw bodies, action input, push tokens, provider payloads,
credentials, notification history, and interaction payloads are redacted.

## App-Facing Examples

Applications call the pack through typed notification lifecycle commands and
receive redacted records or interaction references. Each example assumes the app
already declared `pack.device.notifications.v1` and every command carries
trace, session, tenant, and capability context through the SDK facade.

- Inspect or request authorization with `notifications.inspect_authorization`
  and `notifications.request_authorization`, then use only neutral status
  values in app logic.
- Register a channel with `notifications.register_channel` and a category with
  `notifications.register_category` before posting category-scoped actions.
- Post a synthetic reminder with `notifications.post` using bounded content and
  redacted body references.
- Schedule a notification with `notifications.schedule`, then cancel it by
  `notification_record_id` with `notifications.cancel`.
- Set or clear a badge with `notifications.set_badge` and
  `notifications.clear_badge` only when host status reports support.
- Subscribe to interactions with `notifications.subscribe_interactions` and
  store only redacted action identifiers.
- Inspect push support with `notifications.inspect_push_support` without ever
  exposing push tokens to the app or traces.
- Display unavailable diagnostics from
  `device_notifications_provider_not_installed` without simulating delivery.

## Conformance

Provider authors must cover descriptor fields, host adapter responsibilities,
authorization, notification and interaction state machines, unsupported
behavior, redaction, health/snapshot behavior, replacement strategy,
unavailable behavior, and no raw notification body or push token leakage.
