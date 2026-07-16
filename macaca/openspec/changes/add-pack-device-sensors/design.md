# Device Sensors Pack Design

## Context

`pack.device.sensors.v1` exposes host/device sensors to Macaca applications through a provider-neutral service contract. Sensor APIs vary widely across mobile, desktop, browser, embedded, and remote-host environments. The pack must normalize capability discovery, sampled reads, streams, batching, accuracy, calibration, and coordinate frames while preserving strict privacy and resource controls.

This pack is a device service capability. The microkernel owns identity, policy facade, service-call evidence, trace, and audit primitives. Sensor access, host capability mediation, stream leases, calibration metadata, and provider replacement belong to the device sensor service.

## Supplier Capability Matrix

| Platform/API | Borrowed capability | Macaca mapping |
| --- | --- | --- |
| Android Sensor Framework | sensor types, sampling periods, batching, wake-up sensors, accuracy, dynamic sensors | descriptors, `sampling_policy`, `batching`, `wake_behavior`, accuracy metadata |
| Apple Core Motion | accelerometer/gyro/magnetometer/device motion, update intervals, reference frames | `SensorCoordinateFrame`, stream frequency, vector readings, attitude/orientation metadata |
| W3C Generic Sensor | activation lifecycle, frequency hints, readings, permission policy, error events | stream leases, frequency budgets, permission/policy gates, structured errors |
| W3C DeviceOrientation | orientation/motion privacy, user permission, coordinate conventions | orientation sensor DTOs, redaction, approval, foreground restrictions |
| Windows Sensors | device capability declarations, categories, reports, privacy settings | manifest declarations, host status, unavailable/disabled diagnostics |
| HarmonyOS Sensor Service | sensor subscriptions, sampling, callback model, permissions | service stream subscription, bounded batch/stream events, scope validation |

## Goals

- Provide industrial sensor discovery, inspection, one-shot reads, streams, batch reads, calibration inspection, leases, and host status inspection.
- Normalize sensor type, units, axis ordering, coordinate frames, timestamp clock, accuracy, calibration, sampling frequency, batching, wake behavior, and foreground/background constraints.
- Enforce permission, policy, approval, resource quotas, revocation, and stream lifetime before and during sensor use.
- Support host-native, browser, plugin, remote, mock, and unavailable providers through descriptors and provider-neutral DTOs.
- Provide detailed developer documentation and provider conformance guidance.

## Non-Goals

- Do not own camera, microphone, local files, notifications, foreground/background lifecycle, geolocation, place search, or application-specific activity recognition.
- Do not expose raw provider payloads, raw high-frequency readings, stable hardware identifiers, secrets, credentials, or unbounded data in observability surfaces.
- Do not branch on host OS, browser, sensor model, provider name, or application workflow in OS-layer code.

## Ownership And Boundaries

- Pack id: `pack.device.sensors.v1`.
- Capability family: `device`.
- Backing service: device sensor service.
- SDK surface: `sdk.packs.device.sensors`.
- Command namespace: `sensors.*`.
- Application framework owns manifest declaration and app-scoped permission projection.
- Service runtime owns typed dispatch, decorators, stream leases, provider lifecycle, health, snapshots, and unavailable behavior.
- Runtime host owns concrete host/provider adapters through approved composition roots.
- Shells render diagnostics and call SDK/facade clients only.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `sensors.list` | List available sensor descriptors | Returns normalized sensor descriptors, availability, permission requirements, sampling limits, and host status |
| `sensors.inspect` | Inspect one sensor's capability details | Returns units, axes, coordinate frame, frequency range, batching, wake behavior, accuracy classes, and privacy class |
| `sensors.read` | Perform a one-shot sampled read | Requires permission, foreground/background policy, timeout, freshness, and redaction |
| `sensors.open_stream` | Open a bounded sensor stream lease | Requires sampling frequency, duration/budget, delivery mode, policy, and resource reservation |
| `sensors.read_stream` | Read events from an active stream lease | Returns bounded chunks with sequence numbers, timestamps, accuracy, and dropped-sample counters |
| `sensors.close_stream` | Close a stream lease | Releases resources and emits audit evidence |
| `sensors.read_batch` | Read a bounded batch over a short interval | Enforces max duration/sample count and returns batch metadata |
| `sensors.inspect_calibration` | Inspect calibration/accuracy/provenance metadata | Returns calibration status, accuracy class, last calibration time if available, and provider limitations |
| `sensors.acquire_lease` | Reserve sensor resources for future read/stream | Creates revocable lease with duration, frequency, sensor types, and policy metadata |
| `sensors.release_lease` | Release a sensor lease | Idempotently closes streams and releases resources |
| `sensors.inspect_host` | Inspect host sensor service status | Returns disabled/unavailable/degraded state, permission state, foreground requirements, and provider class |

## DTO Model

- `SensorDescriptor`: stable sensor id, sensor type, display label, vendor class, privacy class, axes, units, min/max frequency, batching support, wake behavior, foreground requirement, permission scopes, and availability.
- `SensorType`: accelerometer, gyroscope, magnetometer, barometer, ambient light, proximity, orientation, gravity, linear acceleration, rotation vector, step counter, device motion, custom typed extension.
- `SensorReading`: descriptor id, timestamp, timestamp clock, value, unit, coordinate frame, accuracy, sequence number, sample interval, redaction state, and provenance.
- `SensorVector`: axis values with axis labels and coordinate-frame reference.
- `SensorCoordinateFrame`: device frame, screen-adjusted frame, world frame, magnetic north, true north, provider-defined extension, and transformation metadata.
- `SensorAccuracy`: high, medium, low, uncalibrated, unavailable, degraded, or provider-specific mapped class.
- `SensorStreamLease`: lease id, sensor ids, frequency, delivery mode, max duration, max samples, foreground/background mode, revocation state, and resource reservation.
- `SensorBatch`: readings, dropped-sample count, start/end timestamps, sample count, clock drift warning, and truncation reason.
- `SensorCalibration`: calibration state, accuracy, calibration age, calibration source, limitations, and redacted provider notes.
- `SensorHostStatus`: provider class, permission state, disabled reason, health, supported commands, active leases, resource pressure, and diagnostics.
- `SensorError`: denied, unavailable, unsupported, disabled, permission prompt required, foreground required, lease expired, lease revoked, sample rate too high, stream overflow, timeout, quota exceeded, calibration unavailable, provider failure, or conflict.

## Permission, Policy, Resource, Entitlement, And Approval

Initial scopes:

- `device.sensors.read`: list, inspect, one-shot read, and host status.
- `device.sensors.stream`: open/read/close streams and read batches.
- `device.sensors.calibration.read`: inspect calibration and accuracy provenance.
- `device.sensors.lease.manage`: acquire and release resource leases.

Policy requirements:

- High-frequency motion streams are privacy-sensitive and require explicit sampling budgets.
- Background sensor access is denied by default unless the host foreground/background pack and policy explicitly allow it.
- Streams require max duration, max sample count, frequency, delivery mode, and revocation behavior.
- Host permission prompt state must return structured diagnostics rather than fake availability.
- Raw high-frequency samples are not written to trace/audit; traces store counters, hashes, privacy class, and bounded diagnostics.
- Revocation closes active streams and releases leases promptly.

## Service Runtime And Provider Strategy

Provider Strategy categories:

- Host-native provider: mobile/desktop OS sensor APIs.
- Browser provider: W3C Generic Sensor or DeviceOrientation-style APIs.
- Remote provider: sensors forwarded from a trusted remote host with explicit transport capability.
- Plugin provider: specialized device/robot/IoT sensor adapters.
- Mock provider: deterministic synthetic streams for tests and docs.
- Unavailable provider: explicit unavailable diagnostics when host support is absent.

Providers declare sensor descriptors, supported commands, frequency limits, batch limits, privacy classes, permission states, foreground requirements, and health. Provider construction is allowed only in approved runtime composition roots.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, lifecycle, command schemas, DTO schemas, permission scopes, effective availability, host status, sensor descriptors, sampling limits, policy templates, examples, diagnostics, compatibility, and documentation links.

The implementation SHALL create `docs/developer-packs/device/sensors.md` with:

- Manifest declaration examples for required and optional sensor use.
- Permission scope and host permission prompt behavior.
- Command-by-command DTO reference.
- Sensor type, unit, coordinate-frame, accuracy, calibration, sampling, batching, and stream lease guidance.
- Foreground/background and revocation rules.
- Error taxonomy and unavailable-provider troubleshooting.
- Trace/audit event reference and replay workflow.
- Provider author conformance checklist.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `sensors.pack_declared`
- `sensors.admission_validated`
- `sensors.policy_decision`
- `sensors.entitlement_checked`
- `sensors.resource_reserved`
- `sensors.command_requested`
- `sensors.provider_selected`
- `sensors.stream_opened`
- `sensors.stream_chunk_delivered`
- `sensors.stream_closed`
- `sensors.lease_revoked`
- `sensors.command_succeeded`
- `sensors.command_failed`
- `sensors.unavailable`
- `sensors.snapshot_recorded`

Events include pack id, command name, service id, descriptor version, trace id, application/session/task/tenant ids when present, provider class, sensor type, privacy class, frequency class, sample count, dropped count, lease id hash, policy decision, latency, and resource counters. Events exclude raw sample vectors, stable hardware identifiers, raw provider payloads, secrets, and unbounded data.

Snapshots include provider health, host status, supported command matrix, sensor descriptor hashes, active lease summaries, resource pressure, policy template hash, unavailable diagnostics, and sanitized replay pointers.

## Design Patterns

- **Facade**: SDK exposes discovery and command builders while `SystemFacade` carries canonical service calls.
- **Command**: every operation is a typed command/result DTO.
- **Adapter**: host, browser, remote, plugin, mock, and unavailable providers map into Macaca DTOs.
- **Strategy**: provider selection, sampling limits, delivery mode, and unavailable behavior are descriptor-driven.
- **Decorator**: trace, policy, resource, entitlement, approval, metering, and redaction wrap every call.
- **State**: stream leases and revocation are explicit state machines.
- **Specification**: admission validates scopes, sensor type, sample rate, duration, foreground mode, and resource budgets.
- **Observer**: trace, audit, health, stream, and service events are subscribable.
- **Memento**: snapshots record descriptors and leases for replay without raw samples.
- **Abstract Factory**: providers are created only in approved composition roots.

## Risks And Mitigations

- Risk: high-frequency streams become privacy leaks. Mitigation: budgeted streams, redacted observability, foreground policy, approval, and revocation.
- Risk: coordinate frames differ across providers. Mitigation: explicit `SensorCoordinateFrame` and transformation/provenance metadata.
- Risk: SDK helpers bypass leases. Mitigation: helpers only build canonical commands and no-direct-provider-call gates enforce service dispatch.
- Risk: unavailable host permissions look like empty sensor lists. Mitigation: host status and unavailable/disabled diagnostics are explicit.
- Risk: stream leaks resources after task cancellation. Mitigation: lease state machine closes streams on revocation, timeout, cancellation, and shutdown.
