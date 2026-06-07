## ADDED Requirements

### Requirement: Context sources SHALL support non-destructive rendering and pruning

Macaca SHALL render large or structured context sources through replaceable renderers and pruning policies before they enter model context, while preserving the canonical source data unchanged.

#### Scenario: Large tool output is summarized without data loss

- **GIVEN** a tool result, trace event, file read, command output, or search result exceeds the configured context budget threshold
- **WHEN** the context engine assembles the model request
- **THEN** Macaca MUST render a bounded snippet containing a summary, excerpt, source reference, and estimated token count
- **AND** the original source payload MUST remain available in the canonical event, transcript, or artifact store
- **AND** `ContextReport` MUST record the pruning decision and estimated pruned tokens

#### Scenario: Renderer selection remains application agnostic

- **GIVEN** multiple applications, workflows, agents, or sessions use the same source kind
- **WHEN** the default renderer or pruning policy decides how to render that source
- **THEN** the decision MUST be based on source kind, size, budget, trust level, and configuration
- **AND** OS crates MUST NOT branch on application name, workflow name, agent name, driver name, or business-specific identifiers

#### Scenario: User replaces pruning policy

- **GIVEN** a user registers a custom pruning or budget policy through the context contract
- **WHEN** the application or agent profile selects that policy
- **THEN** Macaca MUST call the policy through the standard abstraction
- **AND** runtime, framework, and web layers MUST NOT depend on the policy concrete type

### Requirement: Compaction SHALL preserve lineage and auditability

Macaca SHALL support context compaction for long-running sessions by deriving a compacted summary and successor transcript/session, without overwriting or deleting the original history.

#### Scenario: Compaction creates a successor

- **GIVEN** a session approaches its configured context budget or a user triggers manual compaction
- **WHEN** compaction succeeds
- **THEN** Macaca MUST create a successor transcript segment or child session linked to the original lineage
- **AND** the original transcript segment MUST remain readable for audit and debug purposes
- **AND** logical session queries MUST resolve to the latest lineage tip by default

#### Scenario: Compaction summary is reference-only

- **GIVEN** Macaca renders a compaction summary into model context
- **WHEN** the summary is included in a future request
- **THEN** the summary MUST use a fixed envelope that marks it as compaction-derived, untrusted, and reference-only
- **AND** the summary MUST NOT be treated as a new user instruction
- **AND** the summary MUST preserve active task, decisions, open questions, important IDs, and important paths

#### Scenario: Compaction lifecycle hooks run before history is reduced

- **GIVEN** memory or source providers are registered for a session
- **WHEN** compaction is about to summarize or derive a successor
- **THEN** Macaca MUST invoke bounded `before_compaction` hooks through provider abstractions
- **AND** providers MUST receive only the authorized context view, not unrestricted access to internal stores

### Requirement: Memory and wiki context SHALL be optional source providers

Macaca SHALL model memory recall and wiki or digest knowledge as optional context source providers, separate from the context engine and separate from each other.

#### Scenario: Memory is not globally loaded by default

- **GIVEN** durable memory or wiki data exists for an application, user, or session
- **WHEN** a model request is assembled without explicit recall configuration or tool invocation
- **THEN** Macaca MUST NOT inject all memory or wiki content into the prompt by default
- **AND** the context report MUST distinguish memory tokens from wiki or digest source tokens when they are included

#### Scenario: Recall result carries provenance and trust metadata

- **GIVEN** a memory or wiki provider returns recalled context
- **WHEN** the context engine includes that recall in a model request
- **THEN** the recall MUST carry source id, provenance, confidence, and privacy tier metadata
- **AND** the recall MUST be rendered as dynamic, untrusted, request-only context
- **AND** the recall MUST NOT be written back to the canonical session transcript

#### Scenario: Memory provider participates through lifecycle hooks

- **GIVEN** a memory provider needs to sync turns, handle session switches, or flush durable insights before compaction
- **WHEN** the relevant lifecycle event occurs
- **THEN** Macaca MUST call the provider through a standard hook interface
- **AND** the provider MUST NOT bypass the context source contract to mutate LLM requests directly

### Requirement: Preflight recall SHALL be bounded and opt-in

Macaca SHALL support optional preflight recall before the main model call only when explicitly enabled, and the preflight step MUST be bounded, read-only, and safely degradable.

#### Scenario: Preflight recall is disabled by default

- **GIVEN** no application manifest, agent profile, or system configuration enables preflight recall
- **WHEN** a model request is assembled
- **THEN** Macaca MUST skip the preflight recall step
- **AND** the default legacy or selected context engine behavior MUST continue without recall injection

#### Scenario: Preflight recall uses only read-only tools

- **GIVEN** preflight recall is enabled for an application or agent profile
- **WHEN** Macaca runs the preflight recall step
- **THEN** the step MUST be limited to configured read-only recall, search, or get tools
- **AND** it MUST NOT execute write, shell, network mutation, or application action tools

#### Scenario: Preflight failure degrades safely

- **GIVEN** preflight recall times out, fails, or exceeds its configured budget
- **WHEN** Macaca proceeds with the main model call
- **THEN** the recall result MUST degrade to empty or partial context according to policy
- **AND** the main model call MUST continue unless the selected policy explicitly marks the failure as fatal
- **AND** `ContextReport` MUST include a warning decision

### Requirement: External context systems SHALL remain behind adapter safety boundaries

Macaca SHALL allow users to replace or extend context management with external systems only through adapters that enforce schema, budget, trust, timeout, and fallback boundaries.

#### Scenario: Custom in-process engine passes conformance tests

- **GIVEN** a user provides an in-process custom context engine or source provider
- **WHEN** it is registered for use by Macaca
- **THEN** it MUST satisfy the context contract conformance tests
- **AND** it MUST produce valid compiled context and context reports
- **AND** upper layers MUST interact with it only through the public abstraction

#### Scenario: External adapter output is validated

- **GIVEN** a future process, RPC, WASM, or other external context adapter returns context output
- **WHEN** Macaca receives that output
- **THEN** Macaca MUST validate schema, source metadata, token budget, trust boundaries, and maximum payload size before using it
- **AND** untrusted external content MUST remain fenced and dynamic
- **AND** invalid output MUST be rejected or fallback according to policy

#### Scenario: External adapter failure is observable and recoverable

- **GIVEN** an external context adapter times out, returns invalid data, exceeds payload limits, or trips a circuit breaker
- **WHEN** a model request is assembled
- **THEN** Macaca MUST apply the configured fallback context engine or empty external contribution
- **AND** the failure MUST be recorded in `ContextReport`
- **AND** the failure MUST NOT bypass OS-level budget and trust validation

### Requirement: Context diagnostics SHALL explain pruning, compaction, recall, and adapter decisions

Macaca SHALL expose request-level diagnostics that explain how context sources were rendered, pruned, compacted, recalled, or supplied by adapters, without leaking full sensitive content by default.

#### Scenario: Report explains derived context

- **GIVEN** a model request includes pruned snippets, compaction summaries, memory recall, wiki digest, or external adapter output
- **WHEN** a user or UI requests the context report
- **THEN** the report MUST show source kind, source id, included state, estimated tokens, pruned tokens, trust level, and decision reason
- **AND** the report MUST NOT expose full prompt, full tool output, full memory content, or full external context unless explicit debug capture is enabled

#### Scenario: Report links to retrievable originals

- **GIVEN** a source was summarized, excerpted, or compacted
- **WHEN** the diagnostic report references that derived context
- **THEN** the report SHOULD include a safe source reference or artifact reference for authorized debug retrieval
- **AND** unauthorized clients MUST NOT receive raw sensitive source content through the report endpoint
