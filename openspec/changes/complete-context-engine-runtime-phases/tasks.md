# Tasks

## 1. Reconcile Contract vs Runtime Status

- [x] 1.1 Update `add-context-engine-policy-phases/tasks.md` or add an audit note so completed contract tasks are not mistaken for runtime Phase completion.
- [x] 1.2 Create a Phase status matrix covering Contract, Runtime, Diagnostics, and Verification for Phase 0-5.
- [x] 1.3 Re-run GitNexus impact analysis for symbols touched by runtime integration before implementation.

## 2. Runtime Context Facade and Engine Selection

- [x] 2.1 Define a runtime-facing context facade used by framework and runtime model calls.
- [x] 2.2 Add context engine config schema with system default, application override, and agent/profile override.
- [x] 2.3 Register built-in `legacy`, `windowed`, `pruning`, and `summary` engines.
- [x] 2.4 Implement fallback policy and `context_engine_fallback` report/event emission.
- [x] 2.5 Add tests proving engine selection is config-driven and does not branch on app/workflow/agent names.

## 3. Phase 0 Runtime Observability

- [x] 3.1 Persist `ContextReport` for framework ReAct model calls through the runtime context facade.
- [x] 3.2 Persist `ContextReport` for `macaca-runtime` direct agentic loop calls, not only debug log.
- [x] 3.3 Audit simple/declarative/SDK LLM call paths and route them through the reportable facade or document non-applicable paths.
- [x] 3.4 Extend context report payload with compaction count, selected engine id, fallback decision, and source render metadata.
- [x] 3.5 Add integration tests proving every supported LLM call path writes a `context_report` EventLog event.

## 4. Phase 1 PromptComposer Runtime Migration

- [x] 4.1 Replace direct system prompt string assembly in `framework_runner.rs` with typed `PromptSection` construction behind a compatibility adapter.
- [x] 4.2 Model persona files, application semantics, capabilities, workspace paths, workspace guide files, skill index, tool schema, and runtime metadata as typed sources.
- [x] 4.3 Add OpenClaw-style workspace guide source provider for `AGENTS.md`, `SOUL.md`, `TOOLS.md`, `IDENTITY.md`, `USER.md`, `HEARTBEAT.md`, with configurable priorities and budgets.
- [x] 4.4 Ensure stable sections and dynamic sections render through explicit boundary and deterministic sorting.
- [x] 4.5 Add snapshot/hash tests proving dynamic metadata does not change stable prompt hash.
- [x] 4.6 Mark replaced direct prompt constructors as deprecated and keep them searchable.

## 5. Phase 2 Pruning Runtime Behavior

- [x] 5.1 Wire selected `pruning` engine into framework/runtime LLM payload assembly.
- [x] 5.2 Ensure large tool result, trace event, command stdout, file read, and search result sources render as summary/excerpt/source ref in model context.
- [ ] 5.3 Preserve original source payload in EventLog/session/artifact store.
- [x] 5.4 Show pruning decisions, pruned tokens, render mode, trust level, and source ref in context report API/UI.
- [x] 5.5 Add integration tests proving large stdout/file read is not fully sent to LLM while original payload remains retrievable.

## 6. Phase 3 Compaction and Lineage Runtime Flow

- [x] 6.1 Implement `summary` engine or compaction-capable policy flow on top of window/pruning behavior.
- [x] 6.2 Add automatic compaction trigger based on context budget threshold.
- [x] 6.3 Add manual compact API with optional focus topic.
- [x] 6.4 Invoke `before_compaction` and `after_compaction` hooks around summary generation.
- [x] 6.5 Persist compaction summary, successor transcript segment/session, and lineage tip.
- [x] 6.6 Make resume/logical session queries resolve to lineage tip by default, with debug root-to-tip expansion.
- [x] 6.7 Emit `context_compaction` and `context_lineage_updated` events and show them in UI.
- [x] 6.8 Add tests proving compacted sessions continue, original history remains readable, and summary is reference-only/untrusted.

## 7. Phase 4 Memory Recall and Wiki Runtime Flow

- [ ] 7.1 Implement read-only `memory_search`, `memory_get`, and wiki/digest recall entry points through context source providers _(memory tools + optional preflight wired when `context.recall.expose_memory_tools`; wiki/digest recall tool still pending)_.
- [x] 7.2 Add recall policy with max tokens/chars, timeout, provenance, confidence, privacy tier, and source budget.
- [x] 7.3 Add opt-in preflight recall runtime step with read-only tool allowlist and safe degradation.
- [ ] 7.4 Inject recall output as dynamic/untrusted/request-only context and never write it back to canonical transcript _(wire-only injection with explicit untrusted prefix; persistence boundary still framework-owned)_.
- [x] 7.5 Surface memory/wiki recall sources and warnings in `ContextReport`.
- [x] 7.6 Add tests proving memory is not globally loaded by default and recall is visible only when explicitly enabled or invoked.

## 8. Phase 5 Pluggable Engine Runtime Completion

- [x] 8.1 Implement `WindowedContextEngine` using provider-neutral budget/window policies.
- [x] 8.2 Implement `SummaryContextEngine` using compaction summary and lineage.
- [x] 8.3 Support system/app/agent profile engine selection without code changes in application logic.
- [x] 8.4 Add custom in-process engine/provider conformance tests and registration examples.
- [x] 8.5 Emit fallback diagnostics when selected engines fail and verify fallback does not crash the main loop.
- [x] 8.6 Add tests proving engine switching changes strategy without touching application code.

## 9. Verification

- [x] 9.1 Run `openspec validate complete-context-engine-runtime-phases --strict`.
- [x] 9.2 Run targeted Rust tests for `macaca-context`, `macaca-runtime`, `macaca-framework`, `macaca-web`, `macaca-memory`, and `macaca-persist`.
- [x] 9.3 Run frontend lint/build or targeted UI tests for context report and lineage views.
- [ ] 9.4 Run GitNexus impact and change detection using available CLI/MCP workflow.
- [ ] 9.5 Update this checklist only after runtime behavior and verification evidence are complete.
