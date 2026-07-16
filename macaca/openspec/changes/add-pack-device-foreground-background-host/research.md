# Device Foreground/Background Host Pack Research

## Purpose

This note records supplier/API comparison, Macaca provider-neutral mapping,
boundary decisions, existing platform inventory, and GitNexus memo evidence for
`pack.device.foreground_background_host.v1`. The pack must expose host
visibility, lifecycle state, foreground sessions, background leases, throttling,
policy inspection, revocation, event subscriptions, snapshots, and redaction
through typed service commands. It must not own workflow scheduling, process
supervision, notification content, or application-specific background business
logic.

## Source Baseline

- Android foreground services and foreground service types:
  <https://developer.android.com/develop/background-work/services/fgs> and
  <https://developer.android.com/about/versions/14/changes/fgs-types-required>
- Apple app lifecycle and background execution modes:
  <https://developer.apple.com/documentation/uikit/managing-your-app-s-life-cycle>
  and
  <https://developer.apple.com/documentation/xcode/configuring-background-execution-modes>
- Web Page Visibility API:
  <https://developer.mozilla.org/en-US/docs/Web/API/Page_Visibility_API>
- Windows app lifecycle and background tasks:
  <https://learn.microsoft.com/windows/apps/develop/app-lifecycle-and-system-services>
  and <https://learn.microsoft.com/windows/uwp/launch-resume/app-lifecycle>
- HarmonyOS ability lifecycle:
  <https://developer.huawei.com/consumer/en/doc/harmonyos-guides/application-context-stage>

## Supplier API Notes

- Android contributes foreground services, service types, user-visible
  notifications, permissions, runtime restrictions, and background execution
  limits. Macaca should normalize this as foreground sessions and background
  lease policy rather than Android-specific service-type branches.
- Apple contributes foreground/background transitions, suspension, background
  modes, and entitlement-controlled continued execution. Macaca should model
  entitlement and suspension as explicit host lifecycle diagnostics.
- Web Page Visibility contributes visible/hidden transitions and browser
  throttling constraints. Macaca should represent visibility and throttling as
  states, not as hidden errors.
- Windows contributes entered/leaving background events, background tasks,
  extended execution, and system-service constraints. Macaca should normalize
  lease expiry, revocation, and throttling.
- HarmonyOS contributes ability foreground/background transitions and
  continuous task mediation. Macaca should keep these as provider adapter
  details behind a common lifecycle state machine.

## Macaca-Owned Abstractions

`pack.device.foreground_background_host.v1` should define
`HostLifecycleState`, `ForegroundSession`, `BackgroundLease`,
`HostLifecycleEvent`, `HostLifecyclePolicy`,
`HostPresentationRequirement`, `HostThrottleState`,
`HostLifecycleSnapshot`, and `HostLifecycleError`.

The DTOs must carry visibility state, execution state, suspension state,
throttle state, foreground presentation requirements, background lease class,
purpose, max duration, dependent capability list, resource budget, entitlement
evidence, approval evidence, revocation state, policy hash, bounded provider
reason codes, and replay pointers. Raw provider lifecycle payloads, package
bytes, credentials, prompts, and unbounded lifecycle logs are rejected.

## Boundary Decisions

- Workflow task/schedule packs own planning and scheduled work; this pack only
  provides host lifecycle eligibility, foreground sessions, and background
  leases.
- Device camera, sensors, local files, and notifications consume lifecycle
  evidence but keep their own capability semantics and provider contracts.
- Application lifecycle services own application install/load/run state; this
  pack owns host visibility and execution-state mediation for declared
  capabilities.
- Shells render lifecycle indicators and diagnostics through SDK/service events
  but do not define lifecycle policy or background semantics.
- OS process supervision is outside this pack; background leases are
  capability-level eligibility contracts, not process-management guarantees.

## Existing Macaca Platform Inventory

- `crates/foundation/macaca-proto/src/domain_pack_contract/` provides reusable
  descriptor, lifecycle, availability, diagnostics, policy, SDK metadata, and
  unavailable diagnostic structures.
- `crates/facade/macaca-sdk/src/system_facade.rs` provides the Facade pattern
  for upper layers; lifecycle SDK helpers should only produce canonical traced
  service calls.
- `crates/runtime/macaca-host-composition/src/runtime_host.rs` and
  `crates/kernel/macaca-kernel/src/domain_pack_registration.rs` provide generic
  provider registration/composition mechanics.
- Kernel policy, audit, trace, and redaction modules provide reusable
  enforcement and observability substrate, but current evidence does not prove
  host-lifecycle-specific DTOs, descriptors, providers, SDK helpers, ABI, tests,
  or docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
