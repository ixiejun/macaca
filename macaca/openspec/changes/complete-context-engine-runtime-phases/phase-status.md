# Context Engineering Phase Status

This file tracks real implementation status for the original `Phase 0-10` context-engineering plan.

| Phase | Contract | Runtime | Diagnostics | Verification | Overall |
|-------|----------|---------|-------------|--------------|---------|
| Phase 0 Observability | Complete | Complete | Complete | Complete | Complete |
| Phase 1 PromptComposer | Complete | Complete | Complete | Complete | Complete |
| Phase 2 Non-Destructive Pruning | Complete | Complete | Complete | Complete | Complete |
| Phase 3 Compaction And Lineage Foundation | Complete | Complete | Complete | Complete | Complete |
| Phase 4 Memory Recall Runtime | Complete | Complete | Complete | Complete | Complete |
| Phase 5 Pluggable Engine Foundation | Complete | Complete | Complete | Complete | Complete |
| Phase 6 Pruned Source Retrievability Closure | Complete | Complete | Complete | Complete | Complete |
| Phase 7 Lineage UX Closure | Complete | Complete | Complete | Complete | Complete |
| Phase 8 Memory And Wiki Recall Closure | Complete | Complete | Complete | Complete | Complete |
| Phase 9 User Plugin And External Adapter Path | Complete | Complete | Complete | Complete | Complete |
| Phase 10 Migration And Archive Discipline | Complete | Complete | Complete | Complete | Complete |

Definitions:

- Contract: traits, value objects, config, policies, and spec requirements exist.
- Runtime: supported framework/runtime/web model-call paths actually exercise the feature.
- Diagnostics: EventLog, API, UI, and trace surfaces expose the feature for operators.
- Verification: targeted tests and audit artifacts prove the behavior.

What is already fully complete:

- `Phase 0-5` are implemented end to end across `macaca-context`, `macaca-runtime`, `macaca-web`, context report API, and the existing diagnostics UI.
- Built-in `legacy`, `windowed`, `pruning`, and `summary` engines are registered and used by supported runtime paths.
- Framework prompt assembly uses typed sections and workspace guide inputs.
- Memory recall, preflight recall, active recall, compaction hooks, lineage persistence, and logical-session APIs all exist as runtime capabilities.

Final completion evidence:

- `Phase 6`: `ContextSourceArtifactRepository` resolves scoped EventLog refs, rejects cross-session refs, returns bounded previews, and `persist_pruned_source_artifacts` stores pruned model-message originals as `context_source_artifact` EventLog rows before `context_report` is written.
- `Phase 7`: `SessionLineageStore`, the session lineage API, lineage diagnostics UI, and compaction summary diagnostics expose root, tip, successor chain, and reference-only summaries without duplicating the default logical-session UX.
- `Phase 8`: `memory_search` / `memory_get` are read-only runtime-backed tools; `KnowledgeDigestContextProvider` and `WorkspaceKnowledgeDigestCapability` wire wiki/digest recall through the context provider chain as bounded, dynamic, untrusted, request-only context with provenance and privacy metadata.
- `Phase 9`: `ContextEngineRegistry`, config/profile-driven engine overlays, external adapter validation, timeout/fallback, and conformance/failure tests provide the custom engine/provider path without application code changes.
- `Phase 10`: deprecated compatibility shims remain searchable; production context assembly enters through facade/composer/runtime selection; audit tests and `rg` scans cover legacy prompt/context entry points; archive remains gated on OpenSpec, tests, and GitNexus evidence.

Verification evidence:

- `openspec validate complete-context-engine-runtime-phases --strict`
- `openspec validate complete-context-engine-all-phases --strict`
- `cargo test -p macaca-context --lib`
- `cargo test -p macaca-persist --lib`
- `cargo test -p macaca-memory --lib`
- `cargo test -p macaca-runtime --lib`
- `cargo test -p macaca-framework --lib`
- `cargo test -p macaca-web --lib`
- `cargo test -p macaca-web source_artifact --lib`
- `cargo test -p macaca-integration-tests --test memory_runtime_e2e`
- `npm run lint`
- `npm run build`
- GitNexus impact for `assemble_and_emit_report` and `ContextReportingChatModel` returned LOW risk; `detect_changes` is required before commit or archive.
