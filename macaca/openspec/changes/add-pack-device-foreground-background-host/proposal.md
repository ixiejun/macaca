# Change: Add Industrial Device Foreground/Background Host Pack

## Why

Macaca applications need `pack.device.foreground_background_host.v1` to reason about host foreground visibility, background eligibility, lifecycle transitions, foreground service/session presentation, background task leases, suspension, revocation, and policy diagnostics. Device packs such as camera, sensors, local files, and notifications depend on this capability, but the rules must not be hardcoded into each pack or shell.

Foreground/background behavior is host-specific and security-sensitive. Android foreground services require user-visible notification and service types; Apple background modes are entitlement-bound; browsers expose visibility and throttling constraints; Windows has lifecycle and background task contracts. Macaca needs a provider-neutral service pack that makes those states declarative, auditable, and enforceable.

## Supplier/API Baseline

- Android Foreground Services and foreground service types: visible ongoing work, runtime checks, declared service types, permissions, restrictions, and notification requirements. Official docs: https://developer.android.com/develop/background-work/services/fgs and https://developer.android.com/about/versions/14/changes/fgs-types-required
- Apple app lifecycle and background execution modes: foreground/background transitions, suspension, background modes, and entitlement-controlled continued execution. Official docs: https://developer.apple.com/documentation/uikit/managing-your-app-s-life-cycle and https://developer.apple.com/documentation/xcode/configuring-background-execution-modes
- Web Page Visibility / lifecycle patterns: document visibility, hidden/visible transitions, throttling, and browser-mediated background constraints. Official docs: https://developer.mozilla.org/en-US/docs/Web/API/Page_Visibility_API
- Windows app lifecycle and background tasks: entered/leaving background events, background tasks, extended execution, and system service constraints. Official docs: https://learn.microsoft.com/windows/apps/develop/app-lifecycle-and-system-services and https://learn.microsoft.com/windows/uwp/launch-resume/app-lifecycle
- HarmonyOS background task and ability lifecycle model: foreground/background ability transitions and permission-mediated continuous tasks. Official docs: https://developer.huawei.com/consumer/en/doc/harmonyos-guides/application-context-stage

## Macaca Provider-Neutral Mapping

Macaca SHALL expose host lifecycle as a serviceized pack:

- Visibility and lifecycle inspection become `host_lifecycle.inspect_state`.
- Transition events become `host_lifecycle.subscribe_events`.
- Foreground user-visible work becomes `host_lifecycle.open_foreground_session`.
- Background work becomes `host_lifecycle.request_background_lease`.
- Policy/resource state becomes `host_lifecycle.inspect_policy`.
- Lease/session cleanup becomes `host_lifecycle.close_foreground_session`, `host_lifecycle.release_background_lease`, and `host_lifecycle.revoke`.

## What Changes

- Add `pack.device.foreground_background_host.v1` as a service-backed industrial pack under the device family.
- Define command DTOs for host lifecycle state, visibility, foreground sessions, background leases, policy inspection, event subscription, revocation, and host status.
- Define DTOs for `HostLifecycleState`, `ForegroundSession`, `BackgroundLease`, `HostLifecycleEvent`, `HostLifecyclePolicy`, `HostPresentationRequirement`, `HostThrottleState`, `HostLifecycleSnapshot`, and structured errors.
- Define permission scopes, approval/entitlement rules, resource budgets, lifecycle transitions, revocation, throttling, and unavailable diagnostics.
- Require detailed developer documentation under `docs/developer-packs/device/foreground-background-host.md`.

## Impact

- Affected specs: `pack-device-foreground-background-host`, `developer-pack-industrial-capability-catalog`, `sdk-system-facade`, `service-runtime`, `unified-execution-path`.
- Later affected code: protocol DTOs, descriptor/admission validators, SDK pack client, host lifecycle service provider contract, host/browser lifecycle adapters, mock/unavailable providers, trace/audit schemas, and boundary gates.
- Validation: `openspec validate add-pack-device-foreground-background-host --strict`, lifecycle transition tests, lease/session tests, revocation tests, canonical path tests, no-direct-provider-call gates, and docs coverage checks.

## Non-Goals

- This pack does not own task scheduling, notification content, camera/sensor/file semantics, OS process management, workflow execution, or application-specific background business logic.
- This pack does not hardcode Android, Apple, Windows, browser, HarmonyOS, service type names, background mode names, provider names, or application workflows into OS-layer routing.
- This pack does not bypass host restrictions, fake background execution, or expose raw provider payloads, secrets, prompts, package bytes, credentials, or unbounded lifecycle logs in observability.
