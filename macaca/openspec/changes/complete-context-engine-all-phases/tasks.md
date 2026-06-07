# Tasks

## 1. Baseline And Closure Audit

- [x] 1.1 Re-read `docs/context-engineering-openclaw-hermes-research.md`, `docs/superpowers/plans/2026-05-06-complete-context-engine-unfinished-phases-plan.md`, and map original `Phase 0-5` plus closure `Phase 6-10` to current code, OpenSpec changes, and remaining gaps.
- [x] 1.2 Update `openspec/changes/complete-context-engine-all-phases/phase-status.md` so `Phase 0/1/5` reflect complete reality, `Phase 2/3/4` point to `Phase 6/7/8` closure work, and `Phase 9/10` show explicit remaining work.
- [x] 1.3 Record which existing changes are prerequisites, partially superseded, or blocked from archive by this change.
- [x] 1.4 Re-run GitNexus impact analysis before editing symbols touched by `Phase 6-10` implementation and report any HIGH/CRITICAL risk before edits.
- [x] 1.5 Confirm design pattern mapping is reflected in implementation notes and code review checklists: Facade, Adapter, Strategy, Decorator, Repository, Chain of Responsibility, Memento, Ports and Adapters.
- [x] 1.6 Confirm no implementation task introduces hardcoded application names, workflow names, driver names, provider names, or business-specific identifiers; selection must come from config, manifest, profile, or registered capabilities.

## 2. Finalize Phase 6: Non-Destructive Pruning

- [x] 2.1 Audit every pruned source kind: tool result, trace event, command stdout/stderr, file read, search result, skill/capability large payload.
- [x] 2.2 Ensure each pruned source keeps its original payload in canonical EventLog, session store, or artifact store.
- [x] 2.3 Ensure every `ContextReport` source row includes a stable source ref or artifact ref for authorized retrieval.
- [x] 2.4 Define or consolidate a `ContextSourceArtifactRepository` / retrieval adapter boundary that resolves refs through scoped repository APIs instead of UI or context code reading raw storage keys.
- [x] 2.4a Implement retrieval as Repository + Adapter: each source kind maps canonical EventLog/session/artifact storage into a stable source ref without exposing backend keys to UI code.
- [x] 2.5 Add backend debug/API retrieval path for pruned originals where source refs exist but no fetch path exists yet, including cross-session/app rejection.
- [x] 2.6 Extend UI diagnostics so operators can follow a pruned source reference to bounded preview, full debug path when authorized, or explicit unavailable reason.
- [x] 2.7 Add tests proving pruning never mutates canonical source data and original payload remains retrievable for every supported source kind.
- [x] 2.8 Add tests proving unavailable source refs are reported explicitly instead of silently pretending retrieval works.

## 3. Finalize Phase 7: Compaction And Session Lineage

- [x] 3.1 Confirm automatic compaction runs through selected `summary`/compaction-capable engine on budget pressure, not only manual API.
- [x] 3.2 Ensure compaction hooks run before and after summary generation for memory/source providers.
- [x] 3.3 Ensure logical session reads resolve to lineage tip by default across all supported session-loading paths.
- [x] 3.4 Add front-end lineage view or equivalent interaction showing root id, tip id, successor chain, and compaction summaries.
- [x] 3.4a Keep lineage presentation behind a Facade/Strategy boundary so default UI shows one logical session and debug UI can expand root-to-tip details without changing application behavior.
- [x] 3.5 Add debug root-to-tip expansion in UI/API without changing default logical-session UX.
- [x] 3.6 Ensure session list/chat trace default UX still shows one logical session rather than duplicate compaction successors.
- [x] 3.7 Add tests proving compacted sessions resume correctly, original history remains readable, and summary stays reference-only/untrusted.
- [x] 3.8 Add UI/API tests or snapshots for lineage panel/root-to-tip diagnostics.

## 4. Finalize Phase 8: Memory Recall And Wiki/Digest Runtime Flow

- [x] 4.1 Implement runtime wiki/digest recall entry points through context source providers first; add read-only recall tools only if needed without bypassing context reports.
- [x] 4.2 Ensure memory/wiki recall outputs always carry provenance, confidence, privacy tier, and source id metadata.
- [x] 4.3 Ensure all recall injections are dynamic, untrusted, request-only, and never written back to canonical transcript.
- [x] 4.4 Align preflight recall, active recall, and explicit recall tool paths so diagnostics and safety rules are consistent.
- [x] 4.5 Show memory/wiki recall source breakdown and warnings in context report UI/API with clear trust fencing.
- [x] 4.6 Add tests proving wiki/digest recall is opt-in, bounded, safely degraded on failure, and invisible by default.
- [x] 4.7 Add tests proving recall providers do not receive mutable transcript references and injected recall bodies are not persisted into session transcript or duplicate EventLog message bodies.
- [x] 4.8 Reuse digest-vs-raw selection or equivalent strategy so wiki/digest and raw memory do not duplicate context without an explainable report decision.
- [x] 4.9 Keep recall safety as Decorators around providers: tombstone filtering, redaction, privacy filtering, trust fencing, timeout, and bounded rendering must not be hardcoded into one provider implementation.

## 5. Finalize Phase 9: User Plugin And External Adapter Path

- [x] 5.1 Publish or finalize conformance tests for custom in-process `ContextEngine` and provider implementations.
- [x] 5.2 Add runtime boot registration path for custom in-process engines/providers selected by config/profile without code edits in application logic.
- [x] 5.3 Document and implement custom engine selection precedence across system/app/agent profile config.
- [x] 5.4 Add minimal process/RPC/WASM external adapter seam behind explicit adapter boundary; keep transport-specific behavior experimental/off unless explicitly configured.
- [x] 5.4a Use Ports and Adapters / Bridge: local custom engines implement the stable in-process port; external process/RPC/WASM adapters bridge into that port through validation and fallback.
- [x] 5.5 Enforce adapter safety controls: timeout, max payload, schema validation, trust fencing, circuit breaker, and fallback before external output reaches model context.
- [x] 5.6 Add anti-corruption tests proving malformed, oversized, slow, or untrusted external output degrades without crashing the main loop.
- [x] 5.7 Add tests proving custom engines can be installed and selected by config without application code changes.

## 6. Finalize Phase 10: Migration And Deprecation Discipline

- [x] 6.1 Inventory all legacy prompt/context entry points replaced by facade/composer/runtime selection.
- [x] 6.2 Mark remaining compatibility entry points as deprecated with explicit replacement guidance, without deleting them.
- [x] 6.3 Migrate all internal production call sites away from deprecated prompt/context APIs in small slices.
- [x] 6.4 Run `rg` checks confirming no non-test production path still uses deprecated prompt/context entry points.
- [x] 6.5 Update task files and design notes so “complete” only means runtime + diagnostics + verification are all done.
- [x] 6.6 Add or update audit tests/scripts that keep production code from reintroducing deprecated prompt/context entry points.
- [x] 6.7 Prepare archive order for superseded context-engine changes, but do not archive until specs, runtime, diagnostics, tests, and GitNexus evidence are aligned.
- [x] 6.8 Add design-pattern regression checks to migration review: production context assembly must enter through facade/composer/runtime selection, while legacy APIs remain compatibility adapters only.

## 7. Verification And Archive Readiness

- [x] 7.1 Run `openspec validate complete-context-engine-all-phases --strict`.
- [x] 7.2 Re-run targeted Rust tests across `macaca-context`, `macaca-runtime`, `macaca-framework`, `macaca-web`, `macaca-memory`, `macaca-persist`, and integration tests covering context phases.
- [x] 7.3 Run frontend lint/build or targeted UI tests for context report, pruned source retrieval, and lineage views.
- [x] 7.4 Run GitNexus impact and change detection with the best available CLI/MCP workflow and record any limitations.
- [x] 7.5 Update Phase status artifacts to reflect final `Phase 0-10` completion evidence.
- [x] 7.6 Archive or prepare archive order for superseded context-engine changes only after code, tests, and specs align.
- [x] 7.7 Run final scans for app/workflow/driver/provider-specific hardcoding in new context-engine closure code.
