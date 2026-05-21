# Design: Autonomy Application Operations Dialog

## Context

Autonomy schedule management is implemented through serviceized Scheduler
commands and application-scoped Web routes. The current UI placement under a
workspace tab is functionally usable, but semantically misleading because the
tab group is used for session/coordinator/agent execution streams.

## Goals

- Represent Autonomy as an application-level OS capability opened from an app-level action button.
- Keep session and agent tabs focused on execution traces and conversation
  scope.
- Preserve the existing serviceized Autonomy API facade and route ownership.
- Match the existing Macaca black/green terminal visual language.
- Keep the change small, reversible, and free of application-specific logic.

## Non-Goals

- Do not introduce a global Autonomy control center.
- Do not create a new dedicated route in this change.
- Do not change Scheduler provider semantics.
- Do not add heartbeat management UI in this change.
- Do not add application-specific schedule presets or workflow shortcuts.

## Decisions

### Decision: Use an application Operations dialog

Autonomy SHALL be mounted under a generic application Operations dialog opened
from the application header. The dialog is a UI facade for app-scoped OS
capabilities. It can host additional generic panels later, but this change only
mounts schedule management.

### Decision: Keep Scheduler data flow unchanged

The dialog reuses the existing `ScheduleManagerPanel`, which calls the existing
`frontend/lib/autonomy.ts` facade. This keeps HTTP calls under
`/api/apps/{app_id}/autonomy/*` and avoids creating shell-owned Scheduler
semantics.

### Decision: Remove Autonomy from tab state

The `activeWorkspaceTab` state SHALL only represent coordinator and delegated
agent streams. It SHALL NOT contain an `autonomy` pseudo-agent value because
Autonomy is not a trace stream.

## Design Patterns

- Facade: `ApplicationOperationsDialog` presents app-scoped OS capabilities while
  hiding individual panel wiring.
- Composite: The dialog can compose multiple independent capability panels without
  making the chat page own their internals.
- Adapter: The frontend remains a presentation adapter over serviceized
  Scheduler routes.

## Risks and Mitigations

- Risk: The chat page is already large.
  - Mitigation: Add a focused component under `frontend/components/autonomy/`
    instead of adding more panel markup directly to the page.
- Risk: A persistent right rail competes with the existing agent/task/status rail.
  - Mitigation: Use a modal dialog opened by a header button and restore the
    existing right-side `AgentPanel`.
- Risk: Operators may expect Autonomy under the old tab temporarily.
  - Mitigation: The Operations rail header explicitly labels it as
    application-scoped OS capability.

## Validation

- OpenSpec strict validation passes.
- Frontend lint passes.
- The Autonomy schedule API still returns JSON through the frontend proxy.
- Manual UI check confirms `AUTONOMY` is no longer in the session/agent tablist,
  the right-side `AgentPanel` remains visible, and Autonomy appears only after
  opening the application Operations dialog.
