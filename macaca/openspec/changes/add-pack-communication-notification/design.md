# Communication Notification Pack Design

## Context

`pack.communication.notification.v1` exposes notifications as a Macaca OS
serviceized capability. It must support local, push, scheduled, in-app,
interactive, dismissible, and inspectable notifications without making the
microkernel, SDK, shells, or application framework depend on any concrete
provider.

Notifications are not just display primitives. They involve consent, user
attention, host capabilities, device tokens, push credentials, background
execution, service worker or platform activation, provider quotas, delivery
uncertainty, and action callbacks. The pack therefore uses Macaca's service
runtime as the only execution path and models every side effect as a typed
command with trace, policy, resource, entitlement, approval, redaction, and
replay evidence.

## Supplier Capability Matrix

| Supplier/platform | Industrial capability | Macaca abstraction |
| --- | --- | --- |
| Apple UserNotifications/APNs | Authorization status, local notification request, trigger, category, action, remote payload, foreground/focus behavior | `NotificationConsentState`, `NotificationSchedule`, `NotificationActionDefinition`, `NotificationDeliveryChannel`, host capability diagnostics |
| Android Notifications/FCM | Runtime permission, channels, actions, grouping, badges, time-sensitive flows, update/cancel, token/topic targeting | Policy profile, urgency, group key, badge metadata, action callback, replace/update semantics, target descriptor |
| Firebase Cloud Messaging | Notification vs data messages, token/topic/condition targets, platform overrides, HTTP v1 send, foreground/background receive, delivery metrics | Payload class, target class, bounded provider options, delivery handle, delivery inspection, quota and scale diagnostics |
| Web Notifications/Push | Permission, service worker, push subscription endpoint/keys, notification actions, secure context, background push | Host support check, subscription handle, secret-reference endpoint storage, action event bridge, unavailable diagnostics |
| Windows App Notifications | Toast content, local send, interactive activation, action input, notification management, platform limitations | Local host provider, action input DTO, activation event, unsupported-state diagnostics |

## Goals

- Provide a stable pack id `pack.communication.notification.v1` and command
  namespace `notification.*`.
- Support publish, schedule, cancel, update, list, action registration,
  subscription registration, acknowledgement, dismissal, and delivery inspection.
- Keep provider-specific extension points declarative and bounded while the core
  DTOs remain portable.
- Make consent, policy, host support, provider health, delivery state, and
  unavailable diagnostics visible through SDK discovery.
- Emit replayable, sanitized trace/audit evidence for every declaration,
  admission decision, command, policy decision, provider call, action callback,
  snapshot, and unavailable state.
- Require industrial developer documentation with manifest examples, DTO
  examples, failure handling, provider replacement, and trace/audit guidance.

## Non-Goals

- Do not implement a provider-specific APNs, FCM, Web Push, Windows, or Android
  adapter in this proposal.
- Do not expose raw provider payloads, raw device tokens, push subscription
  endpoints, push keys, credentials, private keys, secrets, package bytes, raw
  manifests, or unbounded body content in logs, traces, snapshots, SDK
  diagnostics, or examples.
- Do not make notification UI semantics an OS concern. Applications own product
  copy and presentation intent; host providers own platform rendering limits.
- Do not silently downgrade push to local, local to in-app, or one provider to
  another provider. Fallback requires explicit policy and traceable result
  metadata.

## Ownership And Boundaries

- Pack id: `pack.communication.notification.v1`.
- Family: `communication`.
- Backing service owner: notification service provider.
- SDK surface: `sdk.packs.communication.notification`.
- Command namespace: `notification.*`.
- Microkernel owns only identity, policy facade, service-call evidence,
  trace/audit primitives, scheduler/resource primitives, and registry metadata.
- Application framework owns manifest declarations, app-scoped permissions,
  effective capability projection, and application lifecycle binding.
- Runtime host owns provider registration, service decorators, provider adapter
  composition, and sanitized diagnostics through approved composition roots.
- Shells render notification pack availability and action callback evidence but
  do not own notification semantics.

## Command Surface

| Command | Purpose | Idempotency and side-effect rule |
| --- | --- | --- |
| `notification.publish` | Publish an immediate local, push, in-app, or provider-routed notification | Requires `client_request_id`; duplicate requests return the original delivery handle |
| `notification.schedule` | Schedule a notification for future wall-clock or relative delivery | Requires schedule idempotency key, timezone policy, expiry, and cancellation handle |
| `notification.update` | Replace or patch a previously published/scheduled notification | Requires notification handle and provider capability check |
| `notification.cancel` | Cancel scheduled or active notifications by handle, tag, group, or bounded query | Requires scoped target and audit reason |
| `notification.list_notifications` | List active, scheduled, dismissed, or failed notifications visible to the app/session | Must return bounded pages and redacted content |
| `notification.register_action` | Register an action definition and callback route for a category/profile | Requires action scope, callback capability, and replay event schema |
| `notification.unregister_action` | Remove an action definition or callback route | Must preserve historical audit records |
| `notification.acknowledge` | Mark a notification as seen, acted on, or processed by an app/session | Must not fake provider delivery acknowledgement |
| `notification.dismiss` | Record or request user/app dismissal of a notification | Must distinguish host dismissal from app-side acknowledgement |
| `notification.inspect_delivery` | Inspect delivery state, provider receipt, retry state, and failure reason | Requires inspection permission and redacted provider evidence |
| `notification.register_subscription` | Register a device/browser/push subscription using secret references and capability metadata | Must store only opaque handles and secret references |
| `notification.revoke_subscription` | Revoke or disable a subscription handle | Must revoke provider state when supported and record unavailable/partial states |

Every command defines a typed command DTO, typed success result, typed denied
result, typed unavailable result, typed unsupported result, typed conflict
result, typed quota result, typed provider failure result, redaction profile,
idempotency model, and replay metadata.

## DTO Model

Core DTOs:

- `NotificationMessage`: title, body, subtitle, summary, locale, content
  sensitivity, bounded data map, media handles, category id, thread/group key,
  collapse key, badge metadata, expiry, urgency, and provider option reference.
- `NotificationTarget`: app/session/user/tenant scoped target, device handle,
  subscription handle, topic handle, group handle, condition expression, or
  in-app surface handle. Raw device tokens and push endpoints are forbidden.
- `NotificationDeliveryChannel`: `local`, `push`, `in_app`, `host`,
  `provider_remote`, or `auto_policy`, where `auto_policy` must emit the
  selected channel in trace metadata.
- `NotificationSchedule`: immediate, absolute wall-clock, monotonic delay,
  recurring rule reference, timezone, deadline, expiry, and drift policy.
- `NotificationActionDefinition`: action id, title, semantic role, requires
  foreground, destructive flag, authentication requirement, text input schema,
  callback route, and policy profile.
- `NotificationActionEvent`: notification handle, action id, bounded user input,
  delivery context, app/session/task ids, trace id, and replay pointer.
- `NotificationDeliveryStatus`: accepted, queued, scheduled, sent, delivered,
  displayed, clicked, dismissed, acknowledged, expired, canceled, failed,
  partial, unsupported, unavailable, or unknown.
- `NotificationProviderCapability`: supported channels, target classes, action
  support, scheduling support, update/cancel support, delivery inspection depth,
  max payload, max actions, rate limits, host limitations, and health state.

## Permission, Policy, Resource, Entitlement, And Approval Model

Permission scopes:

- `notification.publish`
- `notification.schedule`
- `notification.update`
- `notification.cancel`
- `notification.action.register`
- `notification.action.receive`
- `notification.subscription.manage`
- `notification.delivery.inspect`
- `notification.host.surface`

Policy defaults:

- Commands are scoped to application id, tenant id, session id, task id, and
  trace id when available.
- Publish/schedule/update commands require consent state, recipient/target
  scope validation, content sensitivity classification, rate limit checks,
  payload-size checks, and host/provider capability checks.
- Push subscription commands require secret-reference storage and must reject raw
  token, raw endpoint, raw key, or raw credential values.
- Action registration requires callback route declaration and prevents action
  callbacks from bypassing application capability checks.
- Delivery inspection requires a separate permission because provider receipts
  may reveal user/device behavior.
- External, cross-tenant, high-frequency, time-sensitive, destructive, or
  background side effects may require approval according to policy.

## SDK Discovery And Developer Documentation

SDK discovery returns:

- Pack id, family, version, lifecycle, service mapping, provider class, health,
  availability, command schemas, permission scopes, policy templates, resource
  budgets, compatibility, examples, unavailable diagnostics, redaction profile,
  and documentation links.
- Command helper builders that only build canonical traced service calls. SDK
  code must not construct providers or decide fallback routing.
- Null Object behavior for unavailable providers that returns explicit
  unavailable diagnostics and does not create side effects.

Developer documentation must be delivered at
`docs/developer-packs/communication/notification.md` and must cover manifest
declaration, permission scopes, command DTOs, result DTOs, idempotency,
subscription security, action callbacks, provider replacement, unavailable
diagnostics, trace/audit interpretation, and generic examples.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `notification_pack_declared`
- `notification_pack_admission_validated`
- `notification_pack_policy_decision`
- `notification_pack_consent_checked`
- `notification_pack_resource_reserved`
- `notification_pack_service_call_requested`
- `notification_pack_provider_call_started`
- `notification_pack_provider_call_succeeded`
- `notification_pack_provider_call_failed`
- `notification_pack_action_received`
- `notification_pack_delivery_status_changed`
- `notification_pack_subscription_registered`
- `notification_pack_unavailable`
- `notification_pack_snapshot_recorded`

Events include pack id, service id, command name, trace id, application/session/
task/tenant identifiers when available, sanitized target class, selected
delivery channel, provider class, bounded capability hash, policy decision,
latency, quota counters, result code, and replay pointer. Events must not include
raw body text unless explicitly redacted/bounded by policy, raw push payloads,
tokens, endpoints, credentials, or provider responses.

Snapshots include descriptor version, command availability, provider health,
consent capability state, subscription handle counts, action definitions,
policy template hash, resource counters, delivery status aggregates, and
sanitized replay pointers.

## Design Patterns

- **Facade**: SDK clients expose discovery and command builders only.
- **Command**: every operation is a typed command/result DTO.
- **Strategy**: providers, routing, delivery channel selection, policy profiles,
  and unavailable behavior are replaceable.
- **Decorator**: trace, policy, consent, entitlement, resource, approval,
  metering, and redaction wrap every service call.
- **Specification**: admission validates manifest declarations, permissions,
  service mapping, provider capability, and version compatibility.
- **Observer**: action callbacks, delivery status, health, trace, and audit
  events are subscribable.
- **Memento**: effective capability reports, snapshots, delivery handles, and
  replay pointers preserve recovery state.
- **Abstract Factory**: provider adapters are created only by approved runtime
  host composition roots.

## Risks And Mitigations

- Risk: notification semantics drift into shell/UI code. Mitigation: shells only
  render capability state and action evidence; all commands go through SDK and
  service runtime.
- Risk: raw tokens or push endpoints leak through DTOs. Mitigation: only opaque
  handles and secret references are accepted; validators reject raw secret-like
  fields.
- Risk: providers have incompatible delivery guarantees. Mitigation: expose
  capability depth and delivery status confidence explicitly instead of
  normalizing to fake success.
- Risk: fallback hides a failed provider. Mitigation: fallback is a policy
  decision with trace metadata and result provenance.
- Risk: high-volume push creates quota or abuse issues. Mitigation: resource
  budgets, rate limits, approval gates, delivery inspection scopes, and bounded
  snapshots are mandatory.
