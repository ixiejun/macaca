## ADDED Requirements

### Requirement: Phase completion status SHALL match runtime reality

Macaca SHALL maintain a Phase status artifact for context engineering where `Phase 0-10` completion reflects actual contract, runtime, diagnostics, and verification evidence rather than stale audit snapshots.

#### Scenario: Status matrix is updated after runtime completion

- **GIVEN** a Phase has gained new runtime integration, diagnostics, or tests
- **WHEN** maintainers update the context-engine OpenSpec artifacts
- **THEN** the Phase status matrix SHALL be updated to reflect the current state
- **AND** a Phase SHALL NOT remain marked partial once its four evidence categories are satisfied

#### Scenario: Phase remains incomplete when any evidence category is missing

- **GIVEN** a contract or implementation exists
- **WHEN** runtime wiring, diagnostics, or verification is still missing
- **THEN** the Phase SHALL remain non-complete in the status matrix
- **AND** the missing category SHALL be identified explicitly

#### Scenario: Original research phases map to closure phases

- **GIVEN** original research `Phase 2`, `Phase 3`, or `Phase 4` has foundation code but incomplete product closure
- **WHEN** maintainers inspect the Phase status artifact
- **THEN** the artifact SHALL map those gaps to the corresponding closure phase
- **AND** `Phase 2` pruning closure SHALL map to `Phase 6`
- **AND** `Phase 3` compaction/lineage closure SHALL map to `Phase 7`
- **AND** `Phase 4` memory/wiki recall closure SHALL map to `Phase 8`

### Requirement: Phase 6 SHALL preserve retrievable originals for every pruned source

Macaca SHALL treat pruning as non-destructive for every supported source kind. Any source rendered as excerpt, summary, or dropped content in model-visible context MUST preserve its original payload in canonical storage and MUST remain retrievable through a source reference or an authorized debug path.

#### Scenario: Pruned tool or command output remains retrievable

- **GIVEN** a large tool result, command stdout/stderr, or search result is pruned before entering model context
- **WHEN** a maintainer or debug client follows the `ContextReport` source reference
- **THEN** Macaca SHALL resolve the reference to the original canonical payload or an explicit authorized retrieval endpoint
- **AND** the pruning step SHALL NOT rewrite or replace the original payload in EventLog, artifact storage, or session storage

#### Scenario: Pruned file or trace payload remains retrievable

- **GIVEN** a file read, trace event, or other large source is rendered as excerpt or summary
- **WHEN** diagnostics are inspected
- **THEN** the `ContextReport` SHALL expose a stable source ref or artifact ref for that source
- **AND** the original payload SHALL remain available through canonical storage if the operator is authorized

#### Scenario: Source retrieval rejects cross-session references

- **GIVEN** a context report source ref belongs to session `A`
- **WHEN** a client attempts to resolve it through session `B`
- **THEN** Macaca SHALL reject the lookup
- **AND** the response SHALL NOT reveal the source payload or whether the source exists outside the authorized scope

#### Scenario: Source retrieval reports unavailable originals explicitly

- **GIVEN** an old or unsupported pruned source has no stable canonical original payload
- **WHEN** diagnostics render that source
- **THEN** Macaca SHALL report an explicit unavailable reason
- **AND** it SHALL NOT claim that the original is retrievable

#### Scenario: Retrieval uses repository or adapter boundary

- **GIVEN** UI or web diagnostics need to fetch a pruned source payload
- **WHEN** the payload is requested
- **THEN** Macaca SHALL resolve it through a scoped retrieval repository or adapter boundary
- **AND** UI or context assembly code SHALL NOT directly construct backend storage keys

### Requirement: Phase 7 SHALL provide lineage-aware logical session UX

Macaca SHALL expose compaction lineage as a first-class runtime capability. Logical session access SHALL resolve to the current lineage tip by default, while diagnostics and UI SHALL allow operators to inspect the root-to-tip lineage without losing auditability.

#### Scenario: Logical session access resolves to lineage tip

- **GIVEN** a root session has one or more compaction successor nodes
- **WHEN** runtime or web code loads the logical session for normal operation
- **THEN** Macaca SHALL resolve the request to the current lineage tip by default
- **AND** the original lineage root and predecessor chain SHALL remain queryable

#### Scenario: UI shows lineage and compaction details

- **GIVEN** a session has been compacted manually or automatically
- **WHEN** a user opens the chat trace or diagnostics UI
- **THEN** the UI SHALL show root session id, tip session id, successor chain, and compaction summary metadata
- **AND** a debug interaction SHALL allow root-to-tip lineage expansion without changing the default logical-session view

#### Scenario: Session list remains logical after compaction

- **GIVEN** a session has compaction successors
- **WHEN** the user opens the normal session list
- **THEN** Macaca SHALL present one logical session entry for that lineage
- **AND** successor nodes SHALL NOT appear as duplicate normal sessions unless debug lineage expansion is requested

#### Scenario: Compaction summary is reference-only

- **GIVEN** a compaction summary is inserted into the successor context
- **WHEN** the summary is rendered for a model request
- **THEN** it SHALL be fenced or otherwise marked as reference-only context
- **AND** it SHALL NOT be treated as a new user instruction

### Requirement: Phase 8 SHALL provide full runtime memory and wiki recall injection

Macaca SHALL support memory recall and wiki/digest recall as optional runtime context sources. These recall outputs MUST be bounded, untrusted, request-only, and explicitly described in `ContextReport`.

#### Scenario: Wiki or digest recall is runtime-selectable

- **GIVEN** wiki/digest recall is enabled through config, profile, or explicit tool/source-provider path
- **WHEN** a model request is assembled
- **THEN** Macaca SHALL retrieve wiki/digest recall through the context runtime path
- **AND** the result SHALL be included only as dynamic, untrusted, request-only context

#### Scenario: Recall carries provenance and privacy metadata

- **GIVEN** memory recall, wiki recall, preflight recall, or active recall returns one or more context items
- **WHEN** those items are injected into a model request
- **THEN** each injected item SHALL carry provenance, confidence, privacy tier, and source id metadata
- **AND** the canonical transcript SHALL NOT be mutated by that injection

#### Scenario: Recall provider cannot mutate transcript

- **GIVEN** a memory or wiki/digest context provider contributes recall candidates
- **WHEN** the provider is invoked during model request assembly
- **THEN** it SHALL receive request metadata and source query inputs only
- **AND** it SHALL NOT receive a mutable reference to the canonical transcript

#### Scenario: Recall diagnostics may persist without duplicating recall body as transcript

- **GIVEN** recall output is injected into a model request
- **WHEN** Macaca records diagnostics
- **THEN** diagnostics MAY store source ids, evidence ids, warnings, budgets, and bounded summaries
- **AND** the injected full recall body SHALL NOT be appended as a canonical transcript message

#### Scenario: Digest and raw memory duplication is explained

- **GIVEN** raw memory recall and wiki/digest recall both match the same evidence
- **WHEN** the context composer selects model-visible candidates
- **THEN** Macaca SHALL apply a deterministic selection strategy or budget policy
- **AND** any suppressed duplicate SHALL be visible as a context report decision

#### Scenario: Recall is invisible by default

- **GIVEN** recall is not enabled and no recall tool or provider path is invoked
- **WHEN** a model request is assembled
- **THEN** Macaca SHALL NOT load memory or wiki content into context by default
- **AND** no recall source SHALL appear in `ContextReport`

### Requirement: Phase 9 SHALL provide installable custom engines and safe external adapters

Macaca SHALL allow operators to install custom in-process context engines/providers and SHALL provide a safe adapter boundary for process, RPC, or WASM-backed external context managers.

#### Scenario: Custom in-process engine is registered and selected by config

- **GIVEN** an operator registers a custom in-process context engine or provider that passes the published conformance checks
- **WHEN** system, application, or agent profile configuration selects that implementation
- **THEN** runtime, framework, and web code SHALL invoke it only through public context abstractions
- **AND** no application logic changes SHALL be required to activate it

#### Scenario: External adapter output is validated and bounded

- **GIVEN** an external process, RPC service, or WASM adapter returns assembled context or diagnostics
- **WHEN** Macaca receives the adapter output
- **THEN** Macaca SHALL enforce timeout, payload size, schema validation, trust fencing, and fallback policy before the output reaches the LLM request
- **AND** malformed or oversized output SHALL degrade safely without crashing the main loop

#### Scenario: Custom engine selection follows explicit precedence

- **GIVEN** system, application, and agent profile config each specify context engine preferences
- **WHEN** Macaca selects a context engine for a model request
- **THEN** it SHALL follow documented precedence
- **AND** it SHALL record the selected engine id and any fallback decision in diagnostics

#### Scenario: External adapter is opt-in

- **GIVEN** no external context adapter is explicitly configured
- **WHEN** Macaca boots runtime, framework, or web context paths
- **THEN** no external process, RPC service, or WASM context adapter SHALL be started
- **AND** builtin context engines/providers SHALL remain available

#### Scenario: External adapter output defaults to untrusted

- **GIVEN** an external adapter returns context candidates
- **WHEN** Macaca validates those candidates
- **THEN** candidates SHALL default to untrusted unless a trusted local policy explicitly promotes them
- **AND** untrusted candidates SHALL be fenced before model visibility

### Requirement: Phase 10 SHALL enforce migration and archive discipline

Macaca SHALL complete context-engine migration only when deprecated legacy prompt/context entry points remain searchable, all production consumers have migrated away from them, and verification/archival gates have been satisfied.

#### Scenario: Deprecated legacy entry remains searchable but unused by production

- **GIVEN** a prompt or context entry point has been replaced by `ContextFacade`, `ContextRuntimeFacade`, or equivalent abstractions
- **WHEN** migration reaches final closure
- **THEN** the old entry point SHALL remain in the codebase with a deprecated marker and replacement guidance
- **AND** no non-test production code SHALL continue to call it

#### Scenario: Archive gates require evidence

- **GIVEN** maintainers want to archive context-engine related changes
- **WHEN** they evaluate readiness
- **THEN** OpenSpec validation, targeted tests, Phase status updates, and available GitNexus impact/change evidence SHALL all be complete
- **AND** a change SHALL NOT be archived while runtime or diagnostics gaps remain open for any claimed-complete Phase

#### Scenario: Deprecated API remains for migration search

- **GIVEN** a legacy prompt/context API is replaced
- **WHEN** maintainers search for migration targets
- **THEN** the legacy API SHALL remain present with replacement guidance
- **AND** it SHALL NOT be deleted solely to make scans pass

#### Scenario: New closure code remains application-generic

- **GIVEN** new code is added for context phase closure
- **WHEN** maintainers run final scans or review implementation
- **THEN** the code SHALL NOT branch on hardcoded application names, workflow names, driver names, provider names, or business-specific identifiers
- **AND** behavior selection SHALL come from config, manifest, profile, or registered provider capabilities
