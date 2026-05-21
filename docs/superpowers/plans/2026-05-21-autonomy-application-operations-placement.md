# Autonomy Application Operations Dialog Placement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move Autonomy schedule management out of the session/agent workspace tabs and persistent right rail into an application-level Operations dialog.

**Architecture:** This is a presentation ownership change only. The frontend keeps using the existing serviceized Autonomy facade, while a new small Operations dialog component acts as a Facade for application-scoped OS capabilities.

**Tech Stack:** Next.js App Router, React client components, TypeScript, existing Macaca terminal CSS, OpenSpec.

---

## File Structure

- Create: `frontend/components/autonomy/ApplicationOperationsDialog.tsx`
  - Owns the application-level Operations modal.
  - Mounts `ScheduleManagerPanel` under an application-scoped shell.
  - Contains no Scheduler semantics and no application-specific templates.
- Modify: `frontend/app/chat/[appId]/page.tsx`
  - Remove the `AUTONOMY` workspace tab.
  - Remove `activeWorkspaceTab === 'autonomy'` special cases.
  - Restore the existing right-side `AgentPanel`.
  - Render an application-level button that opens `ApplicationOperationsDialog`.
- Modify: `frontend/app/globals.css`
  - Add focused Operations rail styles that match the current black/green terminal language.
- Create: `openspec/changes/move-autonomy-to-application-operations-rail/*`
  - Proposal, design, tasks, and a `web-cli-thin-shell-v0` spec delta.

## Task 1: OpenSpec placement proposal

**Files:**
- Create: `openspec/changes/move-autonomy-to-application-operations-rail/proposal.md`
- Create: `openspec/changes/move-autonomy-to-application-operations-rail/design.md`
- Create: `openspec/changes/move-autonomy-to-application-operations-rail/tasks.md`
- Create: `openspec/changes/move-autonomy-to-application-operations-rail/specs/web-cli-thin-shell-v0/spec.md`

- [ ] **Step 1: Write the OpenSpec delta**

The delta must add one requirement: Autonomy schedule management is an application-level Operations surface and SHALL NOT appear as a session/agent trace tab.

- [ ] **Step 2: Validate OpenSpec**

Run:

```bash
openspec validate move-autonomy-to-application-operations-rail --strict
```

Expected: `Change 'move-autonomy-to-application-operations-rail' is valid`.

## Task 2: Add application Operations dialog component

**Files:**
- Create: `frontend/components/autonomy/ApplicationOperationsDialog.tsx`

- [ ] **Step 1: Add the component**

Implementation shape:

```tsx
'use client';

import { ScheduleManagerPanel } from './ScheduleManagerPanel';

interface ApplicationOperationsDialogProps {
  appId: string;
  open: boolean;
  onClose: () => void;
}

export function ApplicationOperationsDialog({ appId, open, onClose }: ApplicationOperationsDialogProps) {
  if (!open) return null;

  return (
    <div className="application-operations-dialog-backdrop" role="presentation">
      <section className="application-operations-dialog" role="dialog" aria-modal="true" aria-labelledby="application-operations-title">
        <header className="application-operations-dialog-header">
        <div>
          <div className="application-operations-kicker">APP OPERATIONS</div>
          <h2 id="application-operations-title">Autonomy</h2>
          <p>Application-scoped OS capabilities that run independently from the active session.</p>
        </div>
          <button type="button" onClick={onClose}>CLOSE</button>
        </header>
        <ScheduleManagerPanel appId={appId} />
      </section>
    </div>
  );
}
```

## Task 3: Move Autonomy out of the session workspace tab state

**Files:**
- Modify: `frontend/app/chat/[appId]/page.tsx`

- [ ] **Step 1: Replace the schedule panel import**

Use:

```tsx
import AgentPanel from '@/components/AgentPanel';
import { ApplicationOperationsDialog } from '@/components/autonomy/ApplicationOperationsDialog';
```

Remove:

```tsx
import { ApplicationOperationsRail } from '@/components/autonomy/ApplicationOperationsRail';
```

- [ ] **Step 2: Remove the Autonomy workspace tab button**

Delete the `AUTONOMY` button from the `workspace-agent-tabs` tablist.

- [ ] **Step 3: Remove Autonomy-specific tab guards**

Replace guards such as:

```tsx
if (activeWorkspaceTab === 'coordinator' || activeWorkspaceTab === 'autonomy') return;
```

with:

```tsx
if (activeWorkspaceTab === 'coordinator') return;
```

Remove any branch that renders `ScheduleManagerPanel` inside the conversation stack.

- [ ] **Step 4: Render the Operations dialog and restore the AgentPanel**

Render:

```tsx
<button type="button" onClick={() => setOperationsDialogOpen(true)}>APP OPERATIONS</button>
<ApplicationOperationsDialog appId={appId} open={operationsDialogOpen} onClose={() => setOperationsDialogOpen(false)} />
<AgentPanel agents={agents} appId={appId} sessionId={currentSession?.session_id} />
```

The dialog is application-level and the right rail remains the existing agents/task/status area.

## Task 4: Add terminal-style Operations dialog CSS

**Files:**
- Modify: `frontend/app/globals.css`

- [ ] **Step 1: Add focused styles**

Add CSS classes:

```css
.application-operations-dialog-backdrop {
  position: fixed;
  inset: 0;
  z-index: 80;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 32px;
  background: rgba(0, 0, 0, 0.74);
}

.application-operations-dialog {
  width: min(1180px, 96vw);
  max-height: min(860px, 92vh);
  display: flex;
  flex-direction: column;
  border: 1px solid rgba(13, 84, 43, 0.5);
  border-radius: 8px;
  overflow: hidden;
  background: rgba(0, 0, 0, 0.92);
}

.application-operations-dialog-header {
  padding: 16px;
  border-bottom: 1px solid rgba(13, 84, 43, 0.5);
  background: linear-gradient(180deg, rgba(3, 46, 21, 0.22), rgba(0, 0, 0, 0));
}

.application-operations-kicker {
  color: #00c950;
  font-family: "SFMono-Regular", ui-monospace, Menlo, Monaco, Consolas, "Liberation Mono", monospace;
  font-size: 10px;
  letter-spacing: 0.16em;
  text-transform: uppercase;
}
```

Also constrain the embedded `autonomy-manager` so it scrolls inside the dialog.

## Task 5: Validate

**Files:**
- Test: frontend lint and OpenSpec validation.

- [ ] **Step 1: Run OpenSpec validation**

Run:

```bash
openspec validate move-autonomy-to-application-operations-rail --strict
```

Expected: valid.

- [ ] **Step 2: Run frontend lint**

Run:

```bash
cd frontend && npm run lint
```

Expected: exit 0.

- [ ] **Step 3: Probe schedule API continuity**

Run with a known application id:

```bash
curl -sS -i http://127.0.0.1:3000/api/apps/<app_id>/autonomy/schedules
```

Expected: `200 OK` with JSON containing `count`, `schedules`, and `trace_id`.

- [ ] **Step 4: Manual UI check**

Open an application chat page and confirm:

- No `AUTONOMY` tab appears in the session/agent tablist.
- The right-side AgentPanel remains visible in the normal page layout.
- The app-level Operations button opens an Autonomy dialog.
- Schedule list loading still calls `/api/apps/{app_id}/autonomy/schedules`.
