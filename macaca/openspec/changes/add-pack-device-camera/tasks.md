## 1. Research, Scope, And Governance

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design patterns, the umbrella industrial catalog proposal, and this child proposal before implementation.
- [x] 1.2 Record supplier/API comparison notes for Android CameraX/Camera2, Apple AVFoundation, Web MediaDevices/ImageCapture, Windows MediaCapture, and HarmonyOS Camera Kit.
- [x] 1.3 Confirm boundaries with device sensors, local files, foreground/background host capabilities, media image/video/audio packs, AI vision, and application-owned UI.
- [x] 1.4 Record GitNexus CRITICAL/HIGH findings as memo-only evidence before implementation commits, per the current refactor instruction.

## 2. Contract, Descriptor, And DTO Schema

- [x] 2.1 Define provider-neutral commands for `camera.inspect_authorization`, `camera.request_authorization`, `camera.list_devices`, `camera.inspect_device`, `camera.open_session`, `camera.start_preview`, `camera.stop_preview`, `camera.capture_photo`, `camera.start_recording`, `camera.stop_recording`, `camera.read_frame`, `camera.set_controls`, `camera.inspect_controls`, `camera.close_session`, and `camera.inspect_host`.
- [x] 2.2 Define `CameraAuthorization`, `CameraDescriptor`, `CameraConstraints`, `CameraSession`, `CameraPreviewLease`, `CameraFrameReference`, `CameraMediaReference`, `CameraControls`, `CameraHostStatus`, and `CameraError`.
- [x] 2.3 Define typed success, partial, denied, unavailable, unsupported, prompt-not-allowed, foreground-required, device-unavailable, constraint-unsatisfied, session-expired, session-revoked, privacy-indicator-unavailable, capture-interrupted, media-too-large, quota-exceeded, provider-failure, and conflict results.
- [x] 2.4 Define descriptor metadata for pack id, family, lifecycle, command schemas, authorization states, device descriptors, constraints, control support, output modes, privacy indicator support, session limits, permission scopes, policy template, resource budgets, SDK metadata, compatibility, diagnostics, and documentation URL.
- [x] 2.5 Add stable descriptor hashing, version compatibility checks, DTO snapshot fixtures, session lifecycle fixtures, media reference fixtures, redaction fixtures, and schema migration tests.

## 3. Permission, Policy, Resource, Entitlement, And Approval

- [x] 3.1 Implement declaration validation for `device.camera.read_status`, `device.camera.request_permission`, `device.camera.preview`, `device.camera.capture_photo`, `device.camera.record_video`, `device.camera.read_frame`, `device.camera.controls`, and `device.camera.session.manage`.
- [x] 3.2 Enforce authorization, foreground state, privacy indicator, constraints, output intent, media retention, frame rate, duration, size, and redaction policies before dispatch.
- [x] 3.3 Require explicit session constraints, output intents, max duration, resource reservation, cancellation behavior, and revocation behavior.
- [x] 3.4 Add resource reservation and quota checks for active sessions, preview streams, recording duration, frame rate, media size, CPU, memory, retained snapshots, and replay metadata.
- [x] 3.5 Add approval behavior for camera permission request, recording, frame analysis, remote-host capture, privacy-indicator degradation, and delegated/background capture.
- [x] 3.6 Add tests proving denied, unavailable, prompt-not-allowed, foreground-required, session-revoked, privacy-indicator-unavailable, media-too-large, and quota paths do not call concrete providers or leak resources.

## 4. Service Provider And Capture Session Strategy

- [x] 4.1 Implement the device camera service provider contract behind the service runtime; do not construct providers from kernel, SDK, shells, or generic application-framework code.
- [x] 4.2 Add provider descriptor support for host-native, browser, remote-host, plugin, mock, and unavailable provider classes.
- [x] 4.3 Add capture session, preview lease, recording, and frame reference state machines covering requested, active, paused, stopping, closed, expired, revoked, failed, and unavailable states.
- [x] 4.4 Add mock and unavailable providers for deterministic tests; host-specific adapters must remain optional providers or plugin/remote modules.
- [x] 4.5 Add provider conformance tests for authorization, device listing, session open/close, preview, photo, recording, frame reference, controls, host status, redaction, and unsupported-command reporting.
- [ ] 4.6 Add lifecycle, health, snapshot, shutdown, timeout, cancellation, interruption handling, resource cleanup, media reference expiry, and bounded output behavior.

## 5. SDK, Admission, Examples, And ABI

- [x] 5.1 Extend SDK discovery for `pack.device.camera.v1` with command schemas, DTO schemas, permission scopes, examples, availability, authorization state, device descriptors, output limits, control support, privacy indicator support, diagnostics, compatibility, and documentation URL.
- [ ] 5.2 Extend application admission so required declarations block when unavailable/disabled and optional declarations degrade explicitly with effective capability mementos.
- [x] 5.3 Add SDK command helper builders that only produce canonical traced service calls and never construct providers or branch on host/platform/camera-model names.
- [x] 5.4 Add WASM/application ABI exposure for camera commands using provider-neutral DTO schemas and canonical service-call dispatch.
- [x] 5.5 Add generic examples for authorization, device listing, preview, photo capture, video recording, frame reference, controls, close/revoke, and unavailable-provider diagnostics.

## 6. Trace, Audit, Replay, And Boundary Gates

- [x] 6.1 Emit sanitized `camera.pack_declared`, `camera.admission_validated`, `camera.policy_decision`, `camera.authorization_requested`, `camera.session_opened`, `camera.preview_started`, `camera.photo_captured`, `camera.recording_started`, `camera.recording_stopped`, `camera.frame_reference_created`, `camera.controls_changed`, `camera.session_closed`, `camera.session_revoked`, `camera.command_failed`, `camera.unavailable`, and `camera.snapshot_recorded` events.
- [x] 6.2 Add replay tests proving every command is trace-addressable through the canonical service path after refresh/restart without raw camera frames or raw media bytes.
- [x] 6.3 Add dependency-boundary gates proving microkernel, SDK, shells, and generic application framework do not import concrete camera providers or host camera APIs.
- [x] 6.4 Add no-direct-provider-call gates proving all camera commands enter through descriptor-owned service registrations and typed service runtime dispatch.
- [x] 6.5 Add redaction tests for raw frames, media bytes, stable hardware identifiers, faces/documents, provider payloads, credentials, session ids, media references, snapshots, and diagnostics.
- [ ] 6.6 Run `openspec validate add-pack-device-camera --strict`, DTO compatibility tests, authorization tests, session lifecycle tests, media redaction tests, revocation tests, boundary gates, file-size gates, and audit replay checks before marking implementation tasks complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/device/camera.md` with purpose, manifest declarations, required/optional behavior, scopes, command DTOs, result DTOs, authorization, device discovery, sessions, preview, photo, video, frame references, controls, privacy indicators, revocation, unavailable diagnostics, and trace/audit behavior.
- [x] 7.2 Add provider author documentation covering descriptor fields, host adapter responsibilities, session/preview/recording state machines, conformance tests, unsupported behavior, redaction rules, health/snapshot behavior, and replacement strategy.
- [x] 7.3 Add minimal app-facing examples for request authorization, list devices, preview, photo, recording, frame reference, controls, close session, and unavailable-provider diagnostics using generic synthetic data.
- [x] 7.4 Cross-link the guide from SDK discovery metadata and the industrial pack catalog index before marking `add-pack-device-camera` complete.
