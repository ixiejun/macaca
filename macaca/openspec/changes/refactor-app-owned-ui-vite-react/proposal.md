# Change: Refactor app-owned UI bundles to Vite React tooling

## Why

Application-owned UI bundles need a maintainable component model for rich
execution streams, markdown rendering, and bridge state without moving
application presentation logic into Macaca's own frontend.

## What Changes

- Allow an application-owned web bundle to use Vite, React, and TypeScript as
  its private build toolchain.
- Keep Macaca responsible only for generic manifest admission, static bundle
  hosting, sandboxing, bridge routing, trace, and audit.
- Keep app-specific presentation, session mementos, markdown rendering, and
  execution-stream UI state inside the installed application package.

## Impact

- Affected specs: application-owned-ui-runtime
- Affected code: `apps/codex-wasm-workbench/app.yaml`,
  `apps/codex-wasm-workbench/ui/**`
- Non-goals: no changes to Macaca's `frontend/`, shell routing, kernel,
  service contracts, or microkernel boundaries.
