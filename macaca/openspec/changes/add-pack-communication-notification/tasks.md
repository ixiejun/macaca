## 1. Supplier API Research And Scope

- [x] 1.1 Re-read architecture governance, microkernel boundaries,
  serviceization allowlist, design-pattern guidance, and the industrial catalog
  umbrella proposal before implementation.
- [x] 1.2 Record API notes for Apple UserNotifications/APNs, Android
  Notifications, Firebase Cloud Messaging, Web Notifications/Push, and Windows
  App Notifications, including permissions, scheduling, actions, subscriptions,
  delivery inspection, host limitations, and provider quota behavior.
- [x] 1.3 Map supplier concepts to Macaca provider-neutral DTOs and explicitly
  reject provider-specific fields that would become OS semantics.
- [x] 1.4 Inventory existing service descriptors, SDK clients, admission paths,
  trace/audit schemas, optional providers, mock providers, and unavailable
  providers that can host `pack.communication.notification.v1`.
- [x] 1.5 Record GitNexus CRITICAL/HIGH findings as memo only before
  implementation commits.

## 2. Contract, Descriptor, And DTOs

- [x] 2.1 Define provider-neutral DTOs for `NotificationMessage`,
  `NotificationTarget`, `NotificationDeliveryChannel`, `NotificationSchedule`,
  `NotificationActionDefinition`, `NotificationActionEvent`,
  `NotificationDeliveryStatus`, `NotificationProviderCapability`,
  `NotificationSubscriptionHandle`, and `NotificationDeliveryHandle`.
- [x] 2.2 Define typed command DTOs for `notification.publish`,
  `notification.schedule`, `notification.update`, `notification.cancel`,
  `notification.list_notifications`, `notification.register_action`,
  `notification.unregister_action`, `notification.acknowledge`,
  `notification.dismiss`, `notification.inspect_delivery`,
  `notification.register_subscription`, and `notification.revoke_subscription`.
- [x] 2.3 Define typed success, partial, denied, unavailable, unsupported,
  conflict, quota, timeout, canceled, and provider-failure result DTOs.
- [x] 2.4 Define descriptor metadata for pack id, family, lifecycle, stability,
  command schemas, permission scopes, policy templates, resource budgets,
  delivery guarantees, subscription security, redaction rules, SDK metadata,
  compatibility, diagnostics, and documentation links.
- [x] 2.5 Add descriptor hash, version compatibility, schema compatibility, and
  redaction-profile tests.

## 3. Admission, Permission, Policy, Resource, And Approval

- [x] 3.1 Implement manifest declaration validation for notification permissions:
  `notification.publish`, `notification.schedule`, `notification.update`,
  `notification.cancel`, `notification.action.register`,
  `notification.action.receive`, `notification.subscription.manage`,
  `notification.delivery.inspect`, and `notification.host.surface`.
- [x] 3.2 Enforce consent, host support, provider health, target scope, content
  sensitivity, payload size, action count, schedule horizon, delivery channel,
  rate limit, resource budget, entitlement, and approval checks before side
  effects.
- [x] 3.3 Reject raw tokens, raw push endpoints, raw push keys, raw credentials,
  raw provider payloads, and unbounded notification content at admission and
  service boundaries.
- [x] 3.4 Model required declarations as readiness blockers and optional
  declarations as explicit degraded effective capabilities.
- [x] 3.5 Add tests proving denied, quota, unsupported, and unavailable paths do
  not call concrete providers.

## 4. Service Provider And Runtime Integration

- [x] 4.1 Implement or bind notification service providers only through the
  service runtime and approved runtime-host composition roots.
- [x] 4.2 Add unavailable and mock providers with deterministic behavior for all
  commands and delivery states.
- [x] 4.3 Add lifecycle, health, snapshot, shutdown, timeout, cancellation,
  idempotency, retry metadata, delivery handle, and bounded pagination support.
- [x] 4.4 Add provider capability reporting for local/push/in-app channels,
  target classes, scheduling, update/cancel, action callbacks, subscription
  management, delivery inspection depth, payload limits, action limits, and rate
  limits.
- [x] 4.5 Add canonical execution-path tests proving every notification command
  traverses SDK/facade, service runtime decorators, and provider dispatch exactly
  once.

## 5. SDK, WASM ABI, Application Framework, And Examples

- [x] 5.1 Extend SDK discovery for `pack.communication.notification.v1` with
  command schemas, provider capability reports, examples, availability,
  diagnostics, docs metadata, policy templates, and compatibility.
- [x] 5.2 Add focused SDK helper builders that only produce canonical traced
  service calls and return Null Object unavailable diagnostics when the pack is
  absent.
- [x] 5.3 Extend WASM/application ABI metadata so applications can declare
  notification capabilities, receive action events, and inspect delivery handles
  only through declared permissions.
- [x] 5.4 Add generic examples for immediate publish, scheduled notification,
  action callback, push subscription registration, unavailable provider handling,
  and delivery inspection without hardcoded application or provider behavior.

## 6. Trace, Audit, Replay, Security, And Gates

- [x] 6.1 Emit sanitized declaration, admission, consent, policy, resource,
  entitlement, approval, service-call, provider-call, action, delivery-status,
  subscription, health, snapshot, and unavailable events.
- [x] 6.2 Add replay tests proving notification calls, action callbacks,
  subscription changes, and delivery status changes are trace-addressable
  through the canonical service path.
- [x] 6.3 Add sanitization tests proving traces, audits, snapshots, SDK
  diagnostics, and examples do not leak raw tokens, raw endpoints, raw keys,
  credentials, raw provider payloads, private data, or unbounded content.
- [x] 6.4 Add dependency gates proving kernel, SDK, shells, and generic
  application framework do not import concrete notification providers.
- [x] 6.5 Run `openspec validate add-pack-communication-notification --strict`,
  targeted cargo tests, boundary gates, file-size gates, canonical execution-path
  tests, and audit replay checks before marking complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/communication/notification.md` with pack
  purpose, platform comparison, manifest declaration, permission scopes, command
  DTOs, result DTOs, idempotency, action callbacks, subscription security,
  delivery inspection, provider replacement, unavailable diagnostics, trace/audit
  interpretation, and operational limits.
- [x] 7.2 Include generic app-facing examples for publish, schedule, update,
  cancel, register action, register subscription, inspect delivery, and handle
  unavailable provider results.
- [x] 7.3 Include provider-author guidance for descriptor metadata, capability
  reporting, redaction, delivery status confidence, health checks, snapshots,
  quota reporting, and conformance tests.
- [x] 7.4 Cross-link the guide from SDK discovery metadata and the industrial
  pack catalog index before marking `add-pack-communication-notification`
  complete.
