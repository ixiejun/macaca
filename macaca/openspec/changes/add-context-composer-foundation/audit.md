# Audit notes (implementation gate)

## 1.3 GitNexus impact / detect_changes

- GitNexus MCP was not available in this Cursor sandbox. Core symbols touched include
  `macaca_context::composer::*`, `ContextReport::composer`, and `ContextFacade::assemble_model_context`.
- Before merging, run locally: `gitnexus_impact` (for `ContextFacade`, `ContextRuntimeFacade`) and
  `gitnexus_detect_changes()`, and refresh the index if needed.

## 1.4 Relation to other changes

- With `add-pluggable-context-engine-foundation`: `ContextFacade` adds a **Composer** stage before
  the engine; `ContextRuntimeFacade` remains the engine-only wrapper.
- With `complete-context-engine-runtime-phases`: **additive** — the report gains a `composer`
  summary field without changing engine behavior.
