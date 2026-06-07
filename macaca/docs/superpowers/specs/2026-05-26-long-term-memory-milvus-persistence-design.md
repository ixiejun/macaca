# Long-Term Memory Milvus Persistence Design

## Problem

Active vector memory now runs before model calls, but the runtime has no durable long-term rows to recall. The local web host constructs an in-memory vector store and mock embedding provider even when config declares Milvus and DashScope. Chat sessions are stored in the session log, but the successful completion path does not write any scoped memory through the Memory Service.

## Design

Use `macaca-memory` as the owner of backend construction. `MemoryBackendFactory` will act as an Abstract Factory that can build a runtime manager from `MacacaConfig.memory`. Web remains a composition root and only chooses the configured factory result; it does not know Milvus internals.

Successful `/api/chat/v2` service-agent completion will call a small web adapter that observes the completed session and sends one bounded `MemoryRememberCommand` through `SystemMemoryClient`. The command uses `SessionShared` scope with application id, session id, agent name, trace context, vector layer, and metadata that marks the source as automatic session completion capture.

## Patterns

- Abstract Factory for backend selection.
- Facade and Command for Memory Service writes.
- Observer for session-completion capture.
- Strategy remains available for provider replacement through config and future provider runtime profiles.

## Safety

No application-specific code or business routing is added. Logs include only stable ids, lengths, backend ids, provider ids, and memory ids. Memory persistence failures are warning-only for chat completion, because missing optional backend infrastructure must not fail the primary task.
