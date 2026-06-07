## Context

Macaca OS governance keeps runtime ownership separate from presentation shells. The kernel owns agent runtime status, application runtime owns WASM host dispatch, and Web is only the adapter that starts the session and renders events.

Agentless WASM chat intentionally bypasses the framework coordinator loop. That path is correct for generic WASM applications, but it also skipped the status writes that framework sessions perform around `service.agent_execution`.

## Decision

Use the existing kernel `AgentStatusTracker` as the status owner. The Web chat route will update app-declared agent activity at lifecycle edges it already observes:

- `Working` before `application_client.host_dispatch`.
- `Idle` after any terminal host-dispatch result that returned normally.
- `Error` when host dispatch itself fails.
- Delegated executor `TaskStarted` events mark the target agent `Working`.
- Delegated executor `TaskCompleted` and `TaskFailed` events mark the target agent `Idle` or `Error`.

This is a boundary adapter action, not a crypto-specific runtime rule. It does not inspect application names, finance payloads, crypto symbols, service ids, prompts, provider payloads, or guest output.

## Alternatives Considered

- Rely only on the active-session override in the agent status route. This was too weak because it depends on metadata projection and does not update the canonical kernel status.
- Add crypto app-specific frontend heuristics. Rejected because the OS must stay generic and shells must not own runtime semantics.
- Move status writes into the WASM runtime provider. Rejected for this fix because the provider should not know Web session presentation identity; a broader runtime lifecycle service can be designed later if needed.

## Risks

- A stale active session could leave an agent working if the spawned task aborts before cleanup. The implementation keeps status cleanup in both success and error branches before removing the active session.
- Executor event ordering already drives the delegated trace tabs. The status projection intentionally reuses the same event stream so the coarse agent panel and detailed trace surface do not diverge.
