# Device Sensors Pack

`pack.device.sensors.v1` provides provider-neutral sensor discovery,
inspection, one-shot reads, bounded streams, stream reads, stream close, batch
reads, calibration inspection, stream leases, and host sensor status.

The pack does not expose raw host APIs. It becomes callable only when a
serviceized sensor provider is registered by the runtime composition root.

## Manifest Declaration

```toml
[service_contract]
optional_packs = ["pack.device.sensors.v1"]
```

Unavailable optional declarations report `device_sensors_provider_not_installed`.
Required declarations block readiness until host support, permissions, policy,
resources, and command-compatible provider schemas are available.

## Commands

- `sensors.list`, `sensors.inspect`, and `sensors.inspect_host`: discover
  `SensorDescriptor` and `SensorHostStatus`.
- `sensors.read`: returns a bounded `SensorReading`.
- `sensors.open_stream`, `sensors.read_stream`, and `sensors.close_stream`:
  manage `SensorStreamLease` and bounded chunks.
- `sensors.read_batch`: returns `SensorBatch` references.
- `sensors.inspect_calibration`: returns `SensorCalibration`.
- `sensors.acquire_lease` and `sensors.release_lease`: manage stream/resource
  lease state.

## DTOs And Results

Core DTOs include `SensorDescriptor`, `SensorType`, `SensorReading`,
`SensorVector`, `SensorCoordinateFrame`, `SensorAccuracy`,
`SensorStreamLease`, `SensorBatch`, `SensorCalibration`, `SensorHostStatus`,
and `SensorError`. Result statuses include success, partial, denied,
unavailable, unsupported, disabled, permission-prompt-required,
foreground-required, lease-expired, lease-revoked, sample-rate-too-high,
stream-overflow, timeout, quota-exceeded, calibration-unavailable,
provider-failure, and conflict.

## Provider Mapping

Android Sensor Framework, Apple Core Motion, W3C Generic Sensor,
DeviceOrientation, Windows Sensors, and HarmonyOS Sensor Service concepts map
to sensor descriptors, coordinate frames, accuracy, calibration, stream leases,
batching, foreground requirements, and host status. Raw sample vectors, stable
hardware identifiers, host payloads, and unbounded stream chunks are not OS
observability data.

## App-Facing Examples

Applications call the pack through typed commands and receive opaque references
plus bounded payloads. Each example assumes the app already declared
`pack.device.sensors.v1` and every call carries trace, session, tenant, and
capability context through the SDK facade.

- List sensors with `sensors.list` and render only descriptor labels, sensor
  types, accuracy bands, and permission status.
- Read one synthetic accelerometer sample with `sensors.read` using
  `sensor_id = "sensor.accelerometer.synthetic"` and a bounded timeout.
- Open a bounded stream with `sensors.open_stream`, read at most one chunk with
  `sensors.read_stream`, and close the returned `stream_lease_id`.
- Read batched synthetic samples with `sensors.read_batch` and keep only the
  returned batch reference plus aggregate metadata in app state.
- Inspect calibration with `sensors.inspect_calibration` before accepting
  high-accuracy readings.
- Revoke access with `sensors.release_lease` or `sensors.close_stream` when the
  UI leaves the foreground.
- Display unavailable diagnostics from
  `device_sensors_provider_not_installed` without retry loops or fake samples.

## Conformance

Provider authors must document descriptor fields, host adapter responsibilities,
stream lease state-machine behavior, unsupported-command behavior, redaction,
health/snapshot behavior, provider replacement strategy, deterministic
unavailable behavior, and no raw sample leakage.
