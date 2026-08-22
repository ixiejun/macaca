## 1. Research, Scope, And Governance

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design patterns, the umbrella industrial catalog proposal, and this child proposal before implementation.
- [x] 1.2 Record supplier/API comparison notes for Android Sensor Framework, Apple Core Motion, W3C Generic Sensor, W3C DeviceOrientation, Windows Sensors, and HarmonyOS Sensor Service.
- [x] 1.3 Confirm boundaries with device camera, device local-files, device notifications, foreground/background host capabilities, location packs, and application lifecycle services so sensors does not absorb unrelated capabilities.
- [x] 1.4 Record GitNexus CRITICAL/HIGH findings as memo-only evidence before implementation commits, per the current refactor instruction.

## 2. Contract, Descriptor, And DTO Schema

- [x] 2.1 Define provider-neutral commands for `sensors.list`, `sensors.inspect`, `sensors.read`, `sensors.open_stream`, `sensors.read_stream`, `sensors.close_stream`, `sensors.read_batch`, `sensors.inspect_calibration`, `sensors.acquire_lease`, `sensors.release_lease`, and `sensors.inspect_host`.
- [x] 2.2 Define `SensorDescriptor`, `SensorType`, `SensorReading`, `SensorVector`, `SensorCoordinateFrame`, `SensorAccuracy`, `SensorStreamLease`, `SensorBatch`, `SensorCalibration`, `SensorHostStatus`, and `SensorError`.
- [x] 2.3 Define typed success, partial, denied, unavailable, unsupported, disabled, permission-prompt-required, foreground-required, lease-expired, lease-revoked, sample-rate-too-high, stream-overflow, timeout, quota-exceeded, calibration-unavailable, provider-failure, and conflict results.
- [x] 2.4 Define descriptor metadata for pack id, family, lifecycle, command schemas, sensor descriptor hashes, permission scopes, privacy classes, sampling limits, batch limits, foreground requirements, policy template, resource budgets, SDK metadata, compatibility, diagnostics, and documentation URL.
- [x] 2.5 Add stable descriptor hashing, version compatibility checks, DTO snapshot fixtures, stream lease fixtures, redaction fixtures, and schema migration tests.

## 3. Permission, Policy, Resource, Entitlement, And Approval

- [x] 3.1 Implement declaration validation for `device.sensors.read`, `device.sensors.stream`, `device.sensors.calibration.read`, and `device.sensors.lease.manage`.
- [x] 3.2 Enforce sensor type, privacy class, frequency, sample count, stream duration, foreground/background, host permission, and retention policies before dispatch.
- [x] 3.3 Require stream commands to declare max duration, max sample count, frequency, delivery mode, cancellation behavior, and revocation behavior.
- [x] 3.4 Add resource reservation and quota checks for active leases, stream frequency, batch size, event buffer, CPU, memory, retained snapshots, and replay metadata.
- [x] 3.5 Add approval behavior for high-frequency motion streams, background access, host permission prompts, remote sensor forwarding, and sensitive environmental sensors.
- [x] 3.6 Add tests proving denied, unavailable, disabled, foreground-required, lease-revoked, and quota paths do not call concrete providers or leak resources.

## 4. Service Provider And Stream Lease Strategy

- [x] 4.1 Implement the device sensor service provider contract behind the service runtime; do not construct providers from kernel, SDK, shells, or generic application-framework code.
- [x] 4.2 Add provider descriptor support for host-native, browser, remote, plugin, mock, and unavailable provider classes.
- [x] 4.3 Add a stream lease state machine covering requested, active, draining, closed, expired, revoked, failed, and unavailable states.
- [x] 4.4 Add mock and unavailable providers for deterministic tests; external or host-specific adapters must remain optional providers or plugin/remote modules.
- [x] 4.5 Add provider conformance tests for list, inspect, one-shot read, stream open/read/close, batch read, calibration inspection, lease acquire/release, host inspection, revocation, redaction, and unsupported-command reporting.
- [x] 4.6 Add lifecycle, health, snapshot, shutdown, timeout, cancellation, backpressure, dropped-sample reporting, resource cleanup, and bounded output behavior.

## 5. SDK, Admission, Examples, And ABI

- [x] 5.1 Extend SDK discovery for `pack.device.sensors.v1` with command schemas, DTO schemas, permission scopes, examples, availability, host status, sensor descriptors, sampling limits, diagnostics, compatibility, and documentation URL.
- [x] 5.2 Extend application admission so required declarations block when unavailable/disabled and optional declarations degrade explicitly with effective capability mementos.
- [x] 5.3 Add SDK command helper builders that only produce canonical traced service calls and never construct providers or branch on host/platform names.
- [x] 5.4 Add WASM/application ABI exposure for sensor commands using provider-neutral DTO schemas and canonical service-call dispatch.
- [x] 5.5 Add generic examples for sensor listing, one-shot read, bounded stream, batch read, calibration inspection, lease revocation, and unavailable-provider diagnostics.

## 6. Trace, Audit, Replay, And Boundary Gates

- [x] 6.1 Emit sanitized `sensors.pack_declared`, `sensors.admission_validated`, `sensors.policy_decision`, `sensors.entitlement_checked`, `sensors.resource_reserved`, `sensors.command_requested`, `sensors.provider_selected`, `sensors.stream_opened`, `sensors.stream_chunk_delivered`, `sensors.stream_closed`, `sensors.lease_revoked`, `sensors.command_succeeded`, `sensors.command_failed`, `sensors.unavailable`, and `sensors.snapshot_recorded` events.
- [x] 6.2 Add replay tests proving every command is trace-addressable through the canonical service path after refresh/restart without raw sample vectors.
- [x] 6.3 Add dependency-boundary gates proving microkernel, SDK, shells, and generic application framework do not import concrete sensor providers or host APIs.
- [ ] 6.4 Add no-direct-provider-call gates proving all sensor commands enter through descriptor-owned service registrations and typed service runtime dispatch.
- [x] 6.5 Add redaction tests for raw sample vectors, stable hardware identifiers, host API payloads, calibration details, stream chunks, lease ids, snapshots, and diagnostics.
- [ ] 6.6 Run `openspec validate add-pack-device-sensors --strict`, DTO compatibility tests, stream lifecycle tests, permission denial tests, revocation tests, boundary gates, file-size gates, and audit replay checks before marking implementation tasks complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/device/sensors.md` with purpose, manifest declarations, required/optional behavior, scopes, command DTOs, result DTOs, sensor types, units, coordinate frames, accuracy, calibration, sampling, batching, stream leases, revocation, unavailable diagnostics, and trace/audit behavior.
- [x] 7.2 Add provider author documentation covering descriptor fields, host adapter responsibilities, stream lease state machine, conformance tests, unsupported behavior, redaction rules, health/snapshot behavior, and replacement strategy.
- [x] 7.3 Add minimal app-facing examples for list sensors, one-shot read, bounded stream, batch read, calibration inspection, lease revocation, and unavailable-provider diagnostics using generic synthetic data.
- [x] 7.4 Cross-link the guide from SDK discovery metadata and the industrial pack catalog index before marking `add-pack-device-sensors` complete.
