# Change: Add Industrial Device Camera Pack

## Why

Macaca applications need `pack.device.camera.v1` for safe host-mediated camera use: capability discovery, authorization, capture-session lifecycle, preview streams, still capture, video recording, frame analysis, camera controls, media references, privacy indicators, and revocation. Camera access is one of the most sensitive host capabilities because it can capture people, screens, documents, locations, and private environments.

The current template does not define session leases, foreground/consent rules, media redaction, stream lifecycle, capture output ownership, or provider conformance. This proposal turns camera into a supplier-grade, provider-neutral device pack.

## Supplier/API Baseline

- Android CameraX/Camera2: camera discovery, lifecycle-bound sessions, preview, image capture, video capture, image analysis, camera controls, permission gates, and foreground UX. Official docs: https://developer.android.com/media/camera/camerax and https://developer.android.com/media/camera/camera2
- Apple AVFoundation capture: `AVCaptureSession`, device discovery, photo/video outputs, metadata outputs, authorization, interruptions, and runtime errors. Official docs: https://developer.apple.com/documentation/avfoundation/capture_setup
- Web MediaDevices/ImageCapture: `getUserMedia`, device enumeration, constraints, tracks, permissions policy, frame capture, and stream teardown. Official docs: https://developer.mozilla.org/docs/Web/API/MediaDevices/getUserMedia and https://www.w3.org/TR/image-capture/
- Windows MediaCapture: camera initialization, preview, photo/video capture, device selection, privacy settings, and app capability declarations. Official docs: https://learn.microsoft.com/windows/apps/develop/camera/
- HarmonyOS Camera Kit: camera input/output, preview, photo/video capture, session configuration, permission-controlled access, and lifecycle callbacks. Official docs: https://developer.huawei.com/consumer/en/doc/harmonyos-guides/camera-overview

## Macaca Provider-Neutral Mapping

Macaca SHALL expose camera access through scoped capture sessions:

- Discovery and permission state become `camera.list_devices`, `camera.inspect_device`, and `camera.inspect_authorization`.
- Consent request becomes `camera.request_authorization`.
- Capture lifecycle becomes `camera.open_session`, `camera.start_preview`, `camera.capture_photo`, `camera.start_recording`, `camera.stop_recording`, `camera.read_frame`, and `camera.close_session`.
- Controls become `camera.set_controls` and `camera.inspect_controls`.
- Output becomes bounded `CameraMediaReference` and `CameraFrameReference` DTOs, not raw bytes in traces.
- Host state becomes `camera.inspect_host`.

## What Changes

- Add `pack.device.camera.v1` as a service-backed industrial pack under the device family.
- Define command DTOs for authorization, device discovery, session/preview/photo/video/frame lifecycle, controls, media references, revocation, and host status.
- Define DTOs for camera descriptors, constraints, capture sessions, preview leases, frame metadata, media references, controls, authorization, host status, and structured errors.
- Define permission scopes, policy/approval rules, foreground requirements, privacy indicators, resource quotas, media retention, and unavailable diagnostics.
- Require detailed developer documentation under `docs/developer-packs/device/camera.md`.

## Impact

- Affected specs: `pack-device-camera`, `developer-pack-industrial-capability-catalog`, `sdk-system-facade`, `service-runtime`, `unified-execution-path`.
- Later affected code: protocol DTOs, descriptor/admission validators, SDK pack client, camera service provider contract, host/browser camera adapters, mock/unavailable providers, capture session manager, trace/audit schemas, and boundary gates.
- Validation: `openspec validate add-pack-device-camera --strict`, authorization tests, session lifecycle tests, media redaction tests, revocation tests, foreground policy tests, no-direct-provider-call gates, and docs coverage checks.

## Non-Goals

- This pack does not own image editing, OCR, computer vision inference, media transcoding, local file storage, notification/call UI, or application-specific capture workflows.
- This pack does not hardcode Android, Apple, Windows, browser, HarmonyOS, camera model, device id, provider name, or application workflow in OS-layer routing.
- This pack does not expose raw camera frames, raw media bytes, stable hardware identifiers, faces, documents, credentials, raw provider payloads, or unbounded captures in traces, audits, snapshots, logs, or examples.
