# Design: Application-Owned UI Runtime

## Overview

Macaca will treat application UI as an app-owned artifact. The operating system
does not render business-specific cards or interpret service identifiers.
Instead, it loads a declared UI bundle in a sandboxed host surface and exposes a
capability-governed bridge.

The first implementation targets Web shell iframe hosting. The same contract is
designed so Desktop can later host the same bundle in a WebView or stronger
process-isolated container.

## Prior Art

- macOS app extensions show that host-defined extension points can launch
  separately packaged code and communicate through defined APIs.
- macOS App Sandbox shows capability-bounded access to files, network, hardware,
  and system resources.
- OpenHarmony HAP/UIAbility shows app packages containing code, resources, and
  configuration with lifecycle-bound UI entry points.
- OpenHarmony UIExtensionAbility shows embeddable UI supplied by another
  ability, including independent-process models.
- WeChat Mini Programs show the usefulness of host bridge APIs and optional UI
  kits, but Macaca will not reproduce WXML/WXSS because modern web frameworks
  already solve application UI.

## Technical Decisions

### Decision 1: App-owned web bundle is the primary UI model

Applications declare `ui.runtime: web_bundle`, `entry`, `assets`, and sandbox
policy in their manifest. Macaca loads the entry from the installed package.

Rationale: This keeps application brand and interaction design inside the
application, reuses React/Vue/Svelte ecosystems, and avoids OS-level business
rendering branches.

### Decision 2: Bridge calls are capability governed

The UI bundle cannot call host services directly. It sends typed bridge
messages to the host shell. The host checks manifest-declared bridge
capabilities and runtime policy before routing.

Rationale: This preserves the same fail-closed model used by WASM
`service.call` and makes UI actions traceable.

### Decision 3: `@macaca/ui` is optional

Macaca may provide a UI Kit and design tokens, but application bundles are not
required to use them. The host shell must render any admitted web bundle the
same way regardless of UI library choice.

Rationale: This matches the WeUI-style convenience model without constraining
application brand or framework choice.

### Decision 4: Presentation schema is fallback only

Declarative schema can render audit/error/simple result surfaces when no app UI
bundle exists or when the bundle fails to load. It is not the primary app UI
runtime.

Rationale: This keeps useful generic diagnostics while avoiding a custom UI DSL.

## Manifest Shape

```yaml
ui:
  runtime: web_bundle
  surface:
    mode: application
    chrome: app_owned
  framework: react
  entry: dist/ui/index.html
  assets:
    - dist/ui/assets/**
  sandbox:
    isolation: iframe
    csp: strict
    network: declared
  bridge:
    required:
      - service.call
      - trace.emit
      - session.read
    optional:
      - storage.kv
      - theme.read
  theme:
    mode: app_owned
```

`runtime` describes the loading adapter. `surface` describes where that loaded
UI belongs in the host shell. Keeping those responsibilities separate prevents
the Web shell from treating every web bundle as a chat attachment.

`surface.mode: application` means the application owns the center interaction
column. The shell keeps global navigation, page header, and the universal
right-side AgentPanel, but it SHALL NOT render main-thread tabs, conversation
turns, or the bottom chat composer inside that center column.

`surface.mode: session` means the host chat/session shell remains primary and
the application may later customize declared slots through app-owned bundles or
optional UI Kit components. Missing `surface` defaults to `session` with
`chrome: host` for compatibility.

## Bridge Message Envelope

All bridge messages must include:

- `type`
- `bridge_version`
- `application_id`
- `session_id`
- `surface_id`
- `trace_id`
- `command_id`

Call messages also include `capability`, `operation`, and `payload`.

## Security Rules

- UI entry and assets must resolve within the installed app package.
- Host surfaces must use restrictive iframe sandboxing and CSP.
- Network access is denied unless declared and admitted.
- Bridge calls fail closed on missing scope, undeclared capability, unsupported
  version, oversized payload, timeout, or stale handshake.
- Host secrets are never exposed to UI bundles.
- Every denied bridge call returns a structured error and audit event.

## Audit Events

The runtime must log:

- `ui.admission`
- `ui.surface.create`
- `ui.surface.destroy`
- `ui.bundle.load`
- `ui.bridge.handshake`
- `ui.bridge.policy_decision`
- `ui.bridge.route_result`
- `ui.bridge.error`

## Migration Strategy

Existing chat-only applications continue to work. Applications without `ui`
fall back to the current chat/result surfaces. WASM applications can add `ui`
incrementally without changing their service contract.
