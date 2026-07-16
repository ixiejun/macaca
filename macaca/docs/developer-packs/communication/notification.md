# Communication Notification Pack

`pack.communication.notification.v1` defines local, push, in-app, scheduled,
interactive, subscription-backed, and inspectable notification operations.
Applications own copy and presentation intent; Macaca owns declaration,
permission, policy, trace, audit, provider replacement, and unavailable
diagnostics.

## Manifest Declaration

```yaml
service_contract:
  optional_packs:
    - pack.communication.notification.v1
```

No installed provider returns `notification_provider_not_installed`.

## Permissions

Use `notification.publish`, `notification.schedule`, `notification.update`,
`notification.cancel`, `notification.action.register`,
`notification.action.receive`, `notification.subscription.manage`,
`notification.delivery.inspect`, and `notification.host.surface`.

## Commands And DTOs

Core DTOs are `NotificationMessage`, `NotificationTarget`,
`NotificationDeliveryChannel`, `NotificationSchedule`,
`NotificationActionDefinition`, `NotificationActionEvent`,
`NotificationDeliveryStatus`, `NotificationSubscriptionHandle`,
`NotificationDeliveryHandle`, and `NotificationProviderCapability`.

Commands are publish, schedule, update, cancel, list notifications, register and
unregister actions, acknowledge, dismiss, inspect delivery, register
subscription, and revoke subscription.

## Examples

Immediate publish:

```json
{"message": {"title_ref": "artifact:title", "body_ref": "artifact:body"}, "channel": "in_app", "client_request_id": "n1"}
```

Scheduled notification:

```json
{"publish": {"client_request_id": "n2"}, "schedule": {"deliver_at_epoch_ms": 1800000000000, "timezone_id": "UTC"}}
```

Action callback:

```json
{"category_id": "task", "action": {"action_id": "approve", "title_ref": "artifact:approve"}}
```

Push subscription registration:

```json
{"target": {"target_id": "user"}, "channel": "push", "secret_ref": "secret:push-subscription"}
```

Inspect delivery:

```json
{"delivery": {"delivery_id": "delivery", "channel": "push"}, "include_provider_evidence": false}
```

Unavailable provider:

```json
{"status": "unavailable", "error": {"code": "unavailable", "message": "notification provider is not installed"}}
```

## App-Facing Example Coverage

Generic examples cover immediate publish, scheduled notification, action
callback registration, push subscription registration, delivery inspection,
acknowledge/dismiss flow, cancellation, and unavailable provider handling. All
examples use synthetic message, channel, action, subscription, delivery, and
secret refs; they must not encode application-specific notification behavior or
provider routing.

## Provider Author Guidance

Provider classes are `host-notification`, `push-bridge`,
`subscription-bridge`, `mock`, and `unavailable`. Providers must expose delivery
state confidence, quota status, action callback metadata, subscription security,
health checks, snapshots, and conformance tests without leaking raw tokens,
push endpoints, keys, credentials, provider payloads, private data, or
unbounded content.
