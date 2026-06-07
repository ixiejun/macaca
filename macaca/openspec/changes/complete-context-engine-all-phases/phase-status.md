# Context Engineering Phase Status

This file is the status artifact required by `complete-context-engine-all-phases`.
It tracks runtime reality for `Phase 0-10` using four evidence categories:
Contract, Runtime, Diagnostics, and Verification.

## Status Matrix

| Phase | Contract | Runtime | Diagnostics | Verification | Overall | Closure Owner |
|-------|----------|---------|-------------|--------------|---------|---------------|
| Phase 0 Observability | Complete | Complete | Complete | Complete | Complete | Existing runtime phases |
| Phase 1 PromptComposer | Complete | Complete | Complete | Complete | Complete | Existing runtime phases |
| Phase 2 Non-Destructive Pruning Foundation | Complete | Complete | Complete | Complete | Complete | Phase 6 |
| Phase 3 Compaction And Lineage Foundation | Complete | Complete | Complete | Complete | Complete | Phase 7 |
| Phase 4 Memory Recall Runtime Foundation | Complete | Complete | Complete | Complete | Complete | Phase 8 |
| Phase 5 Pluggable Engine Foundation | Complete | Complete | Complete | Complete | Complete | Existing runtime phases |
| Phase 6 Pruned Source Retrievability Closure | Complete | Complete | Complete | Complete | Complete | This change |
| Phase 7 Lineage UX Closure | Complete | Complete | Complete | Complete | Complete | This change |
| Phase 8 Memory And Wiki Recall Closure | Complete | Complete | Complete | Complete | Complete | This change |
| Phase 9 User Plugin And External Adapter Path | Complete | Complete | Complete | Complete | Complete | This change |
| Phase 10 Migration And Archive Discipline | Complete | Complete | Complete | Complete | Complete | This change |

## Evidence Definitions

- **Contract**: traits, value objects, config, policies, and OpenSpec requirements exist.
- **Runtime**: framework/runtime/web model-call paths actually exercise the feature.
- **Diagnostics**: EventLog, API, UI, context report, or trace surfaces expose the feature for operators.
- **Verification**: targeted tests, integration tests, OpenSpec validation, GitNexus evidence, and audit scans prove behavior.

## Closure Rules

- A phase MUST remain `Partial` while any one of Contract, Runtime, Diagnostics, or Verification is incomplete.
- Phase 2 foundation gaps MUST close through Phase 6. The completion signal is retrievable canonical payloads for every supported pruned source kind.
- Phase 3 foundation gaps MUST close through Phase 7. The completion signal is logical-session UX plus root-to-tip lineage inspection.
- Phase 4 foundation gaps MUST close through Phase 8. The completion signal is memory/wiki/digest recall entering the same bounded, dynamic, untrusted, request-only context path.
- Phase 9 completion requires config-selected custom engines/providers and safe external adapter degradation.
- Phase 10 completion requires deprecated legacy prompt/context entry points to remain searchable while production callers migrate away.

## Design Pattern Guardrails

- **Facade**: production callers MUST enter through context facade/runtime facade rather than constructing prompt strings directly.
- **Adapter / Repository**: source artifact retrieval MUST resolve canonical payloads through scoped retrieval adapters/repositories.
- **Strategy**: pruning, recall, lineage display, provider selection, and fallback policies MUST remain replaceable.
- **Decorator**: redaction, tombstone, trust fencing, timeout, schema validation, and circuit breaker behavior MUST wrap untrusted/external inputs.
- **Chain of Responsibility**: memory, wiki/digest, skills, MCP, profile, and tool schema sources MUST enter composer through provider stages.
- **Memento**: compaction summaries and successor lineage nodes MUST be stored as audit snapshots rather than destructive transcript rewrites.
- **Ports and Adapters / Bridge**: custom engines/providers and external adapters MUST implement Macaca-owned ports without application-specific logic.

## Completion Evidence

- **Phase 6**: `ContextSourceArtifactRepository` resolves scoped EventLog refs, rejects cross-session refs, returns bounded previews, and now the framework hot path persists pruned `message/{idx}` originals as `context_source_artifact` rows before writing `context_report`.
- **Phase 7**: `SessionLineageStore`, `/api/sessions/{id}/lineage`, `SessionLineagePanel`, and compaction summary diagnostics expose root, tip, successor chain, and reference-only summaries without duplicating the default logical-session UX.
- **Phase 8**: `KnowledgeDigestContextProvider`, `WorkspaceKnowledgeDigestCapability`, active/preflight recall adapters, digest-vs-raw selection, tombstone filtering, redaction, privacy metadata, and request-only diagnostics are wired through the composer/provider chain.
- **Phase 9**: `ContextEngineRegistry`, config/profile-driven overlay selection, `ExternalAdapterContextEngine`, opaque payload validation, adapter timeout/fallback, and runtime registry diagnostics provide the custom engine/provider and external adapter path.
- **Phase 10**: Deprecated compatibility shims remain searchable, production model assembly enters through `ContextFacade`/provider/runtime selection, migration guard tests and `rg` scans cover legacy prompt/context entry points, and superseded changes remain unarchived until this validated closure is accepted.

## Verification Evidence

- `openspec validate complete-context-engine-all-phases --strict`
- `cargo fmt --check`
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
- GitNexus impact for `assemble_and_emit_report` and `ContextReportingChatModel` returned LOW risk; new unindexed helper symbols were verified by targeted tests and will be covered by the next GitNexus index rebuild.
- Hardcoding scan over context closure code found no application/workflow/driver/provider-specific identifiers introduced by this change.
