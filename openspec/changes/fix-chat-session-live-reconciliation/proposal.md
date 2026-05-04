# Change: Fix chat session live reconciliation

## Why

The chat workspace can briefly remove live coordinator output after goal creation because session hydration and stream refresh replace active `turns` with an older persisted session snapshot.

## What Changes

- Reconcile persisted session turns with newer live frontend turns instead of replacing them blindly.
- Adapt live `plan_decision` stream events into coordinator trace steps.
- Preserve existing backend HTTP/SSE contracts and storage behavior.

## Impact

- Affected specs: `chat-session-live-reconciliation`
- Affected code: `frontend/app/chat/[appId]/page.tsx`, `frontend/lib/session-turns.ts`, `frontend/lib/types.ts`
- Compatibility impact: no backend API or persistence contract changes.
