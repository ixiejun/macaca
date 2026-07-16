# Device Foreground/Background Host Pack Design

## Context

`pack.device.foreground_background_host.v1` provides the shared host lifecycle contract for device capabilities. It does not keep applications alive by itself; it describes and mediates host-visible foreground sessions, background leases, lifecycle transitions, throttling, suspension, and revocation through the service runtime.

This pack lets other capabilities ask "is this operation allowed while foreground/background?" through provider-neutral policy evidence instead of embedding platform-specific lifecycle checks.

## Supplier Capability Matrix

| Platform/API | Borrowed capability | Macaca mapping |
| --- | --- | --- |
| Android Foreground Services | user-visible ongoing work, service types, permissions, notification requirement, runtime checks | foreground sessions, presentation requirement, type declarations, policy checks |
| Apple lifecycle/background modes | foreground/background transitions, suspension, entitlement-bound background execution | lifecycle events, background leases, entitlement diagnostics |
| Web Page Visibility | visible/hidden state, throttling, browser constraints | visibility state, throttle state, event subscription |
| Windows lifecycle/background tasks | entered/leaving background, background tasks, extended execution | lifecycle transitions, lease classes, expiration/revocation |
| HarmonyOS ability lifecycle | foreground/background ability states and continuous task mediation | host lifecycle state, provider adapter, policy diagnostics |

## Goals

- Provide lifecycle/visibility inspection, event subscription, foreground session open/close, background lease request/release, policy inspection, revocation, and host status.
- Normalize foreground-visible, background-visible, hidden, suspended, terminated, throttled, locked, and unavailable states.
- Enforce permission, policy, entitlement, approval, resource budgets, presentation requirements, throttling, expiry, and revocation.
- Support host-native, browser, remote-host, plugin, mock, and unavailable providers through descriptors.
- Provide detailed developer documentation and provider conformance guidance.

## Non-Goals

- Do not own workflow scheduling, task execution, notification content, camera/sensors/local-files operations, process supervision, or application-specific background logic.
- Do not bypass host restrictions or guarantee background execution when host policy denies it.
- Do not branch on host OS, service type name, background mode name, provider name, or application workflow in OS-layer code.

## Ownership And Boundaries

- Pack id: `pack.device.foreground_background_host.v1`.
- Capability family: `device`.
- Backing service: foreground/background host lifecycle service.
- SDK surface: `sdk.packs.device.foreground_background_host`.
- Command namespace: `host_lifecycle.*`.
- Application framework owns manifest declaration and app-scoped permission projection.
- Service runtime owns typed dispatch, decorators, foreground/background lease state machines, health, snapshots, and unavailable behavior.
- Runtime host owns concrete platform/browser/provider adapters through approved composition roots.
- Shells may render foreground/background diagnostics but must not define lifecycle semantics.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `host_lifecycle.inspect_state` | Inspect current host visibility/lifecycle state | Returns foreground/background/hidden/suspended/throttled state, reason, provider class, and policy metadata |
| `host_lifecycle.subscribe_events` | Subscribe to lifecycle transition events | Emits canonical events for foreground/background/hidden/suspended/resumed/revoked/throttled transitions |
| `host_lifecycle.open_foreground_session` | Start a user-visible foreground session | Requires purpose, presentation requirement, capability type, max duration, and policy/resource reservation |
| `host_lifecycle.close_foreground_session` | Close foreground session | Idempotently releases presentation and resources |
| `host_lifecycle.request_background_lease` | Request bounded background execution eligibility | Requires lease class, purpose, trigger, max duration, resource budget, entitlement, and approval when configured |
| `host_lifecycle.release_background_lease` | Release background lease | Idempotently releases lease and resources |
| `host_lifecycle.inspect_policy` | Inspect effective lifecycle policy | Returns allowed modes, background classes, throttling, required presentations, and dependent capability constraints |
| `host_lifecycle.revoke` | Revoke active sessions/leases by scope | Closes/revokes sessions and emits audit evidence |
| `host_lifecycle.inspect_host` | Inspect host provider health/status | Returns host support, disabled reason, active sessions/leases, and diagnostics |

## DTO Model

- `HostLifecycleState`: visibility, execution state, suspension state, throttle state, lock/screen state when available, reason, timestamp, and provider class.
- `ForegroundSession`: session id, purpose, capability type, presentation requirement, max duration, state, approval id, resource reservation, dependent capabilities, and revocation state.
- `BackgroundLease`: lease id, lease class, purpose, trigger, max duration, state, entitlement, approval id, resource budget, expiration, and revocation state.
- `HostLifecycleEvent`: event id, transition type, previous/current state, affected sessions/leases, reason code, timestamp, and trace context.
- `HostLifecyclePolicy`: allowed foreground classes, allowed background lease classes, throttling rules, max durations, required presentations, dependent capability rules, and denial reasons.
- `HostPresentationRequirement`: notification/status/menu/shell indicator requirement, label, icon reference, update policy, dismissibility, and privacy class.
- `HostThrottleState`: CPU/network/timer/background limitations, grace period, wake eligibility, and provider diagnostics.
- `HostLifecycleSnapshot`: active session summaries, active lease summaries, state, policy hash, provider health, and replay pointers.
- `HostLifecycleError`: denied, unavailable, unsupported, foreground required, background denied, entitlement required, presentation required, lease expired, lease revoked, throttled, suspended, quota exceeded, provider failure, or conflict.

## Permission, Policy, Resource, Entitlement, And Approval

Initial scopes:

- `device.host_lifecycle.read`: state, policy, host inspection.
- `device.host_lifecycle.events`: lifecycle event subscription.
- `device.host_lifecycle.foreground`: foreground session open/close.
- `device.host_lifecycle.background`: background lease request/release.
- `device.host_lifecycle.revoke`: scoped lifecycle revocation.

Policy requirements:

- Foreground sessions require user-visible presentation when host policy requires it.
- Background leases are denied by default unless entitlement and policy allow the requested lease class.
- Leases/sessions require max duration, purpose, dependent capability list, resource budget, and revocation behavior.
- Host throttling/suspension is an explicit state, not an error to hide.
- Other device packs must depend on this pack through policy evidence rather than duplicating host lifecycle logic.

## Service Runtime And Provider Strategy

Provider Strategy categories:

- Host-native provider: mobile/desktop lifecycle APIs.
- Browser provider: page visibility, lifecycle, and throttling APIs.
- Remote-host provider: delegated host lifecycle state from trusted remote host.
- Plugin provider: specialized shell/embedded host lifecycle adapter.
- Mock provider: deterministic lifecycle transitions for tests/docs.
- Unavailable provider: explicit unavailable diagnostics.

Providers declare supported states, event delivery, foreground presentation types, background lease classes, max durations, throttling metadata, dependent capability rules, and health.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, lifecycle, command schemas, DTO schemas, permission scopes, effective availability, host lifecycle state, supported foreground presentations, supported background lease classes, policy templates, examples, diagnostics, compatibility, and documentation links.

The implementation SHALL create `docs/developer-packs/device/foreground-background-host.md` with manifest declarations, scopes, lifecycle states, foreground session model, background lease model, throttling/suspension behavior, dependent capability integration, revocation, unavailable diagnostics, trace/audit reference, and provider conformance checklist.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `host_lifecycle.pack_declared`
- `host_lifecycle.admission_validated`
- `host_lifecycle.policy_decision`
- `host_lifecycle.state_changed`
- `host_lifecycle.foreground_session_opened`
- `host_lifecycle.foreground_session_closed`
- `host_lifecycle.background_lease_requested`
- `host_lifecycle.background_lease_granted`
- `host_lifecycle.background_lease_released`
- `host_lifecycle.session_or_lease_revoked`
- `host_lifecycle.throttle_changed`
- `host_lifecycle.command_failed`
- `host_lifecycle.unavailable`
- `host_lifecycle.snapshot_recorded`

Events include pack id, command name, service id, descriptor version, trace id, application/session/task/tenant ids when present, provider class, lifecycle state, session/lease id hash, lease class, presentation class, policy decision, duration class, resource counters, and reason codes. Events exclude raw provider payloads, secrets, credentials, prompts, package bytes, and unbounded lifecycle logs.

Snapshots include provider health, current state, policy hash, active session summaries, active lease summaries, throttling state, unavailable diagnostics, and sanitized replay pointers.

## Design Patterns

- **Facade**: SDK exposes lifecycle discovery and command builders while `SystemFacade` carries canonical service calls.
- **Command**: every operation is a typed command/result DTO.
- **Adapter**: host, browser, remote, plugin, mock, and unavailable providers map into Macaca DTOs.
- **Strategy**: provider selection, foreground presentation, background lease class, throttling, and unavailable behavior are descriptor-driven.
- **Decorator**: trace, policy, resource, entitlement, approval, metering, and redaction wrap every call.
- **State**: foreground sessions, background leases, and host lifecycle transitions are explicit state machines.
- **Specification**: admission validates scopes, lease class, presentation, max duration, dependent capabilities, and budgets.
- **Observer**: trace, audit, lifecycle, health, and service events are subscribable.
- **Memento**: snapshots record state/session/lease summaries for replay.
- **Abstract Factory**: providers are created only in approved composition roots.

## Risks And Mitigations

- Risk: pack promises background execution the host cannot provide. Mitigation: explicit unavailable/denied/throttled states and no fake success.
- Risk: device packs duplicate lifecycle logic. Mitigation: dependent capability policy evidence and boundary gates.
- Risk: foreground sessions hide user-visible work. Mitigation: presentation requirements and audit evidence.
- Risk: background leases leak resources. Mitigation: max duration, revocation, resource budgets, and shutdown cleanup.
- Risk: SDK helpers bypass lifecycle service. Mitigation: helpers only build canonical service commands and no-direct-provider-call gates enforce dispatch.
