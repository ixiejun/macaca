# Device Camera Pack

`pack.device.camera.v1` provides provider-neutral camera authorization,
device discovery, device inspection, capture sessions, preview leases, photo
capture, video recording, frame references, controls, session close, and host
camera status.

The pack does not perform media processing, AI vision, OCR, local file
persistence, or application capture UI. Those capabilities remain owned by their
respective packs or applications.

## Manifest Declaration

```toml
[service_contract]
optional_packs = ["pack.device.camera.v1"]
```

Unavailable optional declarations report `device_camera_provider_not_installed`.

## Commands

- `camera.inspect_authorization` and `camera.request_authorization`: inspect or
  request host-owned authorization.
- `camera.list_devices` and `camera.inspect_device`: discover
  `CameraDescriptor` metadata.
- `camera.open_session`, `camera.close_session`, `camera.start_preview`, and
  `camera.stop_preview`: manage `CameraSession` and `CameraPreviewLease`.
- `camera.capture_photo`, `camera.start_recording`, `camera.stop_recording`,
  and `camera.read_frame`: return references, never raw media bytes.
- `camera.set_controls` and `camera.inspect_controls`: manage
  `CameraControls`.
- `camera.inspect_host`: reports `CameraHostStatus`.

## DTOs And Results

Core DTOs include `CameraAuthorization`, `CameraDescriptor`,
`CameraConstraints`, `CameraSession`, `CameraPreviewLease`,
`CameraFrameReference`, `CameraMediaReference`, `CameraControls`,
`CameraHostStatus`, and `CameraError`. Result statuses include success,
partial, denied, unavailable, unsupported, prompt-not-allowed,
foreground-required, device-unavailable, constraint-unsatisfied,
session-expired, session-revoked, privacy-indicator-unavailable,
capture-interrupted, media-too-large, quota-exceeded, provider-failure, and
conflict.

## Provider Mapping

Android CameraX/Camera2, Apple AVFoundation, Web MediaDevices/ImageCapture,
Windows MediaCapture, and HarmonyOS Camera Kit map into authorization,
descriptors, constraints, sessions, preview leases, frame references, media
references, controls, privacy indicators, and host status. Raw frames, raw
media, hardware identifiers, faces/documents, provider payloads, and credentials
must stay out of traces and SDK diagnostics.

## App-Facing Examples

Applications call the pack through typed commands and receive session, preview,
frame, and media references instead of raw camera bytes. Each example assumes
the app already declared `pack.device.camera.v1` and every command carries
trace, session, tenant, and capability context through the SDK facade.

- Inspect or request authorization with `camera.inspect_authorization` and
  `camera.request_authorization`, then branch only on provider-neutral status.
- List devices with `camera.list_devices` and show redacted descriptors such as
  facing mode, capability class, and availability.
- Open a synthetic preview flow with `camera.open_session` followed by
  `camera.start_preview`, storing only `camera_session_id` and
  `preview_lease_id`.
- Capture a photo with `camera.capture_photo` and pass the returned
  `CameraMediaReference` to media or local-files packs when needed.
- Start and stop recording with `camera.start_recording` and
  `camera.stop_recording`, keeping bounded recording metadata only.
- Read a frame reference with `camera.read_frame` for downstream vision packs
  without exposing raw frame contents to logs.
- Adjust controls with `camera.set_controls` after `camera.inspect_controls`
  confirms support, then close the session with `camera.close_session`.
- Display unavailable diagnostics from `device_camera_provider_not_installed`
  without pretending a camera session exists.

## Conformance

Provider authors must cover descriptor fields, host adapter responsibilities,
session/preview/recording state machines, unsupported behavior, interruption
handling, redaction, health/snapshot behavior, replacement strategy,
unavailable behavior, and no raw frame or media leakage.
