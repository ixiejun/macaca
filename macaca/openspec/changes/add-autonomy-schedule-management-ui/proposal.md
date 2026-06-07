# Change: Add Autonomy Schedule Management UI

## Why

Macaca now has serviceized Scheduler and Heartbeat services plus an explicitly
enabled local autonomy runtime, but operators cannot yet browse and manage
serviceized scheduler jobs from the Web UI. The existing legacy schedule routes
are not an acceptable foundation for this feature because they use a direct task
scheduler path rather than the new `service.scheduler` boundary.

## What Changes

- Add application-scoped serviceized Scheduler job management routes under
  `/api/apps/{app_id}/autonomy/*`.
- Add focused SDK/Scheduler client commands for list, get, update, delete, and
  lifecycle transitions.
- Add a generic Web UI `AUTONOMY / SCHEDULES` surface that matches the current
  Macaca terminal workspace style.
- Add bounded run monitoring and safe trace/audit diagnostics in the UI.
- Add boundary gates preventing the new UI from using legacy direct scheduler
  paths or constructing providers outside approved composition roots.
- Document the operator workflow and serviceized ownership model.

## Impact

- Affected specs:
  - `scheduler-service`
  - `sdk-system-facade`
  - `web-cli-thin-shell-v0`
  - `serviceization-escape-hatches`
- Affected code:
  - `macaca-proto` Scheduler DTOs
  - `macaca-sdk` Scheduler focused client
  - `macaca-scheduler` local provider
  - `macaca-web` route adapters and bootstrap
  - `frontend` autonomy API facade and components
  - integration boundary gates
  - autonomy service documentation

## Governance

This change must preserve:

- Web/frontend as shell adapters only.
- Scheduler lifecycle ownership in `service.scheduler`.
- Heartbeat wake ownership in `service.heartbeat`.
- Provider construction only in approved runtime-host/service-provider
  composition roots.
- Trace context on every mutation and query.
- Sanitized logs and audit-friendly response DTOs.
- No application-specific schedule templates, workflow names, business rules,
  provider names, driver names, model names, chain names, payment names, or
  gateway names in generic OS code.
