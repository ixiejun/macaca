# Device Camera Pack Research

## Purpose

This note records supplier/API comparison, Macaca provider-neutral mapping,
boundary decisions, existing platform inventory, and GitNexus memo evidence for
`pack.device.camera.v1`. The camera pack must expose authorization, device
discovery, capture-session lifecycle, preview, photo capture, video recording,
bounded frame references, controls, host status, revocation, freshness, and
redaction through typed service commands. It must not own media processing,
local file persistence, AI vision inference, or application-specific capture UI.

## Source Baseline

- Android CameraX and Camera2:
  <https://developer.android.com/media/camera/camerax> and
  <https://developer.android.com/media/camera/camera2>
- Apple AVFoundation capture setup:
  <https://developer.apple.com/documentation/avfoundation/capture_setup>
- Web MediaDevices and ImageCapture:
  <https://developer.mozilla.org/docs/Web/API/MediaDevices/getUserMedia> and
  <https://www.w3.org/TR/image-capture/>
- Windows camera development:
  <https://learn.microsoft.com/windows/apps/develop/camera/>
- HarmonyOS Camera Kit:
  <https://developer.huawei.com/consumer/en/doc/harmonyos-guides/camera-overview>

## Supplier API Notes

- Android CameraX/Camera2 contribute permission-gated camera discovery,
  lifecycle-bound use cases, preview, image capture, video capture, image
  analysis, controls, interruption behavior, and foreground UX expectations.
  Macaca should normalize these as sessions, preview leases, media references,
  frame references, controls, and structured interruption diagnostics.
- Apple AVFoundation contributes capture sessions, input/output graph
  configuration, device discovery, photo/video/metadata outputs,
  authorization, runtime errors, and interruptions. Macaca should treat the
  capture graph as provider internals behind capability descriptors.
- Web MediaDevices/ImageCapture contributes permission prompts, constraints,
  media tracks, stream teardown, permissions policy, image capture, and secure
  context constraints. Macaca should model constraints and prompt eligibility
  without exposing browser tracks as stable DTOs.
- Windows MediaCapture contributes capability declarations, preview, capture,
  device selection, privacy settings, and host disabled states. Macaca should
  expose host status and unavailable diagnostics before provider dispatch.
- HarmonyOS Camera Kit contributes camera input/output objects, session
  configuration, permission-controlled capture, and lifecycle callbacks. Macaca
  should map this to the same capture-session state machine used by other hosts.

## Macaca-Owned Abstractions

`pack.device.camera.v1` should define `CameraAuthorization`,
`CameraDescriptor`, `CameraConstraints`, `CameraSession`,
`CameraPreviewLease`, `CameraFrameReference`, `CameraMediaReference`,
`CameraControls`, `CameraHostStatus`, and `CameraError`.

The DTOs must carry permission state, prompt eligibility, foreground
requirement, privacy-indicator status, device class, facing mode, constraints,
session state, output intent, media retention class, frame/media expiry,
control support, interruption reason, resource reservation, redaction class,
bounded provider reason codes, and replay pointers. Raw frames, raw media bytes,
stable hardware identifiers, faces, documents, credentials, raw provider
payloads, and unbounded capture data are rejected.

## Boundary Decisions

- Device sensors own inertial/environmental sampled data; camera owns optical
  capture sessions and camera-originated frame/media references only.
- Device local files own user-selected file handles and transfers; camera may
  return media references but does not persist arbitrary local files.
- Foreground/background host owns visibility, lease, and background execution
  policy evidence; camera consumes that evidence before sensitive capture.
- Media image/video/audio packs own editing, transcoding, encoding,
  transformation, and analysis of media artifacts after capture.
- AI vision owns model inference over approved image/frame references; camera
  only supplies bounded references and redaction metadata.
- Applications own capture UI composition; OS layers own generic session,
  policy, trace, audit, resource, and provider boundaries.

## Existing Macaca Platform Inventory

- `crates/foundation/macaca-proto/src/domain_pack_contract/` provides reusable
  descriptor, lifecycle, availability, diagnostics, policy, SDK metadata, and
  unavailable diagnostic structures.
- `crates/facade/macaca-sdk/src/system_facade.rs` provides the Facade pattern
  for app/shell callers; camera SDK helpers should only create canonical traced
  service calls.
- `crates/runtime/macaca-host-composition/src/runtime_host.rs` and
  `crates/kernel/macaca-kernel/src/domain_pack_registration.rs` provide generic
  provider registration/composition mechanics.
- Kernel policy, audit, trace, and redaction modules provide reusable
  enforcement and observability substrate, but current evidence does not prove
  camera-specific DTOs, descriptors, providers, SDK helpers, ABI, tests, or docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
