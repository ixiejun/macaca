# Change: Add Industrial Device Sensors Pack

## Why

Macaca applications need `pack.device.sensors.v1` for safe, audited access to host/device sensors such as accelerometer, gyroscope, magnetometer, barometer, ambient light, proximity, orientation, step counter, and other platform-provided sensor classes. The pack must support discovery, metadata inspection, one-shot reads, bounded streams, batch reads, calibration, coordinate-frame metadata, leases, foreground/background restrictions, and revocation without embedding host-specific code in applications.

Sensors can reveal user behavior, movement, environment, device posture, and side-channel signals. A supplier-grade pack must therefore define permission scopes, sampling/resource budgets, stream lifecycle, redaction, calibration provenance, privacy policy, trace/audit evidence, and unavailable-provider behavior.

## Supplier/API Baseline

The design borrows from mature sensor platforms:

- Android Sensor Framework: sensor discovery, sensor types, sampling period, batching, wake-up sensors, accuracy values, calibration, dynamic sensors, and runtime permission constraints. Official docs: https://developer.android.com/develop/sensors-and-location/sensors/sensors_overview
- Apple Core Motion: accelerometer, gyroscope, magnetometer, device motion, attitude/reference frames, update intervals, availability checks, and privacy-sensitive motion access. Official docs: https://developer.apple.com/documentation/coremotion
- W3C Generic Sensor API: sensor objects, readings, activation, frequency hints, permission policy, secure contexts, and error events. Official spec: https://www.w3.org/TR/generic-sensor/
- W3C DeviceOrientation/Event APIs: orientation/motion readings, permission and privacy considerations for web hosts. Official spec: https://www.w3.org/TR/orientation-event/
- Windows Sensors and Location platform: sensor categories, reports, privacy, and device capability declarations. Official docs: https://learn.microsoft.com/windows/uwp/devices-sensors/
- HarmonyOS Sensor Service: sensor categories, permission-controlled device sensor access, subscription, sampling, and callback model. Official docs: https://developer.huawei.com/consumer/en/doc/harmonyos-guides/sensor-overview

## Macaca Provider-Neutral Mapping

Macaca SHALL model sensors as host capabilities behind a service boundary:

- Sensor discovery becomes `sensors.list` and `sensors.inspect`.
- One-shot sampled reads become `sensors.read`.
- Continuous streams become `sensors.open_stream`, `sensors.read_stream`, and `sensors.close_stream`.
- Batch acquisition becomes `sensors.read_batch`.
- Calibration and accuracy metadata become `sensors.inspect_calibration`.
- Resource control becomes `sensors.acquire_lease` and `sensors.release_lease`.
- Host/provider state becomes `sensors.inspect_host`.

The pack SHALL normalize sensor type, axis/frame, units, accuracy, timestamp clock, sampling frequency, batching support, wake-up behavior, foreground/background policy, and redaction requirements without exposing raw host APIs.

## What Changes

- Add `pack.device.sensors.v1` as a service-backed industrial pack under the device family.
- Define command DTOs for listing, inspecting, reading, streaming, batching, calibration inspection, lease acquisition/release, and host capability inspection.
- Define normalized DTOs for `SensorDescriptor`, `SensorReading`, `SensorVector`, `SensorCoordinateFrame`, `SensorAccuracy`, `SensorStreamLease`, `SensorBatch`, `SensorCalibration`, `SensorHostStatus`, and structured errors.
- Define permission scopes, policy/approval rules, sampling/resource budgets, stream lifetime, foreground/background behavior, revocation, and unavailable diagnostics.
- Require detailed developer documentation under `docs/developer-packs/device/sensors.md`.

## Impact

- Affected specs: `pack-device-sensors`, `developer-pack-industrial-capability-catalog`, `sdk-system-facade`, `service-runtime`, `unified-execution-path`.
- Later affected code: protocol DTOs, descriptor/admission validators, SDK pack client, sensor service provider contract, host provider adapters, mock/unavailable providers, stream lease manager, trace/audit schemas, and boundary gates.
- Validation: `openspec validate add-pack-device-sensors --strict`, stream lifecycle tests, permission denial tests, revocation tests, resource quota tests, no-direct-provider-call gates, and docs coverage checks.

## Non-Goals

- This pack does not own camera frames, microphone/audio capture, file access, notifications, foreground/background app lifecycle, location/geofencing, or application-specific activity recognition.
- This pack does not hardcode Android, Apple, Windows, HarmonyOS, browser, sensor-model, or application-specific behavior into OS-layer routing.
- This pack does not expose raw high-frequency streams, device identifiers, calibration secrets, or unbounded readings in traces, audits, logs, snapshots, or examples.
