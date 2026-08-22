## 1. Research, Scope, And Governance

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design patterns, the umbrella industrial catalog proposal, and this child proposal before implementation.
- [x] 1.2 Record supplier/API comparison notes for Android Notifications, Apple UserNotifications, Web Notifications API, W3C Push API, Windows App Notifications, and HarmonyOS Notification Kit.
- [x] 1.3 Confirm boundaries with communication notification/messaging/inbox packs, gateway push providers, foreground/background host capabilities, workflow schedule, and application lifecycle services.
- [x] 1.4 Record GitNexus CRITICAL/HIGH findings as memo-only evidence before implementation commits, per the current refactor instruction.

## 2. Contract, Descriptor, And DTO Schema

- [x] 2.1 Define provider-neutral commands for `notifications.inspect_authorization`, `notifications.request_authorization`, `notifications.register_channel`, `notifications.register_category`, `notifications.post`, `notifications.schedule`, `notifications.cancel`, `notifications.list_pending`, `notifications.inspect_history`, `notifications.set_badge`, `notifications.clear_badge`, `notifications.subscribe_interactions`, `notifications.inspect_push_support`, and `notifications.inspect_host`.
- [x] 2.2 Define `NotificationAuthorization`, `NotificationChannel`, `NotificationCategory`, `NotificationAction`, `NotificationContent`, `NotificationTrigger`, `NotificationDeliveryPolicy`, `NotificationRecord`, `NotificationInteraction`, and `NotificationError`.
- [x] 2.3 Define typed success, partial, denied, unavailable, unsupported, prompt-not-allowed, channel-missing, category-missing, content-too-large, sensitive-content-blocked, quota-exceeded, schedule-too-far, background-action-denied, interaction-expired, host-disabled, provider-failure, and conflict results.
- [x] 2.4 Define descriptor metadata for pack id, family, lifecycle, command schemas, authorization states, channel/category support, action limits, schedule limits, badge support, history support, interaction support, permission scopes, policy template, resource budgets, SDK metadata, compatibility, diagnostics, and documentation URL.
- [x] 2.5 Add stable descriptor hashing, version compatibility checks, DTO snapshot fixtures, authorization fixtures, scheduling fixtures, interaction fixtures, redaction fixtures, and schema migration tests.

## 3. Permission, Policy, Resource, Entitlement, And Approval

- [x] 3.1 Implement declaration validation for `device.notifications.read_status`, `device.notifications.request_permission`, `device.notifications.post`, `device.notifications.schedule`, `device.notifications.manage`, and `device.notifications.interactions`.
- [ ] 3.2 Enforce authorization, channel/category, content size, redaction, lock-screen, interruption class, quiet hours, foreground/background, scheduling, badge, and interaction policies before dispatch.
- [x] 3.3 Require explicit delivery policy and redaction class for every posted or scheduled notification.
- [ ] 3.4 Add resource reservation and quota checks for pending notification count, schedule horizon, interaction subscription count, content size, action count, badge updates, retained snapshots, and replay metadata.
- [ ] 3.5 Add approval behavior for critical/urgent delivery, sensitive lock-screen content, background actions, remote-host notification delivery, and high-volume notification batches.
- [ ] 3.6 Add tests proving denied, unavailable, prompt-not-allowed, content-blocked, background-action-denied, interaction-expired, and quota paths do not call concrete providers or leak content.

## 4. Service Provider, Notification, And Interaction Strategy

- [x] 4.1 Implement the device notification service provider contract behind the service runtime; do not construct providers from kernel, SDK, shells, or generic application-framework code.
- [x] 4.2 Add provider descriptor support for host-native, browser, remote-host, plugin, mock, and unavailable provider classes.
- [ ] 4.3 Add authorization, notification lifecycle, pending schedule, and interaction subscription state machines.
- [x] 4.4 Add mock and unavailable providers for deterministic tests; host-specific adapters must remain optional providers or plugin/remote modules.
- [ ] 4.5 Add provider conformance tests for authorization, channel/category registration, post, schedule, cancel, pending/history, badge, interaction events, push-support inspection, redaction, and unsupported-command reporting.
- [ ] 4.6 Add lifecycle, health, snapshot, shutdown, timeout, cancellation, schedule expiry, interaction expiry, resource cleanup, and bounded output behavior.

## 5. SDK, Admission, Examples, And ABI

- [x] 5.1 Extend SDK discovery for `pack.device.notifications.v1` with command schemas, DTO schemas, permission scopes, examples, availability, authorization state, channel/category support, schedule limits, badge support, interaction support, diagnostics, compatibility, and documentation URL.
- [ ] 5.2 Extend application admission so required declarations block when unavailable/disabled and optional declarations degrade explicitly with effective capability mementos.
- [x] 5.3 Add SDK command helper builders that only produce canonical traced service calls and never construct providers or branch on host/platform names.
- [ ] 5.4 Add WASM/application ABI exposure for notification commands using provider-neutral DTO schemas and canonical service-call dispatch.
- [x] 5.5 Add generic examples for authorization, channel/category registration, post, schedule, cancel, badge, interaction handling, push-support inspection, and unavailable-provider diagnostics.

## 6. Trace, Audit, Replay, And Boundary Gates

- [ ] 6.1 Emit sanitized `notifications.pack_declared`, `notifications.admission_validated`, `notifications.policy_decision`, `notifications.authorization_requested`, `notifications.authorization_changed`, `notifications.channel_registered`, `notifications.category_registered`, `notifications.notification_posted`, `notifications.notification_scheduled`, `notifications.notification_cancelled`, `notifications.interaction_received`, `notifications.badge_updated`, `notifications.command_failed`, `notifications.unavailable`, and `notifications.snapshot_recorded` events.
- [x] 6.2 Add replay tests proving every command and interaction event is trace-addressable through the canonical service path after refresh/restart without raw notification bodies.
- [x] 6.3 Add dependency-boundary gates proving microkernel, SDK, shells, and generic application framework do not import concrete notification providers or host notification APIs.
- [x] 6.4 Add no-direct-provider-call gates proving all notification commands enter through descriptor-owned service registrations and typed service runtime dispatch.
- [x] 6.5 Add redaction tests for title/body text, action input, push tokens, provider payloads, credentials, notification history, interaction events, snapshots, and diagnostics.
- [ ] 6.6 Run `openspec validate add-pack-device-notifications --strict`, DTO compatibility tests, authorization tests, scheduling tests, interaction replay tests, boundary gates, file-size gates, and audit replay checks before marking implementation tasks complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/device/notifications.md` with purpose, manifest declarations, required/optional behavior, scopes, command DTOs, result DTOs, authorization, channels, categories, actions, content redaction, delivery policy, scheduling, cancellation, badges, interactions, push-support boundary, unavailable diagnostics, and trace/audit behavior.
- [x] 7.2 Add provider author documentation covering descriptor fields, host adapter responsibilities, authorization/notification/interaction state machines, conformance tests, unsupported behavior, redaction rules, health/snapshot behavior, and replacement strategy.
- [x] 7.3 Add minimal app-facing examples for request authorization, register channel/category, post, schedule, cancel, badge, interaction handling, and unavailable-provider diagnostics using generic synthetic data.
- [x] 7.4 Cross-link the guide from SDK discovery metadata and the industrial pack catalog index before marking `add-pack-device-notifications` complete.
