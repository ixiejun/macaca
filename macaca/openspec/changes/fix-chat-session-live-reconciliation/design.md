## Context

The chat page receives session state from multiple sources:

- optimistic local turns when a prompt is submitted,
- live `/api/chat/v2` SSE events,
- live session stream events,
- persisted session detail fetched through `fetchSession`,
- EventLog replay fetched through `fetchSessionEvents`.

The current implementation treats persisted session detail as authoritative even when it is older than live state. This creates a Memento misuse: restoring an old snapshot overwrites newer in-memory state until persistence catches up.

## Goals

- Keep live assistant output and trace steps visible during session hydration and stream refresh.
- Display live `plan_decision` events through the existing coordinator trace renderer.
- Keep the first fix narrow and frontend-only.
- Avoid application-, workflow-, agent-, or driver-specific behavior.

## Non-Goals

- Do not change backend SSE event names or payloads.
- Do not change backend session detail or EventLog schemas.
- Do not migrate app-scoped plan decision persistence in this slice.
- Do not rewrite the chat page state model broadly.

## Decisions

### Reconcile persisted snapshots through a helper

Use a small helper as a frontend Facade/Mediator over session-turn reconciliation. The helper receives live turns and persisted turns, then prefers the more complete state per same-index same-role turn.

For assistant turns, it preserves:

- non-empty live content when persisted content is empty or shorter,
- the longer coordinator trace list,
- the longer driver trace list,
- live delegated traces when persisted has none,
- live pending/error/stopped status when persisted has no status.

This avoids blind concatenation in the first slice and reduces duplicate trace risk.

### Treat `plan_decision` as an adapter event

Add `plan_decision` to the frontend stream event union and adapt it to the already supported `TraceStep` shape:

```ts
{ type: 'plan_decision', decision_data: event.data }
```

The payload stays opaque so the frontend does not encode business-specific plan semantics.

## Risks / Trade-offs

- Same-index same-role matching can miss complex turn rebuilds. Mitigation: keep the first slice narrow and validate the active last assistant turn flow.
- Longer-list selection can still miss server corrections with equal-length traces. Mitigation: do not attempt deep semantic reconciliation until stable event IDs exist.
- Backend app-scoped `plan_decisions` remains noisy. Mitigation: track as a separate backend cleanup after the flicker is fixed.

## Validation

- `openspec validate fix-chat-session-live-reconciliation --strict`
- `cd frontend && npm run lint`
- Manual create-goal smoke test: live message remains visible, does not duplicate, and remains after persistence catches up.
