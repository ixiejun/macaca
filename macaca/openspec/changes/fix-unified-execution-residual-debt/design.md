## Context

The current system already routes canonical YAML/WASM application execution through `service.application_execution` and agent execution through `service.agent_execution`. The remaining defects are residual compatibility surfaces that make the terminal model harder to audit.

## Goals

- Preserve the existing public service and application execution contracts.
- Remove fallback paths that select agents or event history outside the canonical execution projection.
- Keep presentation code as an Adapter over declared application UI bridge capabilities.
- Keep provider/test wiring aligned with current serviceized provider contracts.

## Non-Goals

- Implementing a new WASM runtime provider.
- Changing YAML or WASM application manifest semantics.
- Introducing application-specific logic for Codex Workbench or fullstack-autodev.

## Decisions

- **Strategy: runtime-id scoped manifest selection.** The application shell adapter will only select manifests whose kernel id is bound to the requested runtime application. Name-only matching remains useful for diagnostics, but it must not authorize a manifest selection.
- **Adapter/Bridge: Workbench UI uses only `app.execution` for execution history.** Historical recovery will use `app.execution replay`; generic `session.read events` remains a shell capability for other read-only views but not as Workbench execution replay fallback.
- **Null Object/unavailable behavior: debug loop disabled.** The browser LLM/tool loop will return an explicit UI event explaining that debug execution is disabled instead of starting an alternate execution engine.
- **Configuration over hardcoded defaults.** The application registry will not export or use a concrete default application name. Tests may still use fixture names inside test code.
- **Service contract alignment.** Integration tests will use current LLM service/provider contracts from the service crate instead of a retired SDK module path.

## Risks / Trade-offs

- Removing name-only agent selection may hide agents for callers that have not finished runtime binding. That is desired at terminal state because unbound agents are not safely scoped.
- Removing generic session replay fallback may make old pre-convergence sessions show no Workbench execution timeline. This is acceptable because migration should be handled by backend projections, not UI-side parallel history paths.
- Disabling debug loop removes a developer convenience path. Keeping it would preserve a second execution engine in a production bundle.

## Validation

- `openspec validate fix-unified-execution-residual-debt --strict`
- `cargo test -p macaca-integration-tests --test no_debt_token_gate -- --nocapture`
- `cargo test -p macaca-integration-tests --test unified_audit_replay_terminal_gate -- --nocapture`
- `cargo test -p macaca-integration-tests --test p5_terminal_audit_gates -- --nocapture`
- `cargo test -p macaca-integration-tests --no-run`
- Workbench UI typecheck/build where package scripts are available.
