# Macaca Context / Memory / Skills / MCP Integration Brainstorm And Plan

Date: 2026-05-05

## Inputs

- Research report: `docs/context-memory-integration-openclaw-hermes-research.md`
- Existing context-engineering report: `docs/context-engineering-openclaw-hermes-research.md`
- Existing memory-system report: `docs/memory-system-openclaw-hermes-research.md`
- Design pattern guide: `macaca/docs/design_patterns.md`
- Existing Macaca memory work:
  - `add-memory-fabric-core`
  - `add-memory-vector-backend-topology`
  - `add-memory-provider-runtime`
  - `add-memory-active-recall-integration`
  - `add-memory-governance-knowledge-layer`

## Constraints

- Macaca is an agent OS infrastructure layer, not an application-specific agent.
- No workflow, application, driver, business name, or provider-specific hardcode may enter the core design.
- Context engineering, memory, skills, MCP, tools, LLM, runtime, and web must remain loosely coupled.
- Every feature/module must first ask whether a design pattern improves clarity, extensibility, performance, or testability.
- Use small, reviewable, reversible slices.
- Prefer traits, value objects, and adapters over concrete cross-crate dependencies.
- Do not couple `macaca-context` directly to Milvus, a specific MCP server, a specific skill source, or a specific app.
- Do not couple `macaca-memory` directly to prompt rendering, model providers, web UI, or application orchestration.
- Do not couple `macaca-skill` directly to memory or MCP execution policy; it should expose skill metadata/capability snapshots.
- Do not couple MCP transport directly to prompt construction; MCP should expose capability candidates through an adapter boundary.
- Dynamic recall, MCP resources, and loaded skill content must not mutate canonical transcript.

## Superpowers Brainstorm

### Design Principles

- **Dependency inversion:** upper runtime depends on `ContextFacade`, `MemoryFacade`, `SkillContextProvider`, and `McpContextProvider` traits, not on concrete stores or transports.
- **Composition over inheritance:** context is assembled by composing providers, policies, scanners, renderers, and budgeters.
- **Open/closed principle:** adding a new memory backend, MCP provider, skill source, or context engine should register an implementation, not edit the runtime loop.
- **Interface segregation:** separate profile file loading, capability indexing, active recall, prompt rendering, memory promotion, and tool schema routing.
- **Stable/dynamic split:** stable profile/capability summaries can sit in cached system prefix; active recall/MCP resource/loaded skill context must be dynamic or ephemeral.
- **Provider-neutral topology:** Milvus remains the default vector backend, but `application -> database` and `agent -> collection` are topology semantics, not Milvus-specific code.
- **Auditability by default:** every context injection emits source, scope, trust, budget, redaction, skipped reason, and diagnostics.
- **Safety by boundary:** untrusted files, MCP resources, memory hits, and external provider output are fenced, scanned, redacted, and budgeted before model visibility.

### Candidate Option A: Context Composer Foundation First

Scope:

- Define a generic context composition pipeline in `macaca-context`.
- Introduce `ContextCandidate`, `ContextProvider`, `ContextComposer`, `ContextPlan`, `ContextReport`.
- Add profile files, active memory recall, skills, MCP, runtime tools, and knowledge digest as providers over time.

Design patterns:

- Chain of Responsibility: providers contribute candidates in ordered stages.
- Builder: `ContextPlanBuilder` assembles validated context plans.
- Composite: final prompt/context is a tree of sections/sources.
- Strategy: budget, ordering, redaction, trust, and render policies are replaceable.
- Facade: upper crates call a narrow `ContextFacade`.

Benefits:

- Creates the correct integration boundary before adding many sources.
- Prevents skills/MCP/memory from becoming ad-hoc string concatenation.
- Makes context report and testing natural.
- Keeps future provider additions additive.

Risks:

- If too abstract, it can become an unused framework.
- If the candidate schema is too wide, every provider becomes noisy.
- Initial default implementation may feel slower than direct prompt building.

Controls:

- Start with minimal fields: `source_id`, `kind`, `scope`, `priority`, `trust`, `cache_class`, `target`, `content`, `budget`, `diagnostics`.
- Implement only one concrete composer first.
- Wrap existing behavior initially instead of changing model-visible output.

Assessment:

- Best foundation option.
- Should be the first implementation slice.

### Candidate Option B: Agent Profile Bootstrap First

Scope:

- Implement `AGENTS.md`, `SOUL.md`, `TOOLS.md`, `IDENTITY.md`, `USER.md`, `HEARTBEAT.md`, `MEMORY.md` loading as profile context.
- Add file priority, safety scan, budget, stable/dynamic classification, and context report.

Design patterns:

- Adapter: profile files become provider-neutral `ContextCandidate`.
- Template Method: shared safe-load pipeline, file-kind-specific policy hooks.
- Strategy: file priority, scan policy, truncation policy.
- Value Object: `AgentProfileFileKind`, `AgentProfileFilePriority`, `ProfileFileSnapshot`.

Benefits:

- Directly supports each agent's identity/behavior files.
- Makes profile files visible without coupling them to runtime prompt code.
- Low dependency risk if implemented under `macaca-context`.

Risks:

- If done before composer, it can become yet another prompt helper.
- `MEMORY.md` can bloat prompt if treated as ordinary profile content.
- `HEARTBEAT.md` can leak proactive behavior into normal runs if classification is wrong.

Controls:

- Implement as `ProfileFileContextProvider`, not a web/runtime helper.
- Default full injection only for high-priority short files.
- Treat `MEMORY.md` as seed/audit surface, not default full prompt.
- Treat `HEARTBEAT.md` as heartbeat/dynamic only unless explicitly enabled.

Assessment:

- Strong early slice after context composer foundation.

### Candidate Option C: Active Vector Recall First

Scope:

- Integrate long-term vector memory into context preflight.
- Query AgentPrivate and optionally SessionShared before model calls.
- Return dynamic/ephemeral context candidates with report diagnostics.

Design patterns:

- Strategy: active recall policy, query derivation, rerank, scope selection.
- Adapter: vector backend topology adapter hides Milvus and alternative providers.
- Decorator: governance/tombstone/redaction wrappers around recall.
- Repository: memory vector store remains behind `MemoryFacade` / provider contract.

Benefits:

- Delivers the user's key requirement: long-term memory is proactive, not keyword-triggered.
- Makes AgentPrivate and SessionShared operational in model context.
- Reuses memory work already implemented.

Risks:

- Recall noise can degrade model behavior.
- Vector backend latency can slow every model call.
- Tight dependency from `macaca-context` to `macaca-memory` could appear.
- Tombstone/governance filtering may be missed if direct backend calls are used.

Controls:

- `macaca-context` depends on a narrow `ActiveRecallCapability` or adapter, not on concrete vector backend.
- Default policy has conservative hit/budget limits.
- Recall must pass governance/tombstone/redaction filters.
- Timeout/fallback must be fail-open: no recall should not block the main loop.

Assessment:

- Required, but should be built on context candidate/composer contracts.

### Candidate Option D: Skills/MCP Capability Context First

Scope:

- Add skill and MCP capability summaries to model-visible context.
- Skill index is stable and compact; `SKILL.md` body remains on-demand.
- MCP server/tool/resource/prompt capability map enters context with safety boundaries.

Design patterns:

- Adapter: skill snapshots and MCP registries become `CapabilityCandidate`.
- Composite: skill/MCP/tool capabilities form a capability tree.
- Strategy: capability selection and budget policy.
- Facade: `CapabilityContextFacade` for runtime/Web without exposing source internals.
- Mediator: optional future mediator maps selected skills to required MCP capabilities.

Benefits:

- Macaca's existing skills and MCP support become usable by the model in a disciplined way.
- Reduces tool confusion by giving usage policy, not only schema.
- Enables skills to declare MCP dependencies without runtime hardcoding.

Risks:

- Capability index can become huge.
- MCP resources/prompts may be untrusted prompt-injection surfaces.
- Tool/schema naming collisions can confuse providers.
- Skill/MCP coupling can become tight if skills call MCP directly.

Controls:

- Context only carries compact capability index and usage discipline by default.
- MCP resources are dynamic/untrusted/fenced and never high-priority unless explicitly trusted.
- Namespace and dedup capabilities before tool schema exposure.
- Skill dependencies reference capability IDs, not concrete MCP server internals.

Assessment:

- Should be implemented soon after profile and memory because it is core to Macaca's agent OS promise.

### Candidate Option E: External Provider Protocol First

Scope:

- Define external interfaces for custom context managers, memory systems, skill resolvers, and MCP registries.
- Allow user replacement through local process, RPC, WASM, MCP, or plugin adapters.

Design patterns:

- Ports and Adapters: Macaca owns the port, user systems implement adapters.
- Bridge: runtime abstraction separated from concrete backend implementations.
- Abstract Factory: instantiate provider family from config.
- Anti-Corruption Layer: validate external output into Macaca context/memory/capability candidates.

Benefits:

- Strongest answer to pluggability.
- Keeps Macaca from owning every future context/memory implementation.
- Lets enterprise users plug existing systems in.

Risks:

- Too early remote protocol design can freeze the wrong shape.
- Security, timeout, streaming, and schema validation complexity grows quickly.
- External systems may return unsafe or oversized prompts.

Controls:

- First define in-process Rust traits and local adapters as source of truth.
- Add external adapters only after candidate/context report schema is proven.
- Enforce validation, budget, timeout, scan, trust, and fallback at boundary.

Assessment:

- Important later slice, not first.

### Candidate Option F: Direct Runtime Integration First

Scope:

- Edit runtime/web/framework prompt assembly directly to inject profile files, memory recall, skills, and MCP.

Design patterns:

- Minimal use of existing builder/helpers only.

Benefits:

- Fastest visible behavior.
- Less initial abstraction.

Risks:

- Violates low-coupling requirement.
- Creates prompt-building spaghetti.
- Hard to replace with user context system later.
- Memory/skills/MCP become tightly coupled through runtime code.

Controls:

- Do not choose this option.

Assessment:

- Rejected.

## Recommended Direction

Choose a staged hybrid:

1. Start with **Option A: Context Composer Foundation**.
2. Add **Option B: Agent Profile Bootstrap** as first real provider.
3. Add **Option C: Active Vector Recall** through the existing memory facade/capability boundary.
4. Add **Option D: Skills/MCP Capability Context** as capability providers.
5. Add **Option E: External Provider Protocol** only after local contracts are stable.

Do not implement direct runtime injection as the primary path. Runtime should call a facade and receive a compiled context plan; it should not know how profile files, vector memory, skills, or MCP are discovered internally.

## Architecture Plan

### Core Concepts

`ContextCandidate`

- A provider-neutral unit of context before final rendering.
- Fields: `source_id`, `source_kind`, `scope`, `priority`, `trust_level`, `cache_class`, `target`, `content`, `budget_hint`, `redaction`, `diagnostics`.

`ContextProvider`

- Trait implemented by profile files, active recall, skills, MCP, knowledge digest, runtime tools, and heartbeat.
- Returns zero or more candidates for a `ContextRequest`.

`ContextComposer`

- Owns ordering, budget allocation, dedup, trust handling, rendering, and report creation.
- Does not know provider implementation details.

`ContextPlan`

- Final pre-render tree with stable prefix, dynamic suffix, ephemeral user context, tool-only hints, and skipped candidates.

`CompiledContext`

- Model-call-ready result: system additions, dynamic sections, ephemeral user context, reports, hashes, and diagnostics.

`CapabilityCandidate`

- Specialized context candidate for skills, MCP, builtin tools, memory tools, and external provider capabilities.
- Keeps capability index separate from tool schema.

### Design Patterns To Apply

- Strategy: recall policy, budget policy, render policy, provider selection, redaction policy.
- Chain of Responsibility: context providers contribute candidates in a deterministic pipeline.
- Builder: build `ContextPlan` and `CompiledContext` with validation.
- Facade: runtime/web/framework call `ContextFacade` instead of provider internals.
- Adapter: wrap existing skill snapshots, MCP registries, memory facade, and legacy prompt assembly.
- Decorator: governance, metrics, timeout, redaction, and audit around providers.
- Bridge: separate context abstraction from local/remote provider implementations.
- Abstract Factory: provider family creation at composition root.
- Mediator: future skill-to-MCP capability coordination without direct coupling.

## Write-Plan

### Phase 0: OpenSpec Planning

Goal:

- Create OpenSpec proposal/design/tasks/spec before implementation.

Tasks:

- Define change id such as `integrate-context-memory-skills-mcp`.
- Proposal should state why Macaca needs unified, pluggable context integration.
- Design should document selected patterns and rejected direct-runtime injection.
- Spec deltas should cover context composition, profile files, active vector recall, skills/MCP capability context, and audit/reporting.
- Tasks should be staged and independently testable.

Verification:

- `openspec validate integrate-context-memory-skills-mcp --strict`

### Phase 1: Context Candidate And Composer Foundation

Goal:

- Add minimal, provider-neutral foundation in `macaca-context`.

Tasks:

- Define `ContextCandidate`, `ContextSourceKind`, `ContextScope`, `ContextPriority`, `ContextTrustLevel`, `ContextCacheClass`, `ContextTarget`.
- Define `ContextRequest`, `ContextProvider`, `ContextPlan`, `CompiledContext`, `ContextReport`.
- Implement default `ContextComposer` with deterministic ordering and budget placeholders.
- Implement no-op/legacy adapter provider so existing behavior can be represented without behavior change.
- Add tests for ordering, cache class separation, skipped candidate reporting, and deterministic hashes.

Design rules:

- No dependency on Milvus, MCP transport, web UI, application names, or concrete skill source.
- Keep files under 500 lines; split candidate/model/composer/report/policy modules.

Risks:

- Overly broad types.
- Hidden behavior change.

Controls:

- Start additive.
- Avoid changing model prompt output in this phase.

### Phase 2: Agent Profile File Provider

Goal:

- Integrate per-agent profile files as first real context provider.

Tasks:

- Define `AgentProfileFileKind` for `AGENTS`, `SOUL`, `TOOLS`, `IDENTITY`, `USER`, `HEARTBEAT`, `MEMORY`.
- Define priority and default cache class:
  - high stable: `AGENTS.md`, `SOUL.md`
  - medium stable/dynamic: `TOOLS.md`, `IDENTITY.md`
  - low stable/dynamic: `USER.md`
  - heartbeat dynamic: `HEARTBEAT.md`
  - memory seed/audit: `MEMORY.md`
- Implement safe file loader with path boundary, size budget, frontmatter stripping, and scanner hook.
- Emit profile file candidates through `ProfileFileContextProvider`.
- Add context report fields for loaded/skipped/truncated/blocked files.
- Add tests for priority order, heartbeat exclusion, memory no-full-default, and prompt injection blocking.

Design patterns:

- Adapter for file snapshots.
- Template Method for safe load pipeline.
- Strategy for file-kind policy.

Risks:

- Prompt bloat.
- `HEARTBEAT.md` leaking into normal runs.

Controls:

- Strict defaults and report diagnostics.

### Phase 3: Active Vector Memory Recall Provider

Goal:

- Make long-term vector memory proactive in context preflight.

Tasks:

- Add `MemoryActiveRecallContextProvider` adapter over existing memory active recall capability.
- Derive recall request from current turn, goal/session metadata, agent identity, and capability context.
- Query AgentPrivate by default.
- Query SessionShared/project shared by policy.
- Use provider-neutral topology contract; never call Milvus directly from context.
- Apply governance/tombstone/redaction filters.
- Emit dynamic or ephemeral context candidates.
- Add timeout/fallback behavior and diagnostics.
- Add tests for no canonical transcript mutation, scope filtering, tombstone exclusion, budget truncation, and provider failure fallback.

Design patterns:

- Strategy for recall policy.
- Adapter over `MemoryFacade`/active recall capability.
- Decorator for timeout/governance/redaction.

Risks:

- Recall noise.
- Latency on every model call.
- Context-memory tight coupling.

Controls:

- Conservative default hit limits.
- Strict trait boundary.
- Fail-open timeout.

### Phase 4: Skills Capability Context Provider

Goal:

- Expose Agent Skills ecosystem through compact, auditable context.

Tasks:

- Define `CapabilityCandidate` for skills.
- Implement `SkillContextProvider` adapter over existing skill snapshot/resolver.
- Render compact skill index and mandatory skill-use discipline.
- Keep `SKILL.md` body on-demand only.
- Record selected/loaded skill usage in context report when available.
- Add tests for compact index budget, no full skill body injection, ordering, and skill source scope.

Design patterns:

- Adapter over `macaca-skill`.
- Composite capability tree.
- Strategy for capability budget.

Risks:

- Too many skills in context.
- Skill source coupling to runtime.

Controls:

- Budgeted compact index.
- Source-neutral skill metadata.

### Phase 5: MCP Capability Context Provider

Goal:

- Expose MCP protocol capabilities through context without coupling transport to prompt rendering.

Tasks:

- Define MCP capability adapter trait or use existing registry snapshots if available.
- Implement `McpContextProvider` that emits server/tool/resource/prompt capability candidates.
- Separate stable capability summary from dynamic server health/auth/resource hints.
- Treat MCP resource/prompt content as untrusted/dynamic unless explicitly trusted.
- Add namespace/dedup diagnostics for MCP vs builtin/tool/memory/skill names.
- Add tests for resource non-full-default injection, untrusted fence, namespace conflict, and skill-to-MCP dependency hint.

Design patterns:

- Adapter over MCP registry.
- Facade for runtime access.
- Mediator for future skill/MCP relation.

Risks:

- MCP prompt injection.
- Tool schema bloat and collisions.
- Tight coupling to transport.

Controls:

- Context provider consumes snapshots/capabilities, not live transport internals.
- Resource content remains on-demand and fenced.

### Phase 6: Knowledge Digest And Governance Context

Goal:

- Use governed memory knowledge artifacts as higher-quality context.

Tasks:

- Add `KnowledgeDigestContextProvider` adapter over memory governance compiled digest.
- Prefer digest/claim candidates over raw recall when both exist and evidence supports it.
- Report evidence IDs without leaking full sensitive raw text.
- Add tests for digest priority, evidence references, redaction, and tombstone propagation.

Design patterns:

- Adapter over governance layer.
- Strategy for digest-vs-raw selection.
- Decorator for redaction/audit.

Risks:

- Stale digest outranking fresh recall.

Controls:

- Freshness and confidence in selection policy.

### Phase 7: Runtime Integration Through Facade

Goal:

- Route upper runtime prompt/model-call preparation through `ContextFacade`.

Tasks:

- Identify current context assembly call sites in web/framework/runtime.
- Run GitNexus impact before editing symbols.
- Add facade invocation that receives `CompiledContext`.
- Preserve existing model-visible output initially where possible.
- Inject dynamic recall/skill/MCP/profile sections according to plan targets.
- Ensure canonical transcript is unchanged by dynamic context.
- Emit context report linked to trace/session.

Design patterns:

- Facade for upper crates.
- Adapter for legacy prompt path.

Risks:

- Behavioral regression in prompt content.
- Cross-crate coupling.

Controls:

- Add feature/config flags if needed.
- Snapshot test compiled prompt sections.
- Keep adapters narrow.

### Phase 8: External Provider Extension Boundary

Goal:

- Prepare pluggability without prematurely freezing remote protocol.

Tasks:

- Document in-process provider trait as stable internal port.
- Add registry/factory only for local providers first.
- Define validation and fallback rules for future external adapters.
- Avoid RPC/WASM/MCP provider protocol until local contract has tests and at least two providers.

Design patterns:

- Ports and Adapters.
- Abstract Factory at composition root.
- Anti-Corruption Layer for future external output.

Risks:

- Overdesign.

Controls:

- Documentation and trait-level boundary first; no heavy runtime protocol yet.

## Cross-Cutting Tests

- Context candidates render deterministically.
- Stable prefix hash does not change when active recall changes.
- Dynamic recall does not mutate canonical transcript.
- Profile file injection respects priority and heartbeat mode.
- `MEMORY.md` is not full-injected by default.
- Active vector recall applies application/agent/session metadata filters.
- Tombstoned memories never appear in recall candidates.
- Skill index does not include full `SKILL.md` body.
- MCP resource/prompt is fenced and untrusted by default.
- Capability namespace collisions are diagnosed.
- Provider failure falls back without crashing model call.
- Context report contains source IDs, budgets, redaction, skipped reasons, and diagnostics.

## GitNexus / Impact Requirements

- Before editing any existing symbol, run upstream impact analysis for that symbol.
- Expected high-risk symbols are runtime/web/framework prompt assembly and memory recall adapters.
- If GitNexus returns HIGH/CRITICAL, stop and report blast radius before editing.
- Before commit, run `gitnexus detect-changes`.

## Open Questions To Resolve In OpenSpec Design

- Exact crate ownership:
  - likely `macaca-context` owns context candidates/composer/providers.
  - `macaca-memory` owns recall/governance capabilities.
  - `macaca-skill` owns skill metadata/snapshots.
  - MCP crate/module owns server/tool/resource snapshots.
- Exact injection target names and whether ephemeral user context is represented as LLM messages or framework context sections.
- Whether profile files live under agent workspace, application workspace, or both, and how inheritance/override works.
- Whether `TOOLS.md` is profile guidance, generated tool summary, or both.
- How to persist context reports without leaking sensitive context.
- Whether active recall is enabled by default for all agents or per application/agent policy.

## Recommended Next Step

Create OpenSpec change `integrate-context-memory-skills-mcp` with:

- proposal: why unified pluggable context integration is required.
- design: selected staged hybrid architecture, design patterns, rejected direct-runtime injection, and low-coupling boundaries.
- tasks: phases above.
- spec deltas:
  - context composition foundation
  - agent profile files
  - active vector memory recall
  - skills capability context
  - MCP capability context
  - context report/audit

