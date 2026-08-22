## 1. Research, Scope, And Governance

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design patterns, the umbrella industrial catalog proposal, and this child proposal before implementation.
- [x] 1.2 Record supplier/API comparison notes for Android foreground services/service types, Apple app lifecycle/background modes, Web Page Visibility, Windows lifecycle/background tasks, and HarmonyOS ability lifecycle.
- [x] 1.3 Confirm boundaries with workflow schedule/task, device camera/sensors/local-files/notifications, application lifecycle, shell rendering, and OS process supervision.
- [x] 1.4 Record GitNexus CRITICAL/HIGH findings as memo-only evidence before implementation commits, per the current refactor instruction.

## 2. Contract, Descriptor, And DTO Schema

- [x] 2.1 Define provider-neutral commands for `host_lifecycle.inspect_state`, `host_lifecycle.subscribe_events`, `host_lifecycle.open_foreground_session`, `host_lifecycle.close_foreground_session`, `host_lifecycle.request_background_lease`, `host_lifecycle.release_background_lease`, `host_lifecycle.inspect_policy`, `host_lifecycle.revoke`, and `host_lifecycle.inspect_host`.
- [x] 2.2 Define `HostLifecycleState`, `ForegroundSession`, `BackgroundLease`, `HostLifecycleEvent`, `HostLifecyclePolicy`, `HostPresentationRequirement`, `HostThrottleState`, `HostLifecycleSnapshot`, and `HostLifecycleError`.
- [x] 2.3 Define typed success, partial, denied, unavailable, unsupported, foreground-required, background-denied, entitlement-required, presentation-required, lease-expired, lease-revoked, throttled, suspended, quota-exceeded, provider-failure, and conflict results.
- [x] 2.4 Define descriptor metadata for pack id, family, lifecycle, command schemas, supported states, foreground presentations, background lease classes, throttling metadata, dependent capability rules, permission scopes, policy template, resource budgets, SDK metadata, compatibility, diagnostics, and documentation URL.
- [x] 2.5 Add stable descriptor hashing, version compatibility checks, DTO snapshot fixtures, lifecycle transition fixtures, lease/session fixtures, revocation fixtures, and schema migration tests.

## 3. Permission, Policy, Resource, Entitlement, And Approval

- [x] 3.1 Implement declaration validation for `device.host_lifecycle.read`, `device.host_lifecycle.events`, `device.host_lifecycle.foreground`, `device.host_lifecycle.background`, and `device.host_lifecycle.revoke`.
- [ ] 3.2 Enforce foreground presentation, background lease class, entitlement, max duration, dependent capability, throttling, suspension, resource budget, and revocation policies before dispatch.
- [x] 3.3 Require foreground sessions and background leases to declare purpose, max duration, resource budget, dependent capabilities, and cleanup behavior.
- [ ] 3.4 Add resource reservation and quota checks for active sessions, active leases, background duration, event subscription count, CPU/network/timer budget, retained snapshots, and replay metadata.
- [ ] 3.5 Add approval behavior for long-running foreground sessions, background execution, sensitive dependent capabilities, remote-host lifecycle delegation, and throttling override.
- [ ] 3.6 Add tests proving denied, unavailable, background-denied, presentation-required, lease-expired, lease-revoked, throttled, suspended, and quota paths do not call concrete providers or leak resources.

## 4. Service Provider And Lifecycle State Strategy

- [x] 4.1 Implement the foreground/background host lifecycle service provider contract behind the service runtime; do not construct providers from kernel, SDK, shells, or generic application-framework code.
- [x] 4.2 Add provider descriptor support for host-native, browser, remote-host, plugin, mock, and unavailable provider classes.
- [x] 4.3 Add foreground session and background lease state machines covering requested, active, throttled, suspended, closing, closed, expired, revoked, failed, and unavailable states.
- [x] 4.4 Add mock and unavailable providers for deterministic tests; host-specific adapters must remain optional providers or plugin/remote modules.
- [x] 4.5 Add provider conformance tests for state inspection, event subscription, foreground session open/close, background lease request/release, policy inspection, revocation, throttling, suspension, redaction, and unsupported-command reporting.
- [ ] 4.6 Add lifecycle, health, snapshot, shutdown, timeout, cancellation, state transition, resource cleanup, and bounded output behavior.

## 5. SDK, Admission, Examples, And ABI

- [x] 5.1 Extend SDK discovery for `pack.device.foreground_background_host.v1` with command schemas, DTO schemas, permission scopes, examples, availability, host state, supported foreground presentations, supported background lease classes, throttling metadata, diagnostics, compatibility, and documentation URL.
- [ ] 5.2 Extend application admission so required declarations block when unavailable/disabled and optional declarations degrade explicitly with effective capability mementos.
- [x] 5.3 Add SDK command helper builders that only produce canonical traced service calls and never construct providers or branch on host/platform/service-type names.
- [x] 5.4 Add WASM/application ABI exposure for host lifecycle commands using provider-neutral DTO schemas and canonical service-call dispatch.
- [x] 5.5 Add generic examples for state inspection, event subscription, foreground session, background lease, policy inspection, revocation, throttling, and unavailable-provider diagnostics.

## 6. Trace, Audit, Replay, And Boundary Gates

- [ ] 6.1 Emit sanitized `host_lifecycle.pack_declared`, `host_lifecycle.admission_validated`, `host_lifecycle.policy_decision`, `host_lifecycle.state_changed`, `host_lifecycle.foreground_session_opened`, `host_lifecycle.foreground_session_closed`, `host_lifecycle.background_lease_requested`, `host_lifecycle.background_lease_granted`, `host_lifecycle.background_lease_released`, `host_lifecycle.session_or_lease_revoked`, `host_lifecycle.throttle_changed`, `host_lifecycle.command_failed`, `host_lifecycle.unavailable`, and `host_lifecycle.snapshot_recorded` events.
- [ ] 6.2 Add replay tests proving every command and lifecycle event is trace-addressable through the canonical service path after refresh/restart.
- [x] 6.3 Add dependency-boundary gates proving microkernel, SDK, shells, and generic application framework do not import concrete lifecycle providers or host lifecycle APIs.
- [x] 6.4 Add no-direct-provider-call gates proving all host lifecycle commands enter through descriptor-owned service registrations and typed service runtime dispatch.
- [x] 6.5 Add redaction tests for provider payloads, host identifiers, presentation metadata, lifecycle logs, credentials, session/lease ids, snapshots, and diagnostics.
- [ ] 6.6 Run `openspec validate add-pack-device-foreground-background-host --strict`, DTO compatibility tests, lifecycle transition tests, lease/session tests, revocation tests, boundary gates, file-size gates, and audit replay checks before marking implementation tasks complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/device/foreground-background-host.md` with purpose, manifest declarations, required/optional behavior, scopes, command DTOs, result DTOs, lifecycle states, foreground sessions, background leases, throttling/suspension, dependent capability integration, revocation, unavailable diagnostics, and trace/audit behavior.
- [x] 7.2 Add provider author documentation covering descriptor fields, host adapter responsibilities, lifecycle/session/lease state machines, conformance tests, unsupported behavior, redaction rules, health/snapshot behavior, and replacement strategy.
- [x] 7.3 Add minimal app-facing examples for inspect state, subscribe events, open foreground session, request background lease, inspect policy, revoke, and unavailable-provider diagnostics using generic synthetic data.
- [x] 7.4 Cross-link the guide from SDK discovery metadata and the industrial pack catalog index before marking `add-pack-device-foreground-background-host` complete.
