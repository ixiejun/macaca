# Change: Add Heartbeat Agent Operations UI

## Why

Operators can manage serviceized scheduled tasks from the application operations dialog, but they cannot inspect or edit native heartbeat agent operation state for the current application. This makes heartbeat execution harder to audit even though heartbeat is now a native service lane.

## What Changes

- Add application-scoped Web routes for sanitized heartbeat operations snapshots and native profile edits.
- Add focused SDK/Heartbeat command support for profile updates.
- Add a frontend Heartbeat Operations panel beside the existing Scheduler panel in the same application operations dialog.
- Preserve the heartbeat/scheduler split: heartbeat cadence and mementos remain owned by `service.heartbeat`.

## Impact

- Affected specs: `web-cli-thin-shell-v0`, `sdk-system-facade`, `serviceization-escape-hatches`
- Affected code: `macaca-proto`, `macaca-heartbeat`, `macaca-sdk`, `macaca-web`, `frontend/components/autonomy`, `frontend/lib/autonomy*`
