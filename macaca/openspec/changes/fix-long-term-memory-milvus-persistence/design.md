## Context

The memory architecture already defines provider-neutral facades, typed Memory Service commands, a Milvus vector store, a DashScope embedding provider, active recall, and governance/candidate layers. The current local web composition root does not connect these pieces: it constructs `MemoryBackendFactory::test_manager()`, which uses `InMemoryVectorStore` and `MockEmbedding` regardless of `config.memory.vector.backend`.

## Goals

- Use the configured memory backend in the web composition root.
- Keep Milvus and embedding details inside `macaca-memory` construction utilities, not in context, kernel, SDK, or application code.
- Persist successful chat-session completion evidence through `SystemMemoryClient::remember` with explicit application/session/agent/trace scope.
- Log only bounded identifiers, lengths, backend ids, and memory ids; do not log raw prompts or model outputs.

## Non-Goals

- Do not implement a full autonomous promotion policy in this change.
- Do not make context or active recall call Milvus directly.
- Do not add application-specific memory rules or business-domain extraction.
- Do not require Milvus to run for unit tests.

## Decisions

- **Abstract Factory:** `MemoryBackendFactory` gains a configuration-driven constructor that returns a manager over trait-object vector/embedding providers. This keeps backend selection replaceable and testable.
- **Facade/Command:** session capture writes use `SystemMemoryClient` and `MemoryRememberCommand`, preserving trace, scope, policy, and structured failures.
- **Observer:** chat completion becomes an observation point. It emits one bounded session-shared memory item only after successful completion.
- **Fail explicit:** if the configured backend or embedding provider is unavailable, startup or write paths surface structured memory errors and logs instead of pretending Milvus was used.

## Risks / Trade-offs

- If local Milvus or DashScope credentials are absent, memory writes may fail. The shell logs a warning with stable ids and continues the chat session because memory persistence must not make task completion fail.
- The first session-capture path stores bounded session evidence as scoped memory. Later work can route it through a durable candidate/promotion store, but this change keeps the write behind Memory Service and marks metadata as automatic session capture.

## Verification

- Unit tests prove configured runtime profile selection, trait-object vector/embedding dispatch, and session capture writes a scoped command.
- Existing active recall and context tests prove recall remains provider-neutral and report-only.
- OpenSpec validation proves the behavioral contract is documented.
