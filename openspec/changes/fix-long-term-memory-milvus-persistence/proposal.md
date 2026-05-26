# Change: Fix long-term memory Milvus persistence

## Why

Macaca local runs can enable active vector memory and still return zero long-term memory rows because the web runtime composes the workspace memory runtime with an in-memory vector store instead of the configured Milvus backend. Chat sessions are persisted as session logs, but successful session completion does not write any scoped memory item through the Memory Service boundary.

## What Changes

- Add a configuration-driven memory runtime composition path that selects the configured vector backend and embedding provider while keeping Web as a composition root, not a semantic owner.
- Preserve provider-neutral Memory Service calls for all long-term session capture writes.
- Add a bounded, application-neutral session completion memory capture path that writes scoped session-shared memory after successful chat completion.
- Add sanitized logs for backend selection, memory runtime degradation, and session-memory capture outcomes.
- Keep active recall and context layers vendor-neutral; they continue to depend on `SystemMemoryClient` / `MemoryRuntimeFacade`, never Milvus directly.

## Impact

- Affected specs: `macaca-memory-runtime-persistence`
- Affected code:
  - `macaca/crates/services/macaca-memory/src/backend.rs`
  - `macaca/crates/services/macaca-memory/src/store.rs`
  - `macaca/crates/shells/macaca-web/src/lib.rs`
  - `macaca/crates/shells/macaca-web/src/session_memory_capture.rs`
  - `macaca/crates/shells/macaca-web/src/chat_orchestrator.rs`
