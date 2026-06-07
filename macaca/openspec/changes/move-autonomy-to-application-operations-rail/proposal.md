# Change: Move Autonomy to Application Operations Dialog

## Why

The Autonomy schedule-management panel is currently reachable from the
session/agent workspace tab group. That placement suggests Autonomy belongs to a
single session, coordinator stream, or delegated agent trace, but Scheduler jobs
are application-scoped OS capabilities.

Macaca Web must communicate the same ownership boundaries as the serviceized
backend: sessions own conversation and trace views, while applications own
access to app-scoped system capabilities.

## What Changes

- Move Autonomy schedule management out of the session/agent workspace tabs.
- Add an application-level Operations button that opens an Autonomy dialog.
- Reuse the existing serviceized Autonomy frontend facade and schedule
  management components.
- Preserve existing `/api/apps/{app_id}/autonomy/*` routes and Scheduler
  service contracts.
- Keep the shell generic: no application-specific workflows, schedule
  templates, provider names, driver names, model names, gateway names, chain
  names, payment names, or business-domain branches.

## Impact

- Affected specs:
  - `web-cli-thin-shell-v0`
- Affected code:
  - `frontend/app/chat/[appId]/page.tsx`
  - `frontend/components/autonomy/ApplicationOperationsDialog.tsx`
  - `frontend/app/globals.css`
- No backend service, SDK, provider, or Scheduler contract changes are required.
