## Context

Macaca hosts application-owned UI bundles as generic sandboxed web assets. The
host shell must not learn application-specific component trees, execution
labels, markdown presentation, or business semantics.

## Goals / Non-Goals

- Goal: Move the workbench UI source to a typed, component-oriented app-owned
  bundle.
- Goal: Keep session replay, websocket events, bridge calls, and presentation
  rendering traceable and auditable from the application package.
- Non-goal: Change Macaca's own `frontend/` shell or add application-specific
  branches to OS services.
- Non-goal: Change application-execution service contracts.

## Decisions

- Decision: Use Vite + React + TypeScript inside the application-owned UI
  package.
  Vite emits static assets that Macaca already knows how to host, React gives
  the app package a component model, and TypeScript keeps bridge/state DTOs
  explicit without moving logic into OS layers.
- Decision: Keep markdown parsing and timeline presentation as app-owned
  Strategy-style helpers.
  Macaca remains responsible for durable events and routing while the app owns
  how those events are displayed to users.

## Risks / Trade-offs

- Risk: Build output can be missed during application installation.
  Mitigation: keep `ui/dist/index.html` as the manifest entry and verify the
  installed static asset path with HTTP checks.
- Risk: React state can diverge from mutable bridge refs during replay.
  Mitigation: write replayed events through the same state patch path used by
  bridge calls and session mementos.

## Migration Plan

1. Build the app-owned UI bundle with `npm run build`.
2. Install or sync the application package into the workspace apps directory.
3. Restart the backend so manifest admission discovers the new static entry.
4. Validate that the original application id still resolves and serves the
   React bundle entry.
