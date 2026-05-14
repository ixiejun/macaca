# Change: Add Application-Owned UI Runtime

## Why

Macaca applications need fully branded, highly interactive UI surfaces without
forcing the operating system to know application business logic or invent a new
UI language. The platform should host app-owned React/Vue/Svelte web bundles
through a governed bridge, while Macaca remains responsible for sandboxing,
capability policy, lifecycle, trace, and audit.

## What Changes

- Add a manifest-level `ui` declaration for application-owned web bundles.
- Validate UI entry paths, asset scopes, sandbox policy, and bridge capability
  declarations during app admission.
- Expose application UI metadata through shell-facing app/session APIs.
- Add a generic Web iframe host surface that loads app UI bundles without
  app-specific rendering code.
- Add a message-based host bridge for `service.call`, `trace.emit`,
  `session.read`, and future declared capabilities.
- Record audit events for UI admission, surface lifecycle, bridge handshake,
  policy decisions, routed calls, and bridge results.
- Introduce developer-facing `@macaca/app-sdk` and optional `@macaca/ui`
  direction without making UI Kit usage mandatory.

## Impact

- Affected specs: `application-ui-runtime`
- Affected code:
  - `macaca/crates/application/macaca-app`
  - `macaca/crates/runtime/macaca-runtime-host`
  - `macaca/crates/shells/macaca-web`
  - `frontend/app/chat/[appId]`
  - `frontend/components`
  - `frontend/lib`
  - `/Users/quantum/Code/dev/wasm-stock-agent-app`

