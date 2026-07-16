# Device Sensors Pack Research

## Purpose

This note records supplier/API comparison, Macaca provider-neutral mapping,
boundary decisions, existing platform inventory, and GitNexus memo evidence for
`pack.device.sensors.v1`. The sensors pack must expose discovery, inspection,
one-shot reads, bounded streams, batch reads, calibration inspection, leases,
host status, revocation, and redaction through typed service commands. It must
not own camera frames, microphone/audio capture, local files, notifications,
foreground/background lifecycle, geolocation/place search, or
application-specific activity recognition.

## Source Baseline

- Android Sensor Framework:
  <https://developer.android.com/develop/sensors-and-location/sensors/sensors_overview>
- Apple Core Motion:
  <https://developer.apple.com/documentation/coremotion>
- W3C Generic Sensor API:
  <https://www.w3.org/TR/generic-sensor/>
- W3C DeviceOrientation Event:
  <https://www.w3.org/TR/orientation-event/>
- Windows Sensors:
  <https://learn.microsoft.com/windows/uwp/devices-sensors/>
- HarmonyOS Sensor Service:
  <https://developer.huawei.com/consumer/en/doc/harmonyos-guides/sensor-overview>

## Supplier API Notes

- Android Sensor Framework contributes sensor discovery, sensor types, sampling
  periods, batching, wake-up sensors, dynamic sensors, accuracy values,
  calibration, and permission constraints. Macaca should normalize sampling,
  batching, wake behavior, and accuracy metadata.
- Apple Core Motion contributes accelerometer, gyroscope, magnetometer, device
  motion, update intervals, attitude/reference frames, availability checks, and
  privacy-sensitive motion access. Macaca should model coordinate frames and
  update intervals without exposing Core Motion managers.
- W3C Generic Sensor contributes activation lifecycle, frequency hints,
  readings, permission policy, secure contexts, and error events. Macaca should
  model stream leases, permissions, and structured errors.
- W3C DeviceOrientation contributes orientation/motion events and privacy
  considerations for browser hosts. Macaca should treat orientation data as a
  privacy-sensitive sensor stream with explicit bounds.
- Windows and HarmonyOS contribute sensor categories, reports, privacy/device
  capability declarations, subscriptions, sampling, and callbacks. Macaca
  should expose host disabled/unavailable diagnostics and provider capability
  descriptors.

## Macaca-Owned Abstractions

`pack.device.sensors.v1` should define `SensorDescriptor`, `SensorType`,
`SensorReading`, `SensorVector`, `SensorCoordinateFrame`, `SensorAccuracy`,
`SensorStreamLease`, `SensorBatch`, `SensorCalibration`,
`SensorHostStatus`, and `SensorError`.

The DTOs must carry sensor type, privacy class, units, axes, coordinate frame,
timestamp clock, accuracy, calibration state, sampling frequency, batching
limits, wake behavior, foreground/background policy, lease state, stream
sequence, dropped-sample counters, resource reservation, redaction class,
bounded provider reason codes, and replay pointers. Stable hardware ids, raw
high-frequency sample vectors in generic observability, calibration secrets,
credentials, and unbounded readings are rejected.

## Boundary Decisions

- Device camera owns optical capture and frame/media references; sensors own
  non-camera sampled device/environment readings.
- Device local-files owns scoped host file grants and transfers; sensors do not
  persist data directly to host files.
- Device notifications owns host notification display; sensors may trigger
  events only through higher-level workflow/application logic.
- Foreground/background host owns background eligibility and lifecycle policy;
  sensors consume lifecycle evidence before background or high-frequency access.
- Location packs own maps, geocode, routes, place search, and timezone; sensors
  do not absorb geolocation or place semantics.
- Application lifecycle services and applications own product-specific activity
  recognition; sensors expose bounded readings and streams only.

## Existing Macaca Platform Inventory

- `crates/foundation/macaca-proto/src/domain_pack_contract/` provides reusable
  descriptor, lifecycle, availability, diagnostics, policy, SDK metadata, and
  unavailable diagnostic structures.
- `crates/facade/macaca-sdk/src/system_facade.rs` provides the Facade pattern
  for upper layers; sensor SDK helpers should only create canonical traced
  service calls.
- `crates/runtime/macaca-host-composition/src/runtime_host.rs` and
  `crates/kernel/macaca-kernel/src/domain_pack_registration.rs` provide generic
  provider registration/composition mechanics.
- Kernel policy, audit, trace, and redaction modules provide reusable
  enforcement and observability substrate, but current evidence does not prove
  sensor-specific DTOs, descriptors, providers, SDK helpers, ABI, tests, or docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
