# Macaca Context Engineering Brainstorm And Plan

Date: 2026-05-05

## Inputs

- Research report: `docs/context-engineering-openclaw-hermes-research.md`
- OpenSpec guide: `openspec/AGENTS.md`
- Design pattern guide: `macaca/docs/design_patterns.md`
- Current code touchpoints observed in this planning pass:
  - `macaca/crates/macaca-web/src/framework_runner.rs` builds agent system prompts, injects capabilities, workspace paths, and skill snapshots.
  - `macaca/crates/macaca-framework/src/react_agent.rs` builds per-iteration model context from system prompt plus working memory.
  - `macaca/crates/macaca-runtime/src/agentic_loop.rs` trims context with `ContextWindowManager` before direct LLM calls.
  - `macaca/crates/macaca-memory/` already owns memory abstractions and should remain separate from context assembly.
  - `macaca/crates/macaca-skill/` already supports skill snapshots and progressive disclosure.
  - `macaca/crates/macaca-llm/` should remain provider-facing and not absorb Macaca-specific context policy.

## Constraints

- Macaca is an Agent OS infrastructure layer for 7x24 autonomous operation.
- Context engineering must be generic across applications, agents, sessions, tools, runtimes, and LLM providers.
- No workflow name, application name, driver name, or business-specific concept may be hardcoded into core logic.
- Existing behavior should be wrapped first, not replaced directly.
- The architecture must be pluggable: users must be able to replace Macaca's default context management system with their own implementation.
- Every non-trivial module should first ask whether a design pattern improves clarity, extensibility, performance, or testability.
- Avoid over-design: interfaces should be narrow and proven by the first default implementation.

## Superpowers Brainstorm

### Design Principles

- Depend on traits and domain contracts, not concrete context engines.
- Put context policy behind Strategy interfaces so application code selects behavior by configuration, not by branches.
- Use Factory or Abstract Factory only at composition roots, not inside agent loops.
- Use Builder and Composite for prompt assembly, because prompt construction is multi-step and source-ordered.
- Use Adapter and Bridge for third-party context management systems.
- Use Facade for upper layers, so `macaca-web`, `macaca-runtime`, and applications do not know the internal engine graph.
- Use Observer or audit-log style reporting for diagnostics rather than exposing full prompt internals everywhere.
- Use Policy objects for pruning, compaction, memory recall, and source rendering decisions.
- Keep `memory`, `skill`, `tool schema`, `trace event`, and `session history` as context sources; do not merge them into a single untyped blob.
- Preserve stable/dynamic prompt boundaries so dynamic recall, time, trace, and runtime state do not break prompt cache stability.

### Option A: Observability-First Foundation

Scope:

- Introduce `ContextReport` and source accounting around existing prompt and history behavior.
- Do not change what is sent to models.
- Capture prompt/source size estimates, stable/dynamic candidates, tool schema size, skill snapshot size, memory size, and history size.

Design patterns:

- Observer: collect request-time context diagnostics without owning context construction.
- Facade: expose `ContextReportService` to Web/API consumers.
- Adapter: wrap existing prompt builders and context-window trimming as reportable sources.

Benefits:

- Lowest behavior risk.
- Gives evidence before pruning or compaction changes.
- Makes later performance work measurable.
- Creates user-facing diagnostics without coupling users to an engine implementation.

Risks:

- Does not immediately reduce token use.
- Token estimates may be approximate unless provider-specific tokenizers are introduced.
- If report data is too verbose, it can become a privacy/sensitivity risk.

Controls:

- Store summaries, source IDs, sizes, and hashes by default; store full prompt only behind explicit debug configuration.
- Treat tokenizer-specific accuracy as a later enhancement.
- Keep report schema provider-neutral.

### Option B: Pluggable ContextEngine Foundation

Scope:

- Define a narrow `ContextEngine` trait and `LegacyContextEngine` default implementation.
- Introduce an engine registry and config-driven selection.
- Route model-call assembly through a facade while preserving existing behavior.

Design patterns:

- Strategy: each engine is a replaceable context assembly strategy.
- Abstract Factory: create engine families with their dependencies from runtime/application profile config.
- Registry: map configured engine IDs to providers.
- Adapter: wrap existing `framework_runner`, `ReActAgent`, and runtime loop context behavior.
- Template Method: shared lifecycle steps such as bootstrap, assemble, after-turn, and compact are explicit but overridable.

Benefits:

- Establishes the correct plugin boundary early.
- Enables user-provided context systems without Web/runtime knowing their internals.
- Allows multiple engines such as legacy, windowed, summary, memory-aware, or remote.
- Keeps future pruning/compaction/memory work as extensions, not invasive rewrites.

Risks:

- Trait can become too wide if copied directly from OpenClaw.
- Async lifecycle hooks can create ordering and failure-mode complexity.
- Too much registry/config work before a second engine exists can be over-design.

Controls:

- Start with `assemble`, `report`, and `after_turn`; add compaction and child lifecycle only when implementing those slices.
- Ship only `LegacyContextEngine` first.
- Keep engine errors explicit and observable, with fallback rules in config.

### Option C: PromptComposer Stable/Dynamic Split First

Scope:

- Extract prompt construction into typed sections with deterministic ordering.
- Separate stable system prefix from dynamic per-request suffix.
- Introduce prompt hashes and cache-boundary diagnostics.

Design patterns:

- Builder: construct `CompiledPrompt` from ordered sections.
- Composite: sections can contain source groups such as workspace, skills, capabilities, memory, and runtime metadata.
- Value Object: `PromptSection`, `PromptStability`, `TrustLevel`, `ContextSourceId`.

Benefits:

- Directly improves prompt clarity and prompt cache friendliness.
- Makes context sources explicit without requiring full engine lifecycle.
- Reduces string-concatenation sprawl in Web/framework code.

Risks:

- If done without an engine facade, upper layers may still couple to composer internals.
- Stable/dynamic classification mistakes can hide dynamic state in the cacheable prefix.
- Tests must lock deterministic rendering order.

Controls:

- Implement under the `ContextEngine` facade or as its dependency, not as a Web-only utility.
- Make section stability mandatory and fail closed for unknown dynamic sources.
- Add snapshot tests for ordering and stable hash behavior.

### Option D: Pruning And Compaction First

Scope:

- Immediately reduce context by trimming tool outputs, trace events, old history, and large artifacts.
- Add compaction summaries when sessions approach context limits.

Design patterns:

- Chain of Responsibility: source-specific renderers or pruners process context sources in order.
- Strategy/Policy: `PruningPolicy`, `CompactionPolicy`, and `BudgetPolicy` are replaceable.
- Memento/Event Sourcing: preserve original transcript/event store while deriving compacted context views.

Benefits:

- Fastest route to token and latency improvements.
- Solves large tool output and long-session pressure.
- Forces explicit source rendering boundaries.

Risks:

- Highest semantic regression risk if done before observability.
- Bad summaries can cause models to repeat old tasks or lose active state.
- Compaction can accidentally mutate or obscure audit history if not modeled carefully.

Controls:

- Do not start here as the first slice.
- Pruning must be non-destructive and reportable.
- Compaction summaries must use a strict envelope and preserve IDs, paths, decisions, active task, and open questions.
- Original events and transcripts must remain retrievable.

### Option E: External Context Manager Protocol First

Scope:

- Define a provider protocol so users can plug in local or remote context systems.
- Macaca only sends bootstrap/ingest/assemble/after-turn requests and receives compiled context plus diagnostics.

Design patterns:

- Ports and Adapters: Macaca owns the port; user systems implement adapters.
- Bridge: separate Macaca runtime abstraction from concrete context backends.
- Abstract Factory: instantiate local, process, RPC, or WASM providers from config.
- Anti-Corruption Layer: normalize external context output into Macaca `CompiledPrompt` and `ContextReport`.

Benefits:

- Strongest answer to the pluggability requirement.
- Avoids locking users into Macaca's internal memory/context stack.
- Enables specialized enterprise context stores, private knowledge systems, or agent-specific context engines.

Risks:

- Too early remote protocol design can freeze the wrong contract.
- Security, timeout, data leakage, and prompt-injection boundaries become larger.
- External engines can return malformed, unsafe, oversized, or non-deterministic context.

Controls:

- First define the in-process Rust trait as the source of truth.
- Add external adapters only after the local default contract is proven.
- Enforce size budgets, trust boundaries, timeouts, schema validation, and fallback behavior at the Macaca boundary.

### Option F: Memory-Centric Context System

Scope:

- Make memory recall, wiki/digest, and durable knowledge the center of context engineering.
- Build recall hooks and memory provider lifecycle before prompt/source architecture.

Design patterns:

- Strategy: memory recall providers.
- Repository: memory stores and wiki stores.
- Observer: lifecycle hooks such as before-compaction and after-turn sync.

Benefits:

- Useful for long-running agent workloads.
- Leverages existing `macaca-memory` concepts.
- Aligns with OpenClaw memory-core and memory-wiki lessons.

Risks:

- Wrong abstraction boundary: memory is one context source, not the whole context engine.
- Can accidentally couple context to a single memory backend.
- Recall injection can pollute prompts and create security issues.

Controls:

- Keep memory as a source provider behind `ContextSourceProvider`.
- Make recall opt-in and budget-limited.
- Mark recall content untrusted and keep it in dynamic request-only context.

## Recommendation

Use a staged combination of Option A, Option B, and Option C as the foundation, then add Option D extension points, Option E external adapters, and Option F memory integration incrementally.

Recommended sequence:

1. Observability-first `ContextReport` around existing behavior.
2. Narrow `ContextEngine` trait plus `LegacyContextEngine`.
3. `PromptComposer` stable/dynamic section model.
4. Non-destructive pruning policies for tool outputs, trace events, and large history.
5. Compaction with session lineage and strict summary envelopes.
6. Memory recall and wiki/digest as optional context sources.
7. External context manager adapters after the in-process contract stabilizes.

Rationale:

- This path is gradual and reversible.
- It meets the pluggability requirement without freezing a premature remote protocol.
- It avoids strong coupling between Macaca Agent OS and any one context management implementation.
- It keeps existing behavior as the default fallback.
- It gives users replacement points at the engine, source-provider, policy, and external-adapter levels.

## Proposed Architecture

### Core Boundary

Macaca core should own the context contract, not the default implementation:

- `ContextEngine`: strategy for request-time assembly and lifecycle.
- `ContextEngineProvider`: factory/provider registered under an engine ID.
- `ContextEngineRegistry`: selects providers by config, application profile, or agent profile.
- `ContextManagerFacade`: stable API used by runtime/framework/web layers.
- `PromptComposer`: builds stable/dynamic prompt sections into a compiled provider request.
- `ContextSourceProvider`: contributes bounded source candidates such as skills, memory, workspace, trace, history, or tool schema.
- `ContextRenderable`: source-specific renderer into a model-safe snippet.
- `ContextReport`: request-scoped accounting and audit summary.
- `ContextPolicySet`: pruning, compaction, recall, and budget policies.

The core rule is:

- Upper layers call the facade.
- The facade calls the selected engine.
- The engine calls source providers and policies.
- External systems implement adapters.
- No application or Web code depends on concrete engine internals.

### Design Pattern Map

- `ContextEngine`: Strategy.
- `LegacyContextEngine`, `WindowedContextEngine`, `SummaryContextEngine`, user engines: Concrete Strategies.
- `ContextEngineProvider`: Factory Method.
- Engine family creation from config: Abstract Factory.
- Engine lookup: Registry.
- `ContextManagerFacade`: Facade.
- `PromptComposer`: Builder.
- Prompt sections and source groups: Composite.
- `ContextSourceProvider`: Provider pattern plus Strategy.
- `ContextRenderable`: Strategy or Chain of Responsibility for source rendering.
- `PruningPolicy`, `CompactionPolicy`, `MemoryRecallPolicy`, `BudgetPolicy`: Policy/Strategy.
- External user context manager: Adapter plus Bridge.
- Legacy prompt building integration: Adapter.
- Context reports: Observer/Audit Log.
- Compaction lineage: Memento/Event Sourcing style.
- Trust boundary validation: Anti-Corruption Layer.

### Pluggability Contract

Macaca should support replacement at four levels:

- Engine replacement: a user can replace the entire context engine for an application or agent profile.
- Source replacement: a user can provide custom `ContextSourceProvider`s without replacing the engine.
- Policy replacement: a user can customize budget, pruning, compaction, and recall policies.
- External manager replacement: a user can run a separate local or remote context manager behind an adapter.

Selection must be config/manifest driven:

- `context.engine = "legacy"` as the default.
- `context.engine = "windowed"` or `"summary"` for built-ins.
- `context.engine = "custom:<provider-id>"` for user implementations.
- No branch may check application name, workflow name, or agent name to choose special behavior.

### Trust And Safety Boundaries

- System prompt stable sections are trusted only if generated by Macaca or explicit application policy.
- Memory recall, external context, workspace files, trace events, and tool outputs are untrusted background unless explicitly promoted.
- Dynamic injections should be request-only and must not be written back to canonical transcript.
- Context reports should store sizes, source IDs, hashes, and decisions by default, not full sensitive prompt content.
- External context managers must be budget-limited, timeout-limited, and schema-validated before output reaches the LLM provider.

### Initial Trait Shape

The first implementation should avoid copying a large lifecycle surface from OpenClaw. A narrow first contract is enough:

```rust
#[async_trait::async_trait]
pub trait ContextEngine: Send + Sync {
    fn info(&self) -> ContextEngineInfo;

    async fn assemble(
        &self,
        input: ContextAssembleInput,
    ) -> MacacaResult<ContextAssembleResult>;

    async fn after_turn(
        &self,
        input: ContextAfterTurnInput,
    ) -> MacacaResult<()>;
}
```

Later extensions can add:

- `bootstrap` when session import/resume needs explicit handling.
- `ingest_turn` when source stores need independent indexing.
- `compact` when compaction is implemented.
- `prepare_child` and `child_finished` when delegate/fork context lineage is formalized.
- `maintain` only if engines need safe transcript rewrite requests.

### Minimal Data Shapes

```rust
pub struct ContextAssembleInput {
    pub app_id: ApplicationId,
    pub session_id: SessionId,
    pub agent_name: String,
    pub model: String,
    pub base_messages: Vec<LlmMessage>,
    pub tools: Vec<ToolDefinition>,
    pub budget: ContextBudget,
    pub request_metadata: ContextRequestMetadata,
}
```

```rust
pub struct ContextAssembleResult {
    pub messages: Vec<LlmMessage>,
    pub options_patch: ContextOptionsPatch,
    pub report: ContextReport,
}
```

```rust
pub struct ContextReport {
    pub request_id: String,
    pub app_id: ApplicationId,
    pub session_id: SessionId,
    pub agent_name: String,
    pub engine_id: String,
    pub model: String,
    pub estimated_total_tokens: u32,
    pub token_budget: u32,
    pub stable_prompt_tokens: u32,
    pub dynamic_prompt_tokens: u32,
    pub history_tokens: u32,
    pub tool_schema_tokens: u32,
    pub skill_tokens: u32,
    pub memory_tokens: u32,
    pub trace_tokens: u32,
    pub pruned_tokens: u32,
    pub sources: Vec<ContextSourceReport>,
    pub decisions: Vec<ContextDecisionReport>,
}
```

These are planning sketches, not final OpenSpec-approved API.

## Module Placement Options

### Placement 1: Put core abstractions in `macaca-framework`

Benefits:

- `ReActAgent` already assembles prompt, memory, tools, and model calls.
- Close to current framework-level agent construction.
- Natural place for prompt composer and source providers.

Risks:

- `macaca-runtime` direct loop also needs context management.
- Framework may become too large if it owns external provider adapters and Web reports.

### Placement 2: Put core abstractions in `macaca-runtime`

Benefits:

- Runtime loop already has `ContextWindowManager`.
- Good fit for 7x24 execution concerns, budgets, pruning, and loop integration.
- Can serve direct runtime execution and framework agents.

Risks:

- `macaca-framework` prompt semantics and skill integration may need to depend downward into runtime.
- Runtime could become coupled to framework message types if boundaries are not clean.

### Placement 3: Add a focused `macaca-context` crate

Benefits:

- Cleanest boundary for an infrastructure-level context subsystem.
- Avoids bloating runtime, framework, memory, or LLM crates.
- Lets `macaca-framework`, `macaca-runtime`, and `macaca-web` depend on a shared contract.
- Strongest answer to pluggability and long-term architecture.

Risks:

- New crate increases workspace surface.
- Could be overkill if the first slice only adds reports.
- Needs careful dependency direction to avoid cycles.

### Recommendation On Placement

Use a new focused `macaca-context` crate for contracts and default generic components once implementation begins.

Reasoning:

- Context engineering is a cross-cutting Agent OS capability, not just a framework helper.
- `macaca-llm` should remain provider-facing and should not know Macaca source semantics.
- `macaca-memory` should remain memory-specific and should not own prompt assembly.
- `macaca-web` should only read reports through a facade/API.
- A crate boundary makes pluggability clearer and prevents context logic from becoming another large Web/runtime file.

Control against over-design:

- First crate contents should be narrow: types, `ContextEngine`, `LegacyContextEngine`, `ContextReport`, `PromptComposer`, and simple policies.
- External adapters and advanced engines should wait for later OpenSpec changes.

## Risk Register

- Risk: Context engine interface becomes too broad.
  Control: start with assemble/report/after-turn only; add lifecycle methods only when a slice needs them.

- Risk: Concrete default engine leaks into applications.
  Control: upper layers receive `Arc<dyn ContextEngine>` or a facade; selection happens at composition roots.

- Risk: Context becomes coupled to `macaca-web`.
  Control: Web only exposes reports and selects sessions; it does not assemble prompts or implement policies.

- Risk: Context becomes coupled to `macaca-memory`.
  Control: memory is one source provider; memory engines are not context engines by default.

- Risk: Context becomes coupled to `macaca-skill`.
  Control: skill snapshots expose bounded `SkillIndexContext`; full skill bodies remain on-demand resources.

- Risk: Context becomes coupled to a single LLM provider.
  Control: `ContextBudget` and `ContextReport` are provider-neutral; provider-specific token estimation is optional.

- Risk: Prompt cache is broken by dynamic data.
  Control: stable/dynamic section typing is mandatory; dynamic sections never enter stable prompt hash.

- Risk: Pruning or compaction causes task loss.
  Control: observe first; pruning non-destructive; compaction summary strict; original transcript retained.

- Risk: External context manager introduces prompt injection.
  Control: external output is untrusted by default, fenced, source-tagged, budget-limited, and validated.

- Risk: External context manager causes runtime instability.
  Control: timeouts, circuit breaker, fallback engine, and observable degradation events.

- Risk: Users cannot replace the default system cleanly.
  Control: publish stable traits, provider registry, config selection, and conformance tests for custom engines.

- Risk: Implementation creates giant files.
  Control: split by responsibility before coding and keep files under the project 500-line rule.

## Superpowers Write Plan

### Phase 0: Code And Spec Audit

Goal:

- Establish the exact integration points before changing behavior.

Tasks:

- Read current OpenSpec specs and pending changes that mention runtime, framework, web, LLM, memory, skill, session, and trace.
- Use GitNexus impact analysis before editing any symbols in implementation phases.
- Map every LLM request entry point:
  - `macaca-framework` `ReActAgent::reasoning`.
  - `macaca-runtime` `AgenticLoop::run_iteration`.
  - `macaca-agent` and `macaca-sdk` simple/declarative agent calls.
  - Web factories that construct agents and prompts.
- Map every prompt source:
  - persona/base prompt.
  - application prompt semantics.
  - capabilities.
  - workspace paths.
  - skill snapshot prompt.
  - memory/working memory.
  - tool definitions/schema.
  - trace/tool results and session history.
- Identify which sources are stable, dynamic, trusted, untrusted, persisted, and request-only.

Deliverables:

- Source inventory in the OpenSpec design document.
- Blast-radius notes for all symbols selected for edits.

### Phase 1: OpenSpec Proposal

Goal:

- Create one foundation proposal before implementation.

Recommended change ID:

- `add-pluggable-context-engine-foundation`

Proposal scope:

- Add a pluggable context engineering foundation.
- Add request-time `ContextReport`.
- Add `PromptComposer` stable/dynamic sections.
- Add default `LegacyContextEngine` that preserves current behavior.
- Add engine selection contract and fallback behavior.

Out of scope for the first proposal:

- LLM-based compaction.
- External remote protocol.
- Active memory recall sub-agent.
- Full UI redesign.
- Deleting or replacing existing memory/session/event storage.

Required OpenSpec files:

- `openspec/changes/add-pluggable-context-engine-foundation/proposal.md`
- `openspec/changes/add-pluggable-context-engine-foundation/design.md`
- `openspec/changes/add-pluggable-context-engine-foundation/tasks.md`
- `openspec/changes/add-pluggable-context-engine-foundation/specs/context-engine/spec.md`
- Optional deltas for `runtime`, `framework-agent`, or `web-context-report` if existing specs already own those capabilities.

Validation:

- Run `openspec list`.
- Run `openspec list --specs`.
- Run `openspec validate add-pluggable-context-engine-foundation --strict`.

### Phase 2: Create Context Contract Boundary

Goal:

- Add the minimum reusable contract without changing runtime behavior.

Preferred implementation:

- Add `macaca/crates/macaca-context/` if OpenSpec approves the new crate.
- Export core value objects and traits.
- Keep dependencies low: `macaca-proto`, `serde`, `async-trait`, and existing error/result types only if already standard in the workspace.

Components:

- `ContextEngine`
- `ContextEngineInfo`
- `ContextAssembleInput`
- `ContextAssembleResult`
- `ContextAfterTurnInput`
- `ContextBudget`
- `ContextReport`
- `ContextSourceReport`
- `ContextDecisionReport`
- `PromptSection`
- `PromptComposer`
- `LegacyContextEngine`

Design checks:

- File split by responsibility before coding.
- No file over 500 lines.
- No app/workflow/agent special cases.
- All extensibility through traits or policy objects.

### Phase 3: Wrap Existing Prompt Assembly With Legacy Engine

Goal:

- Preserve behavior while moving call sites toward the new facade.

Tasks:

- Adapt existing framework prompt construction behind a legacy source provider or adapter.
- Adapt `ReActAgent::reasoning` so model request assembly can be observed without changing messages.
- Adapt `AgenticLoop::run_iteration` around the current `ContextWindowManager` behavior.
- Keep direct simple/declarative agents working with legacy behavior.
- Produce `ContextReport` for each model call.

Design patterns:

- Adapter wraps existing prompt/string builders.
- Facade hides engine internals from call sites.
- Strategy enables later engine replacement.

Validation:

- Existing tests pass.
- Snapshot or unit tests prove legacy messages are unchanged.
- Reports are generated but do not alter LLM payloads.

### Phase 4: PromptComposer Stable/Dynamic Sections

Goal:

- Replace ad hoc prompt string concatenation with typed sections.

Tasks:

- Model sections with `id`, `stability`, `trust_level`, `source_kind`, and `content`.
- Render stable sections first and dynamic sections after an explicit boundary.
- Sort tool, skill, capability, workspace, and agent-derived sections deterministically.
- Compute stable hash and total hash.
- Keep dynamic request-only injections out of persisted transcript.

Design patterns:

- Builder constructs prompt in ordered steps.
- Composite groups related sections.
- Value Object makes stability and trust explicit.

Validation:

- Stable hash stays unchanged when only dynamic request metadata changes.
- Deterministic render tests cover map/list ordering.
- Existing prompt content remains equivalent unless OpenSpec explicitly approves changes.

### Phase 5: Context Report API And UI Surface

Goal:

- Make context budget and source decisions visible without leaking full prompts.

Tasks:

- Persist request-level report summary tied to app/session/agent/request/turn.
- Add backend read API for context reports.
- Add trace UI entry points to inspect context report summaries per model call.
- Display source breakdown, estimated tokens, pruned tokens, engine ID, prompt hashes, and warnings.

Design patterns:

- Observer/Audit Log records decisions.
- Facade/API avoids UI depending on engine internals.

Validation:

- A session can answer "what entered this model call and why?"
- Sensitive full prompt is not exposed unless debug mode is enabled.

### Phase 6: Non-Destructive Pruning

Goal:

- Reduce oversized tool/trace/history context while preserving original data.

Tasks:

- Add `ContextRenderable` for tool results, trace events, file reads, command outputs, search results, and skill indexes.
- Add `PruningPolicy` and `BudgetPolicy`.
- Render bounded excerpts plus artifact/event references.
- Keep originals in canonical stores.
- Report each pruning decision.

Design patterns:

- Chain of Responsibility for source renderers.
- Policy/Strategy for pruning decisions.
- Memento/Event Sourcing to keep canonical source data intact.

Validation:

- Large outputs no longer enter model context in full.
- ContextReport explains every dropped or summarized source.
- UI can still fetch original events/artifacts.

### Phase 7: Compaction And Session Lineage

Goal:

- Support long-running sessions without losing auditability.

Tasks:

- Add strict compaction summary envelope.
- Add `CompactionPolicy`.
- Trigger memory flush hooks before compaction.
- Represent successor transcript segments or child sessions with lineage metadata.
- Present one logical session in UI while retaining internal lineage.

Design patterns:

- Strategy for compaction engines.
- Memento/Event Sourcing for lineage and audit history.
- Template Method for compaction flow.

Validation:

- Compacted sessions continue correctly.
- Original history remains accessible.
- Summary cannot be mistaken for a new user instruction.

### Phase 8: Memory Recall And Wiki/Digest Sources

Goal:

- Add long-term knowledge as optional, budgeted, untrusted context.

Tasks:

- Define memory source providers separate from context engines.
- Add recall policy with max tokens/chars, timeout, provenance, confidence, and privacy tier.
- Keep active/preflight recall opt-in and read-only.
- Add wiki/digest integration only through source provider contracts.

Design patterns:

- Strategy for recall providers.
- Repository for durable memory/wiki stores.
- Adapter for external memory systems.

Validation:

- Memory is not loaded globally by default.
- Recall content is fenced as untrusted.
- Prompt cache stable sections are not invalidated by recall.

### Phase 9: User Plugin And External Adapter Path

Goal:

- Allow users to replace Macaca context management with their own systems.

Tasks:

- Publish a conformance test suite for `ContextEngine`.
- Add config-driven provider registration.
- Add local in-process custom provider support first.
- Add process/RPC/WASM adapter only after in-process trait is stable.
- Add adapter safety controls: timeout, max payload, schema validation, circuit breaker, fallback.

Design patterns:

- Ports and Adapters for user systems.
- Bridge for runtime-to-context backend decoupling.
- Abstract Factory for adapter families.
- Anti-Corruption Layer for validation.

Validation:

- A custom context engine can be installed and selected without modifying Macaca core.
- Failure falls back according to policy and emits diagnostics.
- External output cannot bypass budget/trust validation.

### Phase 10: Migration And Deprecation Discipline

Goal:

- Migrate upper layers without deleting old interfaces prematurely.

Tasks:

- Mark legacy direct prompt-building entry points as deprecated once adapter-based call sites exist.
- Prohibit new call sites from using deprecated prompt/context APIs.
- Migrate all internal consumers in small slices.
- Keep deprecated functions searchable for later cleanup.
- Archive OpenSpec changes only after code, tests, and specs align.

Validation:

- `rg` finds no non-test production calls to deprecated prompt/context entry points.
- OpenSpec tasks are all checked only after implementation is complete.
- `openspec validate --strict` passes.
- `gitnexus_detect_changes()` is run before commit.

## First Implementation Proposal Shape

The next concrete user request should create an OpenSpec proposal with this shape:

- Change ID: `add-pluggable-context-engine-foundation`
- Capability: `context-engine`
- Main requirement: Macaca shall provide a pluggable context engine contract selected by configuration and defaulting to legacy behavior.
- Main requirement: Macaca shall produce a context report for each model request.
- Main requirement: Macaca shall compose prompts from typed stable and dynamic sections.
- Main requirement: Macaca shall allow future user-provided context engines through provider registration without app-specific hardcoding.

Acceptance examples:

- Existing agents produce equivalent LLM messages when using `legacy`.
- Switching engine ID changes the selected strategy without changing application code.
- Context report identifies source breakdown without storing full prompt by default.
- Dynamic request-only context does not alter stable prompt hash.
- No core code branches on application name or workflow name.

## Non-Goals

- Do not implement a full OpenClaw clone.
- Do not copy Hermes' monolithic agent loop.
- Do not put context policy into `macaca-llm`.
- Do not make memory the only context abstraction.
- Do not expose external context protocol before the in-process trait is proven.
- Do not add UI pagination or trace storage changes in this context-engineering proposal unless needed for reports.
- Do not delete legacy interfaces in the first migration slice.

## Decision Checklist For Every Future Module

- What problem does this module own?
- Which design pattern fits the problem, if any?
- Is the pattern improving extensibility or just adding ceremony?
- Does the module depend on an abstraction rather than a concrete implementation?
- Can a user replace this behavior without modifying Macaca core?
- Is the behavior selected by config/profile instead of app-name branching?
- Are stable and dynamic prompt sources separated?
- Is untrusted context explicitly marked?
- Is original data preserved when pruning or compaction derives a smaller view?
- Can the module be tested independently?
- Will the file remain below 500 lines?
