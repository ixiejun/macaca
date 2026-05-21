# Tasks: Move Autonomy to Application Operations Dialog

## 1. OpenSpec and Governance

- [x] 1.1 Validate the proposal against
  `macaca-os-architecture-governance.md`,
  `macaca-os-microkernel-boundaries.md`, and
  `macaca-os-serviceization-allowlist.md`.
- [x] 1.2 Confirm this is a presentation ownership change only and does not
  alter Scheduler service contracts.
- [x] 1.3 Run
  `openspec validate move-autonomy-to-application-operations-rail --strict`.

## 2. Frontend Operations Dialog

- [x] 2.1 Add a focused `ApplicationOperationsDialog` component under
  `frontend/components/autonomy/`.
- [x] 2.2 Mount the existing `ScheduleManagerPanel` inside the dialog without
  adding application-specific templates, workflows, provider names, driver
  names, model names, gateway names, chain names, payment names, or business
  branches.
- [x] 2.3 Add concise English comments explaining that the dialog is an
  application-level OS capability surface, not a session trace owner.

## 3. Chat Workspace Placement

- [x] 3.1 Remove the `AUTONOMY` button from the session/agent workspace tablist.
- [x] 3.2 Remove the `activeWorkspaceTab === 'autonomy'` render branch and
  guard logic.
- [x] 3.3 Restore the existing right-side `AgentPanel`.
- [x] 3.4 Render an application-level header button that opens the Operations
  dialog.
- [x] 3.5 Preserve coordinator and delegated-agent trace behavior.

## 4. Styling

- [x] 4.1 Add focused Operations dialog CSS in `frontend/app/globals.css`.
- [x] 4.2 Match the current Macaca terminal visual language.
- [x] 4.3 Keep the embedded schedule manager scrollable inside the dialog.

## 5. Validation

- [x] 5.1 Run OpenSpec strict validation.
- [x] 5.2 Run frontend lint.
- [x] 5.3 Probe `/api/apps/{app_id}/autonomy/schedules` through the frontend
  proxy with a valid application id.
- [x] 5.4 Statically confirm the Autonomy panel is outside the session/agent
  tablist and persistent right rail by checking that the chat page no longer
  contains an `AUTONOMY` tab, `activeWorkspaceTab === 'autonomy'` branch, or
  persistent `ApplicationOperationsRail` render.
- [x] 5.5 Mark these tasks complete only after implementation and validation
  evidence exists.
