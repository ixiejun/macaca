# Application-Owned UI Runtime Design

## Context

Macaca OS is evolving from a task runner into an agent application operating
system. WASM applications must be installable, runnable, auditable, and visually
independent without requiring Macaca Web or Desktop shells to know application
business logic. A declarative presentation schema is useful for fallback views,
but it is not sufficient for branded, highly interactive applications because it
would force Macaca to invent a constrained UI language.

The selected direction is Application-Owned UI Runtime. Each application may
ship its own web UI bundle, built with React, Vue, Svelte, or plain browser
technologies. Macaca owns the host contract, sandbox, bridge, lifecycle,
capability policy, trace, and audit. The application owns its visual design,
component choices, interaction model, and brand system.

## Platform Research Summary

macOS treats applications and extensions as separately packaged, capability
bounded units. App extensions can run separately from the containing app and the
host defines extension points and communication. App Sandbox limits access to
files, network, hardware, and system resources through explicit entitlements.
This supports the Macaca model of app-owned UI plus host-owned permissions.

OpenHarmony packages applications as HAPs containing code, resources,
third-party libraries, and configuration. UIAbility owns interactive UI
lifecycle through WindowStage, and UIExtensionAbility can embed UI from another
ability, including process-isolated models. This supports the Macaca model of
packaged UI resources, lifecycle callbacks, and future independent UI process
hosting.

WeChat Mini Programs separate logic and view layers, provide bridge APIs, and
offer optional UI libraries such as WeUI. However, WXML/WXSS are custom platform
languages. Macaca should reuse the modern web ecosystem instead of inventing an
equivalent DSL.

## Goals

- Let a Macaca application fully own its UI brand, layout, interaction model,
  animation, and frontend framework.
- Keep Macaca shells generic: Web UI and future Desktop hosts must not contain
  app-name, service-name, workflow-name, domain-pack, or business-specific
  rendering branches.
- Provide a stable host bridge for application UI bundles to call OS services
  such as `service.call`, `trace.emit`, `session.read`, and `storage.kv`.
- Enforce manifest-declared capabilities before any bridge call reaches host
  services.
- Record replayable audit evidence for UI runtime load, bridge handshakes,
  policy decisions, service routes, and bridge results.
- Make React the first developer experience while keeping the runtime protocol
  framework-neutral.

## Non-Goals

- Do not create a Macaca-specific UI language as the primary application UI
  model.
- Do not require applications to use `@macaca/ui`.
- Do not let application UI code access host services directly without the
  bridge and capability policy.
- Do not embed finance, stock, writing, news, or any application-specific
  renderer in Macaca shells.
- Do not support native-process UI extensions in the first implementation
  slice; reserve the manifest shape for future Desktop/WebView evolution.

## Architecture

The architecture follows an App Shell plus Bridge pattern.

1. The application package declares a UI bundle in `app.yaml`.
2. Macaca admission validates the declared UI entry, assets, sandbox profile,
   and bridge capabilities.
3. The Web shell exposes a generic application surface that loads the bundle in
   a sandboxed iframe.
4. The application bundle imports `@macaca/app-sdk` or uses the raw bridge
   protocol to perform a handshake.
5. The shell only accepts bridge calls that match the manifest's declared
   bridge capabilities and current session scope.
6. The host routes accepted calls to generic runtime services and records audit
   events for every decision and result.

This keeps all application-specific visual work inside the application package
while keeping the operating system responsible for governance.

## Manifest Contract

The first manifest shape should be explicit and data-only:

```yaml
ui:
  runtime: web_bundle
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

`runtime` identifies the host strategy. `framework` is descriptive metadata for
developer tooling and diagnostics, not a renderer switch. `entry` and `assets`
must resolve inside the application package. `sandbox` describes the minimum
isolation and CSP profile. `bridge.required` and `bridge.optional` are
capability declarations that feed policy sync.

## Bridge Contract

The bridge protocol is message-based so Web iframe and future Desktop WebView
hosts can share the same model:

- `macaca.handshake`: app UI announces runtime version, application id, surface
  id, and requested bridge capabilities.
- `macaca.call`: app UI requests an OS capability with a command id, operation,
  payload, and trace context.
- `macaca.result`: host returns a structured result, policy denial, or runtime
  error.
- `macaca.event`: host publishes session, theme, lifecycle, or trace events.

Every bridge message must carry enough envelope metadata for audit:

- `application_id`
- `session_id`
- `surface_id`
- `trace_id`
- `command_id`
- `capability`
- `operation`

The host must reject missing scope, unknown capability, undeclared capability,
oversized payloads, unsupported bridge versions, and stale handshakes.

## SDK And UI Kit

Macaca should publish two developer packages:

- `@macaca/app-sdk`: framework-neutral bridge client plus React hooks in the
  first release. It hides `postMessage`, command correlation, retries, timeout,
  and structured errors.
- `@macaca/ui`: optional React component library and design tokens for apps that
  want a native Macaca feel. Apps may ignore it and use their own design system.

The UI kit must be a convenience library, not an operating system dependency.
The host shell must never assume the app used Macaca UI components.

## Security And Governance

The sandbox policy is fail-closed:

- UI bundle files must be loaded from the installed application package.
- The iframe must use a restrictive `sandbox` attribute and CSP.
- Network access is disabled unless declared and admitted by policy.
- Bridge calls are the only supported path to host services.
- Secrets remain host-owned and never enter the UI bundle.
- Payload size, message rate, and command timeout are bounded.
- All host decisions are logged with reason codes.

The first Web implementation may use iframe isolation. The contract should keep
`runtime: web_bundle` abstract enough for Desktop to map it to a WebView later.

## Trace And Audit

Macaca must emit audit events for:

- UI manifest admission.
- UI surface creation and teardown.
- Bundle load and CSP policy selection.
- Bridge handshake accepted or rejected.
- Bridge capability policy decision.
- Routed host service call result.
- Renderer fallback or runtime error.

Audit metadata must include `application_id`, `session_id`, `surface_id`,
`bridge_version`, `capability`, `operation`, `policy_decision`, `reason_code`,
`latency_ms`, and `trace_id` where available.

## Fallback Rendering

Declarative presentation schema remains useful only as fallback:

- render execution envelopes when an app has no UI bundle,
- render error or audit surfaces when a UI bundle fails to load,
- support simple internal tools.

It must not become the primary UI programming model.

## First Implementation Slice

The first slice should prove the end-to-end model with the independent
`wasm-stock-agent-app`:

1. Extend Macaca app manifest parsing with `ui`.
2. Validate UI entry paths, asset globs, sandbox policy, and bridge capability
   declarations.
3. Expose UI metadata through existing app/session APIs.
4. Add a generic Web iframe host surface.
5. Add a bridge runtime with handshake, call, result, timeout, and audit.
6. Add a minimal `@macaca/app-sdk` package inside the frontend workspace for
   local development.
7. Add a React UI bundle to `wasm-stock-agent-app` that calls generic bridge
   capabilities, not direct Macaca internals.
8. Verify that Web UI renders the app-owned UI and that all service calls still
   pass through generic service policy and audit.

## Open Questions For Later Phases

- Whether signed UI bundle digests should be mandatory before marketplace
  distribution.
- Whether Desktop should host app UI in WebView, isolated process WebView, or a
  native UI extension model.
- Whether app-owned UI should support background surfaces, widgets, or command
  palette surfaces.
- Whether `@macaca/ui` should be versioned with semantic design tokens or host
  runtime versions.

