# Change: Update Codex Workbench Agent Collaboration

## Why

The Codex WASM Workbench declares coordinator, planner, coder, and reviewer agents, but the production WASM host-command plan currently delegates only to the coder. This prevents model-decided collaboration depth and makes complex tasks behave like a single-agent execution.

## What Changes

- Update the Workbench application package so production execution delegates through coordinator, planner, coder, and reviewer in sequence.
- Require the coordinator to use model judgment to classify task complexity and produce the collaboration plan, instead of relying on keyword or application-specific rules.
- Pass prior agent outputs through generic Component Model host-command result placeholders so downstream agents can consume the model-authored plan and handoff notes.
- Keep all collaboration semantics in the application package; Macaca OS continues to execute provider-neutral `agent_delegate` host imports and service calls.

## Impact

- Affected specs: `codex-workbench-agent-collaboration`
- Affected code: `apps/codex-wasm-workbench/app.yaml`, `apps/codex-wasm-workbench/workflows/*.md`, `apps/codex-wasm-workbench/scripts/build-wasm.sh`, generated Workbench WASM artifacts
- Boundary impact: no kernel, runtime-host, service, SDK, Web shell, or frontend semantic ownership changes
