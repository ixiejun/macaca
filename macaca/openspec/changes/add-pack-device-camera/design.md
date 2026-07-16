# Device Camera Pack Design

## Context

`pack.device.camera.v1` exposes host camera capability through Macaca's service runtime. Camera APIs differ by mobile, desktop, browser, and embedded hosts, but they share critical primitives: authorization, device discovery, capture sessions, preview/video/photo outputs, constraints, controls, interruptions, and privacy indicators. Macaca must normalize these without letting applications access raw host APIs directly.

This pack owns camera capture sessions and camera-originated media references. Media processing, image analysis models, OCR, local file storage, and application-specific capture UX remain separate packs or applications.

## Supplier Capability Matrix

| Platform/API | Borrowed capability | Macaca mapping |
| --- | --- | --- |
| Android CameraX/Camera2 | lifecycle-bound preview/photo/video/image-analysis use cases, controls, permissions | session state machine, preview lease, photo/video/frame commands, control DTOs |
| Apple AVFoundation | capture session, device inputs, outputs, interruptions, runtime errors | provider adapter, session lifecycle, interruption diagnostics |
| Web getUserMedia/ImageCapture | constraints, tracks, permissions policy, image capture, stream stop | constraints DTO, preview/frame stream, authorization state, track teardown |
| Windows MediaCapture | app capabilities, preview, capture, privacy settings | host status, foreground policy, unavailable/disabled diagnostics |
| HarmonyOS Camera Kit | camera inputs/outputs, capture session configuration, permissions | descriptor-driven session outputs and lifecycle callbacks |

## Goals

- Provide authorization inspection/request, device discovery/inspection, capture session open/close, preview start/stop, photo capture, video recording, frame reads, controls, media reference inspection, revocation, and host status.
- Normalize constraints, facing mode, resolution/fps, focus/exposure/zoom/torch, orientation, timestamps, privacy indicators, media retention, and interruption diagnostics.
- Enforce permission, foreground, approval, resource quotas, media redaction, retention, and revocation before/during capture.
- Support host-native, browser, remote-host, plugin, mock, and unavailable providers through descriptors.
- Provide detailed developer documentation and provider conformance guidance.

## Non-Goals

- Do not own image editing, OCR, computer vision inference, media transcoding, local file persistence, notifications, calls, or application-specific camera UX.
- Do not expose raw frames/media in generic observability.
- Do not branch on host OS, camera model, device id, provider name, or application workflow in OS-layer code.

## Ownership And Boundaries

- Pack id: `pack.device.camera.v1`.
- Capability family: `device`.
- Backing service: device camera service.
- SDK surface: `sdk.packs.device.camera`.
- Command namespace: `camera.*`.
- Application framework owns manifest declaration and app-scoped permission projection.
- Service runtime owns typed dispatch, decorators, capture session lifecycle, output leases, health, snapshots, and unavailable behavior.
- Runtime host owns concrete host/browser/provider adapters through approved composition roots.
- Shells render diagnostics and permission surfaces only through SDK/service events.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `camera.inspect_authorization` | Inspect camera authorization and host state | Returns permission state, prompt eligibility, privacy indicator, disabled reason, and provider class |
| `camera.request_authorization` | Request user-mediated camera authorization | Requires foreground policy and returns granted/denied/limited/prompt-not-allowed state |
| `camera.list_devices` | List available cameras | Returns redacted descriptors without stable hardware identifiers |
| `camera.inspect_device` | Inspect one camera capability | Returns facing mode, supported constraints, controls, output modes, privacy class, and provider limitations |
| `camera.open_session` | Open a scoped capture session | Requires device/constraint selection, output intents, max duration, foreground policy, and resource reservation |
| `camera.start_preview` | Start preview stream lease | Requires active session, preview policy, max duration/fps/resolution, and redaction |
| `camera.stop_preview` | Stop preview stream lease | Releases preview resources idempotently |
| `camera.capture_photo` | Capture still image | Returns bounded media reference with metadata and retention policy |
| `camera.start_recording` | Start video recording | Requires active session, output policy, max duration/size, and resource budget |
| `camera.stop_recording` | Stop video recording | Finalizes media reference and releases recording resources |
| `camera.read_frame` | Read bounded analysis frame/reference | Enforces frame rate, size, privacy, and output redaction |
| `camera.set_controls` | Set focus/exposure/zoom/torch/etc. | Validates supported controls and policy |
| `camera.inspect_controls` | Inspect control state/capabilities | Returns supported ranges, modes, and current state |
| `camera.close_session` | Close capture session | Closes preview/recording outputs, releases resources, emits audit |
| `camera.inspect_host` | Inspect camera provider health/status | Returns disabled/degraded/provider diagnostics |

## DTO Model

- `CameraAuthorization`: state, prompt eligibility, limited mode, host disabled reason, privacy indicator state, and provider class.
- `CameraDescriptor`: opaque id, facing mode, kind, redacted label, supported output modes, privacy class, constraints, controls, and availability.
- `CameraConstraints`: resolution, fps, aspect ratio, facing mode, focus mode, exposure mode, torch, stabilization, audio inclusion flag, and fallback policy.
- `CameraSession`: session id, device ids, constraints, output intents, state, max duration, foreground requirement, approval id, resource reservation, and revocation state.
- `CameraPreviewLease`: preview id, session id, resolution/fps class, delivery mode, started/stopped timestamps, dropped-frame count, and privacy class.
- `CameraFrameReference`: frame id, session id, timestamp, resolution class, format, orientation, redaction state, content reference, and expiry.
- `CameraMediaReference`: media id, kind, duration/size class, format, orientation, thumbnails when allowed, retention class, content scan status, and storage/resource reference.
- `CameraControls`: focus, exposure, white balance, zoom, torch, stabilization, orientation, and supported ranges/modes.
- `CameraHostStatus`: provider class, authorization state, device count class, active sessions, privacy indicator, resource pressure, disabled reason, and diagnostics.
- `CameraError`: denied, unavailable, unsupported, prompt not allowed, foreground required, device unavailable, constraint unsatisfied, session expired, session revoked, privacy indicator unavailable, capture interrupted, media too large, quota exceeded, provider failure, or conflict.

## Permission, Policy, Resource, Entitlement, And Approval

Initial scopes:

- `device.camera.read_status`: authorization, host, device, and control inspection.
- `device.camera.request_permission`: user-mediated authorization request.
- `device.camera.preview`: preview stream.
- `device.camera.capture_photo`: still capture.
- `device.camera.record_video`: video recording.
- `device.camera.read_frame`: bounded frame references for analysis.
- `device.camera.controls`: camera control changes.
- `device.camera.session.manage`: open/close/revoke sessions.

Policy requirements:

- Camera use requires foreground-visible context unless host policy explicitly allows delegated capture.
- Sessions require max duration, output intents, privacy indicator state, and revocation behavior.
- Raw frames/media are never written to generic trace/audit; events store references, hashes/classes, and counters.
- Recording and frame analysis require stricter quotas than still capture.
- Face/document/credential detection belongs to media/AI services; this pack only labels privacy class and redaction state.

## Service Runtime And Provider Strategy

Provider Strategy categories:

- Host-native provider: OS camera APIs.
- Browser provider: MediaDevices/ImageCapture.
- Remote-host provider: delegated trusted host camera.
- Plugin provider: specialized camera/robot/IoT adapter.
- Mock provider: deterministic synthetic media references for tests/docs.
- Unavailable provider: explicit unavailable diagnostics.

Providers declare authorization state, devices, constraints, controls, output modes, privacy indicator support, session limits, recording limits, frame limits, foreground requirements, and health. Provider construction is allowed only in approved runtime composition roots.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, lifecycle, command schemas, DTO schemas, permission scopes, effective availability, authorization state, host status, device descriptors, constraints, output limits, control support, privacy indicator support, policy templates, examples, diagnostics, compatibility, and documentation links.

The implementation SHALL create `docs/developer-packs/device/camera.md` with manifest declarations, scopes, authorization flow, device discovery, session lifecycle, preview, photo, video, frame references, controls, privacy indicators, revocation, unavailable diagnostics, trace/audit reference, and provider conformance checklist.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `camera.pack_declared`
- `camera.admission_validated`
- `camera.policy_decision`
- `camera.authorization_requested`
- `camera.session_opened`
- `camera.preview_started`
- `camera.photo_captured`
- `camera.recording_started`
- `camera.recording_stopped`
- `camera.frame_reference_created`
- `camera.controls_changed`
- `camera.session_closed`
- `camera.session_revoked`
- `camera.command_failed`
- `camera.unavailable`
- `camera.snapshot_recorded`

Events include pack id, command name, service id, descriptor version, trace id, application/session/task/tenant ids when present, provider class, device class, session id hash, media id hash, output mode, resolution/fps class, privacy class, policy decision, latency, and resource counters. Events exclude raw frames, raw media, stable hardware identifiers, faces, documents, credentials, secrets, and unbounded provider payloads.

Snapshots include provider health, authorization state, device descriptor hashes, active session summaries, active output summaries, privacy indicator state, resource pressure, policy template hash, unavailable diagnostics, and sanitized replay pointers.

## Design Patterns

- **Facade**: SDK exposes discovery and command builders while `SystemFacade` carries canonical service calls.
- **Command**: every operation is a typed command/result DTO.
- **Adapter**: host, browser, remote, plugin, mock, and unavailable providers map into Macaca DTOs.
- **Strategy**: provider selection, constraints, outputs, controls, and unavailable behavior are descriptor-driven.
- **Decorator**: trace, policy, resource, entitlement, approval, metering, privacy indicator, and redaction wrap every call.
- **State**: authorization, capture sessions, preview leases, and recordings are explicit state machines.
- **Specification**: admission validates scopes, constraints, foreground state, output intent, max duration, and resource budgets.
- **Observer**: trace, audit, capture lifecycle, health, and service events are subscribable.
- **Memento**: snapshots record sessions/media references for replay without raw frames.
- **Abstract Factory**: providers are created only in approved composition roots.

## Risks And Mitigations

- Risk: camera access bypasses user consent. Mitigation: authorization, foreground policy, privacy indicator checks, and approval gates.
- Risk: raw media leaks in traces. Mitigation: media/frame references and bounded metadata only.
- Risk: long-running sessions leak resources. Mitigation: session state machine with max duration, cancellation, revocation, and shutdown cleanup.
- Risk: provider constraints are inconsistent. Mitigation: normalized descriptors plus unsupported/constraint-unsatisfied diagnostics.
- Risk: SDK helpers bypass host APIs directly. Mitigation: helpers only build canonical service commands and no-direct-provider-call gates enforce dispatch.
