# Device Foreground/Background Host Pack

`pack.device.foreground_background_host.v1` provides provider-neutral host
visibility and lifecycle inspection, lifecycle event subscription, foreground
session management, background lease management, policy inspection, revocation,
throttling/suspension diagnostics, and host status.

The pack does not keep applications alive by itself and does not own workflow
scheduling, task execution, process supervision, camera, sensors, local files,
notifications, or application-specific background logic.

## Manifest Declaration

```toml
[service_contract]
optional_packs = ["pack.device.foreground_background_host.v1"]
```

Unavailable optional declarations report
`device_foreground_background_host_provider_not_installed`.

## Commands

- `host_lifecycle.inspect_state`: returns `HostLifecycleState`.
- `host_lifecycle.subscribe_events`: subscribes to redacted
  `HostLifecycleEvent` records.
- `host_lifecycle.open_foreground_session` and `close_foreground_session`:
  manage `ForegroundSession`.
- `host_lifecycle.request_background_lease` and `release_background_lease`:
  manage `BackgroundLease`.
- `host_lifecycle.inspect_policy`: returns `HostLifecyclePolicy`.
- `host_lifecycle.revoke`: revokes sessions or leases.
- `host_lifecycle.inspect_host`: returns host lifecycle status.

## DTOs And Results

Core DTOs include `HostLifecycleState`, `ForegroundSession`,
`BackgroundLease`, `HostLifecycleEvent`, `HostLifecyclePolicy`,
`HostPresentationRequirement`, `HostThrottleState`, `HostLifecycleSnapshot`,
and `HostLifecycleError`. Result statuses include success, partial, denied,
unavailable, unsupported, foreground-required, background-denied,
entitlement-required, presentation-required, lease-expired, lease-revoked,
throttled, suspended, quota-exceeded, provider-failure, and conflict.

## Provider Mapping

Android foreground services/service types, Apple app lifecycle and background
modes, Web Page Visibility, Windows lifecycle/background tasks, and HarmonyOS
ability lifecycle map into foreground sessions, background leases, presentation
requirements, dependent capabilities, throttling, suspension, and revocation.
Host OS names, service type names, background mode names, provider payloads,
host identifiers, session/lease ids, and unbounded lifecycle logs are not OS
routing semantics.

## App-Facing Examples

Applications call the pack through typed host-lifecycle commands and receive
redacted state, event, session, and lease references. Each example assumes the
app already declared `pack.device.foreground_background_host.v1` and every
command carries trace, session, tenant, and capability context through the SDK
facade.

- Inspect current visibility and lifecycle state with
  `host_lifecycle.inspect_state`.
- Subscribe to bounded lifecycle events with `host_lifecycle.subscribe_events`
  and store only event references plus redacted state transitions.
- Open a foreground session with `host_lifecycle.open_foreground_session` when a
  dependent pack requires foreground presence, then close it explicitly.
- Request a background lease with `host_lifecycle.request_background_lease` and
  honor returned throttling, suspension, and expiration metadata.
- Inspect policy with `host_lifecycle.inspect_policy` before requesting work
  that could continue after the UI is hidden.
- Revoke sessions or leases with `host_lifecycle.revoke` when the user or
  policy removes permission.
- Display unavailable diagnostics from
  `device_foreground_background_host_provider_not_installed` without keeping an
  application alive through local workarounds.

## Conformance

Provider authors must cover descriptor fields, host adapter responsibilities,
lifecycle/session/lease state machines, throttling and suspension, unsupported
behavior, redaction, health/snapshot behavior, replacement strategy,
unavailable behavior, and replay-safe bounded lifecycle evidence.
