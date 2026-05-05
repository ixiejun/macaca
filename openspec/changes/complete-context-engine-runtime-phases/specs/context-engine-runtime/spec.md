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
