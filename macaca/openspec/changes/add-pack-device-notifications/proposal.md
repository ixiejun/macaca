# Change: Add Industrial Device Notifications Pack

## Why

Macaca applications need `pack.device.notifications.v1` for safe host-mediated user notifications: local notifications, scheduled notifications, notification categories, actions, channels/topics, badges, quiet/critical delivery classes, interaction callbacks, pending notification management, and permission diagnostics. The current template is too shallow because it does not model platform authorization, channel/category registration, action routing, interruption policy, foreground/background constraints, or notification lifecycle evidence.

Notifications can interrupt users, reveal sensitive data on lock screens, trigger background actions, and bridge to push infrastructure. Macaca needs a provider-neutral service pack that treats notification delivery as a permissioned, auditable host capability rather than an application-owned side channel.

## Supplier/API Baseline

- Android Notifications: runtime `POST_NOTIFICATIONS` permission, notification channels, importance, actions, pending intents, foreground-service visibility, and notification drawer behavior. Official docs: https://developer.android.com/develop/ui/views/notifications and https://developer.android.com/develop/ui/compose/notifications/notification-permission
- Apple UserNotifications: authorization, notification requests, content, triggers, categories, actions, badges, sounds, provisional/critical behavior, and notification center callbacks. Official docs: https://developer.apple.com/documentation/usernotifications and https://developer.apple.com/documentation/usernotifications/unusernotificationcenter
- Web Notifications API and Push API: browser permission, notification display, actions, service workers, push subscriptions, background delivery, and user-agent restrictions. Official docs: https://developer.mozilla.org/docs/Web/API/Notifications_API and https://www.w3.org/TR/push-api/
- Windows App Notifications: local toast notifications, content templates, actions, activation handling, notification history, and cloud push separation. Official docs: https://learn.microsoft.com/windows/apps/develop/notifications/app-notifications/
- HarmonyOS Notification Kit: notification authorization, slots/channels, actions, badges, and distributed-device notification mediation. Official docs: https://developer.huawei.com/consumer/en/doc/harmonyos-guides/notification-overview

## Macaca Provider-Neutral Mapping

Macaca SHALL normalize platform concepts:

- Permission and host status become `notifications.inspect_authorization` and `notifications.request_authorization`.
- Channel/category/action registration becomes `notifications.register_channel` and `notifications.register_category`.
- Local/scheduled notification delivery becomes `notifications.post`, `notifications.schedule`, `notifications.cancel`, and `notifications.list_pending`.
- Badge state becomes `notifications.set_badge` and `notifications.clear_badge`.
- User interactions become `notifications.subscribe_interactions` and canonical service events.
- Push subscription metadata becomes `notifications.inspect_push_support`; actual remote push gateway delivery belongs to gateway/communication packs unless explicitly serviceized.

## What Changes

- Add `pack.device.notifications.v1` as a service-backed industrial pack under the device family.
- Define command DTOs for authorization, channel/category registration, posting, scheduling, cancellation, pending/history inspection, badge management, interaction subscription, push-support inspection, and host status.
- Define DTOs for notification content, trigger, channel, category, action, delivery policy, redaction policy, interaction, badge state, authorization state, and structured errors.
- Define permission scopes, approval rules, interruption classes, lock-screen redaction, background action restrictions, scheduling/resource quotas, and unavailable diagnostics.
- Require detailed developer documentation under `docs/developer-packs/device/notifications.md`.

## Impact

- Affected specs: `pack-device-notifications`, `developer-pack-industrial-capability-catalog`, `sdk-system-facade`, `service-runtime`, `unified-execution-path`.
- Later affected code: protocol DTOs, descriptor/admission validators, SDK pack client, notification service provider contract, host notification adapters, mock/unavailable providers, interaction event bridge, trace/audit schemas, and boundary gates.
- Validation: `openspec validate add-pack-device-notifications --strict`, authorization tests, channel/category tests, scheduling tests, cancellation tests, interaction replay tests, redaction tests, no-direct-provider-call gates, and docs coverage checks.

## Non-Goals

- This pack does not own email, SMS, chat messaging, inbox storage, gateway push provider routing, marketing campaign logic, application-specific notification ranking, or business workflow reminders.
- This pack does not hardcode Android, Apple, Windows, browser, HarmonyOS, channel names, action names, provider names, or application workflows into OS-layer routing.
- This pack does not expose raw notification bodies, secrets, credentials, push tokens, raw provider payloads, or unbounded interaction data in traces, audits, snapshots, logs, or examples.
