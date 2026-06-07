# Change: Add industrial tool observability and shell diagnostics

## Why

Industrial tool execution must be traceable, auditable, explainable, and operable. Users and operators need visible and hidden tools, provider health, policy decisions, approvals, resource leases, invocation lifecycle, artifacts, and replayable audit references without moving semantics into Web, CLI, or frontend shells.

## What Changes

- Add sanitized EventLog/SSE events for planning, hidden diagnostics, policy, approval, leases, invocation lifecycle, artifacts, and provider health.
- Add `tool.audit.query`, `tool.provider.status`, `tool.provider.health`, `tool.policy.explain`, and `tool.catalog.snapshot` behavior.
- Add Web/CLI thin shell routes for tool diagnostics and audit surfaces.
- Add frontend panels for visible/hidden tool plans, provider health, invocation traces, approval state, artifacts, and audit refs.
- Add audit replay tests and shell-boundary tests.

## Impact

- Affected specs: `tool-observability`, `web-cli-thin-shell-v0`, `web-cli-thin-shell-completion`
- Affected code: `macaca-runtime-host`, `macaca-web`, `frontend/`, CLI shell adapters
- Depends on: contracts, planning, invocation, environments/gateway proposals

## Constraints

- Web, CLI, and frontend must render diagnostics only through SDK/service clients.
- Policy, approval semantics, provider lifecycle, and invocation routing must remain in services.
- Logs, UI payloads, EventLog rows, SSE events, snapshots, and audit records must be bounded and sanitized.
