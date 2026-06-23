# Change: Fix unified execution residual debt

## Why

Recent architecture auditing found that the canonical execution path is mostly enforced, but residual compatibility branches still weaken the terminal-state guarantees. The remaining debt includes name-only agent manifest selection, a hardcoded default application name, UI-side replay fallback, a debug-only UI execution loop that bypasses `service.application_execution`, and an integration test still referencing a retired SDK LLM surface.

## What Changes

- Remove application-scoped agent manifest name-only fallback so runtime ids remain the authoritative binding.
- Remove the application-framework default app constant and require callers to choose an app by explicit configuration or lookup.
- Keep Codex Workbench execution and replay on the `app.execution` bridge path; disable the debug browser loop in production UI code.
- Fix integration coverage to use the current serviceized LLM provider contract instead of the retired SDK LLM module.
- Clear retired debt tokens that now fail executable gates.

## Impact

- Affected specs: `unified-execution-path`, `web-cli-thin-shell-v0`
- Affected code: application shell adapter, application registry, Codex Workbench UI, domain-pack wiring integration test, debt-token gate fixtures
- Boundary model: no new capability ownership; this tightens existing microkernel/service/application/shell boundaries.
