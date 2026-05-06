## ADDED Requirements

### Requirement: Runtime completion SHALL be distinct from contract definition

Macaca SHALL track context-engine contract availability separately from runtime Phase completion. A Phase is complete only when its behavior is used by supported model-call paths, reported through diagnostics, and verified by tests.

#### Scenario: Contract exists but runtime is not wired

- **GIVEN** a context trait, value object, policy, or engine implementation exists in `macaca-context`
- **WHEN** it is not selected by framework/runtime model-call paths
- **THEN** OpenSpec tasks MUST mark the contract work separately from runtime completion
- **AND** the related Phase MUST NOT be considered complete

#### Scenario: Runtime Phase completion requires evidence

- **GIVEN** a Phase claims to be implemented
- **WHEN** maintainers review the change
- **THEN** the change MUST provide contract evidence, runtime integration evidence, diagnostic/report evidence, and verification evidence

### Requirement: Phase 0 SHALL persist ContextReports for all supported model calls

Macaca SHALL generate and persist `ContextReport` events for every supported LLM request path that passes through Macaca runtime or framework infrastructure.

#### Scenario: Framework model call is reported

- **GIVEN** a framework ReAct agent sends an LLM request
- **WHEN** the request is assembled
- **THEN** Macaca MUST persist a `context_report` EventLog event before or with the model call
- **AND** the report MUST include selected engine id, source breakdown, estimated tokens, hash fields, and warning/fallback decisions

#### Scenario: Runtime direct model call is reported

- **GIVEN** `macaca-runtime` direct agentic loop sends an LLM request
- **WHEN** the request is assembled
- **THEN** Macaca MUST persist a `context_report` EventLog event or equivalent durable report
- **AND** debug logging alone MUST NOT satisfy the Phase 0 requirement

#### Scenario: Unsupported call path is documented

- **GIVEN** an LLM call path cannot use the shared runtime context facade
- **WHEN** Phase 0 is verified
- **THEN** the path MUST be documented with rationale
- **AND** it MUST be covered by a migration or exclusion test

### Requirement: Phase 1 SHALL use PromptComposer for real prompt assembly

Macaca SHALL assemble real framework/runtime prompts from typed `PromptSection`s rather than relying on ad hoc string concatenation as the primary prompt-building path.

#### Scenario: Persona and application prompt sources become typed sections

- **GIVEN** an agent has persona files, application semantics, capabilities, workspace paths, skills, and tool schema
- **WHEN** Macaca builds the system prompt
- **THEN** these sources MUST be represented as typed sections with source id, stability, trust level, and source kind
- **AND** the rendered prompt MUST preserve legacy-compatible behavior unless a selected engine explicitly changes it

#### Scenario: Workspace guide files are loaded as configurable sources

- **GIVEN** an application workspace or app directory contains configured guide files such as `AGENTS.md`, `SOUL.md`, `TOOLS.md`, `IDENTITY.md`, `USER.md`, or `HEARTBEAT.md`
- **WHEN** prompt sections are assembled
- **THEN** Macaca MUST load existing guide files through a source provider with deterministic priority and budget handling
- **AND** missing files MUST be skipped without error
- **AND** truncation or exclusion MUST be visible in `ContextReport`

#### Scenario: Stable hash ignores dynamic runtime data

- **GIVEN** stable prompt sections remain unchanged and dynamic session metadata, time, trace snippets, or recall results change
- **WHEN** `PromptComposer` renders the request
- **THEN** `stable_prompt_hash` MUST remain unchanged
- **AND** `prompt_hash` MAY change

### Requirement: Phase 2 SHALL apply pruning to selected runtime engines

Macaca SHALL apply non-destructive pruning to real model context when a pruning-capable engine is selected.

#### Scenario: Pruning engine is selected by configuration

- **GIVEN** system config, application manifest, or agent profile selects a pruning-capable engine
- **WHEN** framework or runtime assembles an LLM request
- **THEN** Macaca MUST use that engine without changing application code
- **AND** large tool outputs, trace events, command stdout, file reads, or search results MUST be rendered as bounded snippets in model context

#### Scenario: Original source remains retrievable

- **GIVEN** a large source is pruned before entering model context
- **WHEN** an authorized debug client follows the report source reference
- **THEN** the original source payload MUST remain retrievable from canonical storage
- **AND** pruning MUST NOT rewrite canonical transcript, EventLog, or artifact data

#### Scenario: Pruning is explained in diagnostics

- **GIVEN** pruning changes model-visible source content
- **WHEN** context report API or UI displays the request
- **THEN** it MUST show render mode, pruned tokens, trust level, source kind, and source reference

### Requirement: Phase 3 SHALL provide compaction runtime flow and lineage UI

Macaca SHALL perform compaction as a runtime flow that creates successor context while preserving original history and exposing lineage diagnostics.

#### Scenario: Automatic compaction creates successor lineage

- **GIVEN** a session approaches the configured context budget threshold
- **WHEN** a compaction-capable engine assembles context
- **THEN** Macaca MUST trigger compaction according to policy
- **AND** it MUST create a successor transcript segment or child session linked to root lineage
- **AND** future logical session access MUST resolve to the lineage tip by default

#### Scenario: Manual compaction supports focus topic

- **GIVEN** a user or API client requests manual compaction with an optional focus topic
- **WHEN** compaction completes
- **THEN** Macaca MUST create a reference-only summary focused on the requested topic when provided
- **AND** the original transcript MUST remain readable

#### Scenario: Compaction diagnostics are visible

- **GIVEN** compaction occurs
- **WHEN** context report or trace UI is opened
- **THEN** UI/API MUST show compaction event, summary source, successor id, root id, and lineage tip
- **AND** debug mode SHOULD allow root-to-tip expansion

### Requirement: Phase 4 SHALL provide runtime memory recall and wiki injection

Macaca SHALL support memory and wiki recall as optional, bounded, untrusted runtime context sources.

#### Scenario: Memory recall tools are read-only and explicit

- **GIVEN** memory recall is not enabled and no recall tool is invoked
- **WHEN** a model request is assembled
- **THEN** Macaca MUST NOT load all memory into context by default
- **AND** no memory recall source MUST appear in `ContextReport`

#### Scenario: Recall output is request-only dynamic context

- **GIVEN** `memory_search`, `memory_get`, wiki digest, or preflight recall returns context
- **WHEN** Macaca includes the recall in a model request
- **THEN** it MUST be rendered as dynamic, untrusted, request-only context
- **AND** it MUST carry provenance, confidence, privacy tier, and source id metadata
- **AND** it MUST NOT be written back to canonical transcript

#### Scenario: Preflight recall degrades safely

- **GIVEN** preflight recall is enabled and the recall step times out or fails
- **WHEN** the main model call proceeds
- **THEN** Macaca MUST degrade to empty or partial recall according to policy
- **AND** `ContextReport` MUST record a warning decision

### Requirement: Phase 5 SHALL provide configurable pluggable engines

Macaca SHALL allow applications and agents to select built-in or custom context engines through configuration without code changes in application logic.

#### Scenario: Built-in engines are selectable

- **GIVEN** system config, application manifest, or agent profile selects `legacy`, `windowed`, `pruning`, or `summary`
- **WHEN** Macaca creates a model request
- **THEN** it MUST select the corresponding context engine through a registry/factory
- **AND** OS crates MUST NOT branch on app name, workflow name, agent name, or business identifiers

#### Scenario: Engine failure falls back observably

- **GIVEN** a selected engine fails during context assembly
- **WHEN** fallback policy allows recovery
- **THEN** Macaca MUST fallback to the configured fallback engine or empty optional contribution
- **AND** it MUST emit a context report decision and fallback event
- **AND** the main loop MUST not crash because of an optional context contribution failure

#### Scenario: Custom engine can be registered in-process

- **GIVEN** a user registers a custom in-process context engine or source provider that passes conformance tests
- **WHEN** config selects that implementation
- **THEN** framework/runtime/web layers MUST call it only through public context abstractions
- **AND** no upper layer may require its concrete type

### Requirement: Phase 6 SHALL preserve retrievable originals for every pruned source

Macaca SHALL treat pruning as non-destructive for every supported source kind. Any source rendered as excerpt, summary, or dropped content in model-visible context MUST preserve its original payload in canonical storage and MUST remain retrievable through a source reference or an authorized debug path.

#### Scenario: Pruned source remains retrievable through scoped repository

- **GIVEN** a large model-call source is pruned before entering model context
- **WHEN** a maintainer or debug client follows the `ContextReport` source reference
- **THEN** Macaca MUST resolve the reference through a scoped repository or adapter boundary
- **AND** the pruning step MUST NOT rewrite or replace the original payload in EventLog, artifact storage, or session storage

#### Scenario: Source retrieval rejects cross-session references

- **GIVEN** a context report source ref belongs to session `A`
- **WHEN** a client attempts to resolve it through session `B`
- **THEN** Macaca MUST reject the lookup
- **AND** the response MUST NOT reveal the source payload or whether the source exists outside the authorized scope

#### Scenario: Source retrieval reports unavailable originals explicitly

- **GIVEN** an old or unsupported pruned source has no stable canonical original payload
- **WHEN** diagnostics render that source
- **THEN** Macaca MUST report an explicit unavailable reason
- **AND** it MUST NOT claim that the original is retrievable

### Requirement: Phase 7 SHALL provide lineage-aware logical session UX

Macaca SHALL expose compaction lineage as a first-class runtime capability. Logical session access SHALL resolve to the current lineage tip by default, while diagnostics and UI SHALL allow operators to inspect the root-to-tip lineage without losing auditability.

#### Scenario: Logical session access resolves to lineage tip

- **GIVEN** a root session has one or more compaction successor nodes
- **WHEN** runtime or web code loads the logical session for normal operation
- **THEN** Macaca MUST resolve the request to the current lineage tip by default
- **AND** the original lineage root and predecessor chain MUST remain queryable

#### Scenario: UI shows lineage and compaction details

- **GIVEN** a session has been compacted manually or automatically
- **WHEN** a user opens the chat trace or diagnostics UI
- **THEN** the UI MUST show root session id, tip session id, successor chain, and compaction summary metadata
- **AND** a debug interaction MUST allow root-to-tip lineage expansion without changing the default logical-session view

#### Scenario: Compaction summary is reference-only

- **GIVEN** a compaction summary is inserted into the successor context
- **WHEN** the summary is rendered for a model request
- **THEN** it MUST be fenced or otherwise marked as reference-only context
- **AND** it MUST NOT be treated as a new user instruction

### Requirement: Phase 8 SHALL provide full runtime memory and wiki recall injection

Macaca SHALL support memory recall and wiki/digest recall as optional runtime context sources. These recall outputs MUST be bounded, untrusted, request-only, and explicitly described in `ContextReport`.

#### Scenario: Wiki or digest recall is runtime-selectable

- **GIVEN** wiki/digest recall is enabled through config, profile, or explicit source-provider path
- **WHEN** a model request is assembled
- **THEN** Macaca MUST retrieve wiki/digest recall through the context runtime path
- **AND** the result MUST be included only as dynamic, untrusted, request-only context

#### Scenario: Recall carries provenance and privacy metadata

- **GIVEN** memory recall, wiki recall, preflight recall, or active recall returns one or more context items
- **WHEN** those items are injected into a model request
- **THEN** each injected item MUST carry provenance, confidence, privacy tier, and source id metadata
- **AND** the canonical transcript MUST NOT be mutated by that injection

#### Scenario: Recall provider cannot mutate transcript

- **GIVEN** a memory or wiki/digest context provider contributes recall candidates
- **WHEN** the provider is invoked during model request assembly
- **THEN** it MUST receive request metadata and source query inputs only
- **AND** it MUST NOT receive a mutable reference to the canonical transcript

#### Scenario: Recall diagnostics may persist without duplicating recall body as transcript

- **GIVEN** recall output is injected into a model request
- **WHEN** Macaca records diagnostics
- **THEN** diagnostics MAY store source ids, evidence ids, warnings, budgets, and bounded summaries
- **AND** the injected full recall body MUST NOT be appended as a canonical transcript message

### Requirement: Phase 9 SHALL provide installable custom engines and safe external adapters

Macaca SHALL allow operators to install custom in-process context engines/providers and SHALL provide a safe adapter boundary for process, RPC, or WASM-backed external context managers.

#### Scenario: Custom in-process engine is registered and selected by config

- **GIVEN** an operator registers a custom in-process context engine or provider that passes the published conformance checks
- **WHEN** system, application, or agent profile configuration selects that implementation
- **THEN** runtime, framework, and web code MUST invoke it only through public context abstractions
- **AND** no application logic changes MUST be required to activate it

#### Scenario: External adapter output is validated and bounded

- **GIVEN** an external process, RPC service, or WASM adapter returns assembled context or diagnostics
- **WHEN** Macaca receives the adapter output
- **THEN** Macaca MUST enforce timeout, payload size, schema validation, trust fencing, and fallback policy before the output reaches the LLM request
- **AND** malformed or oversized output MUST degrade safely without crashing the main loop

#### Scenario: External adapter is opt-in

- **GIVEN** no external context adapter is explicitly configured
- **WHEN** Macaca boots runtime, framework, or web context paths
- **THEN** no external process, RPC service, or WASM context adapter MUST be started
- **AND** builtin context engines/providers MUST remain available

### Requirement: Phase 10 SHALL enforce migration and archive discipline

Macaca SHALL complete context-engine migration only when deprecated legacy prompt/context entry points remain searchable, all production consumers have migrated away from them, and verification/archival gates have been satisfied.

#### Scenario: Deprecated legacy entry remains searchable but unused by production

- **GIVEN** a prompt or context entry point has been replaced by `ContextFacade`, `ContextRuntimeFacade`, or equivalent abstractions
- **WHEN** migration reaches final closure
- **THEN** the old entry point MUST remain in the codebase with a deprecated marker and replacement guidance
- **AND** no non-test production code MUST continue to call it

#### Scenario: Archive gates require evidence

- **GIVEN** maintainers want to archive context-engine related changes
- **WHEN** they evaluate readiness
- **THEN** OpenSpec validation, targeted tests, Phase status updates, and available GitNexus impact/change evidence MUST all be complete
- **AND** a change MUST NOT be archived while runtime or diagnostics gaps remain open for any claimed-complete Phase

#### Scenario: Deprecated API remains for migration search

- **GIVEN** a legacy prompt/context API is replaced
- **WHEN** maintainers search for migration targets
- **THEN** the legacy API MUST remain present with replacement guidance
- **AND** it MUST NOT be deleted solely to make scans pass

#### Scenario: New closure code remains application-generic

- **GIVEN** new code is added for context phase closure
- **WHEN** maintainers run final scans or review implementation
- **THEN** the code MUST NOT branch on hardcoded application names, workflow names, driver names, provider names, or business-specific identifiers
- **AND** behavior selection MUST come from config, manifest, profile, or registered provider capabilities
