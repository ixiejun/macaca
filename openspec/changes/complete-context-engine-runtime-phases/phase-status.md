# Context Engineering Phase Status

This file separates contract availability from runtime completion.

| Phase | Contract | Runtime | Diagnostics | Verification |
|-------|----------|---------|-------------|--------------|
| Phase 0 Observability | Partial | Partial | Partial | Partial |
| Phase 1 PromptComposer | Partial | Partial | Partial | Partial |
| Phase 2 Pruning | Partial | Partial | Partial | Partial |
| Phase 3 Compaction/Lineage | Partial | Partial | Partial | Partial |
| Phase 4 Memory/Wiki Recall | Partial | Partial | Partial | Partial |
| Phase 5 Pluggable Engine | Partial | Partial | Partial | Partial |

Contract means traits, value objects, policies, or unit tests exist.
Runtime means supported framework/runtime model-call paths use the feature.
Diagnostics means EventLog/API/UI expose the behavior.
Verification means targeted tests prove the runtime behavior.

The previous `add-context-engine-policy-phases` checklist mostly tracks contract-level work. This change tracks runtime completion.

Runtime progress in this change:

- System-level context config exists with `legacy` defaults.
- Built-in `legacy`, `windowed`, `pruning`, and `summary` engines are registered in `ContextRuntimeFacade`.
- Framework model calls use selected context engine and persist `context_report`; non-legacy engines can alter the outgoing payload.
- Runtime direct loop emits a durable-equivalent `DriverTrace` context report when an event channel is available.
- Framework prompt assembly now uses typed `PromptSection`s and loads configured workspace guide files.
- Manual compaction and lineage query APIs exist.
- Read-only `memory_search` / `memory_get` tools and optional preflight recall run when `context.recall.expose_memory_tools` is enabled; wiki/digest recall entry points remain to be modeled.
- Coordinator chat streams surface `context_compaction` and `context_lineage_updated`.

Remaining runtime gaps:

- App/agent profile engine override is not implemented because `AppManifest` impact is CRITICAL and needs a dedicated migration.
- Wiki/digest read-only recall (separate from memory tools) and durable original pruning payloads (task 5.3) remain open.
- Full E2E coverage across every LLM path is not complete.
