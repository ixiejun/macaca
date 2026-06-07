# Application Surface Mode Design

## Context

Application-Owned UI Runtime currently proves that Macaca can load an
application-supplied web bundle through a generic bridge. The first slice still
mounts that bundle above the chat conversation, so the main thread tab,
conversation stream, session logs, and bottom composer remain host-owned. That
is correct for chat-first applications, but it is wrong for product-like
applications that should own the center interaction column.

Macaca needs an explicit manifest-level surface contract. The contract must say
whether a UI bundle replaces the center conversation column or extends the
session shell. The Web shell and future Desktop shell must interpret that
contract without checking application ids, service ids, domain packs, or
business data.

## Goals

- Let applications declare whether their UI is a full application workspace or
  a session-shell extension.
- Keep `ui.runtime` focused on loading strategy and add a separate
  `ui.surface` contract for placement and host chrome ownership.
- Preserve existing chat-first applications by defaulting missing surface
  declarations to the session shell.
- Let application UI replace the center main thread window, agent tabs,
  conversation stream, and bottom chat composer.
- Keep global shell navigation, page header, and the universal right-side
  AgentPanel owned by Macaca for every application.
- Keep the host bridge, policy, trace, and audit path identical for both
  surface modes.
- Leave room for a future optional Macaca UI Kit and declarative session slots
  without making the OS render business-specific cards.

## Non-Goals

- Do not add app-specific renderers to Macaca Web or Desktop.
- Do not create a custom Macaca UI language as the primary app UI model.
- Do not remove the existing chat shell for chat-first applications.
- Do not require app-owned bundles to use Macaca UI Kit components.

## Manifest Contract

Applications declare surface mode under `ui.surface`:

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
```

`surface.mode` accepts:

- `application`: the app owns the center interaction column. The shell keeps
  global navigation, page header, and the universal right-side AgentPanel, but
  it does not render main-thread chat UI, conversation turns, agent trace tabs,
  or the composer inside the center column.
- `session`: the host keeps the chat/session workspace and may mount app-owned
  or UI-Kit-backed slots inside that shell.

`surface.chrome` accepts:

- `app_owned`: workspace chrome belongs to the app bundle.
- `host`: workspace chrome belongs to the host shell.

When `ui.surface` is missing, Macaca defaults to:

```yaml
surface:
  mode: session
  chrome: host
```

This preserves compatibility for existing chat-only and first-slice web-bundle
applications.

## Architecture

The design uses Strategy, Adapter, and Composition patterns.

- `ApplicationSurfaceStrategy` renders an app-owned center column. It composes
  the generic iframe/WebView adapter and the capability bridge, but does not
  include chat shell widgets.
- `SessionSurfaceStrategy` renders the existing chat shell and optional future
  session slots. It remains the default.
- `AppOwnedUiSurface` remains the runtime adapter for web bundles.
- A shell router chooses the strategy from sanitized manifest metadata, not
  application-specific names.

This keeps runtime loading, bridge policy, and shell placement separate.

## Data Flow

1. Application admission parses and validates `ui.surface`.
2. Application Service projects sanitized `ui.surface` metadata with the rest
   of the UI runtime view.
3. Web shell fetches `AppInfo`.
4. The chat route delegates workspace rendering to a surface router.
5. For `application` mode, the router renders global shell navigation, header,
   universal AgentPanel, and the app-owned center surface.
6. For `session` mode, the router keeps the existing chat/session shell.
7. Bridge calls from either mode continue through the same generic
   `/api/apps/{app_id}/ui/bridge` route and service policy checks.

## Trace And Audit

The host must log surface projection and surface selection with:

- `application_id`
- `ui_runtime`
- `surface_mode`
- `surface_chrome`
- `bridge_required`
- `bridge_optional`

Bridge call audit remains unchanged because surface mode controls placement,
not service authority.

## Error Handling

- Unknown `surface.mode` or `surface.chrome` values are rejected during manifest
  parsing.
- Missing `ui.surface` falls back to session mode.
- If a full application bundle cannot load, the Web shell shows a generic
  runtime error inside the full workspace area and does not fall back to
  app-specific cards.

## Testing

- Rust manifest tests cover default session mode, application mode parsing, and
  invalid surface values.
- Rust projection tests cover sanitized surface metadata.
- Frontend type/build checks cover application mode routing.
- Manual verification opens `wasm-stock-agent-app` and confirms the chat
  composer/main-thread tabs do not appear in the center column for
  `surface.mode: application`, while the right-side AgentPanel remains visible.
