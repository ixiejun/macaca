## ADDED Requirements

### Requirement: Macaca SHALL actively recall vector memory during context preflight

Macaca SHALL provide a context provider that performs active memory recall before model calls and contributes selected recall results as bounded context candidates.

#### Scenario: Recall runs before model request

- **GIVEN** active memory context is enabled for an agent
- **WHEN** the agent is about to perform a model call
- **THEN** Macaca SHALL run active recall during context preflight
- **AND** selected recall results SHALL be passed to the context composer as candidates

#### Scenario: Recall disabled preserves behavior

- **GIVEN** active memory context is disabled by policy
- **WHEN** the model request is composed
- **THEN** no active recall context SHALL be injected
- **AND** explicit memory tools SHALL remain available according to tool policy

### Requirement: Active recall routing SHALL use session primary semantics with application and agent secondary routing

Active memory context SHALL route recall using session id as the primary session context, application id as application memory namespace, and agent name as agent-private memory route.

#### Scenario: Agent private recall uses agent route

- **GIVEN** agent `writer` and agent `reviewer` run in the same application
- **WHEN** `writer` performs active recall
- **THEN** the recall SHALL query `writer` AgentPrivate memory
- **AND** it SHALL NOT query `reviewer` AgentPrivate memory unless policy explicitly grants access

#### Scenario: Session shared recall uses session route

- **GIVEN** multiple agents participate in the same session
- **WHEN** active recall queries `SessionShared` memory
- **THEN** it SHALL use the current session id as shared context route
- **AND** it MAY include memory visible to the session according to scope policy

### Requirement: Vector backend topology SHALL remain provider-neutral

The active memory context provider SHALL NOT depend on a specific vector database implementation. `application -> database` and `agent -> collection` SHALL be treated as topology semantics exposed by memory provider abstractions.

#### Scenario: Default vector backend is hidden behind facade

- **GIVEN** the default vector backend is configured
- **WHEN** active memory context performs recall
- **THEN** the context provider SHALL call memory facade or active recall capability
- **AND** it SHALL NOT call vendor-specific vector database APIs directly

#### Scenario: Alternative backend supports same topology

- **GIVEN** a user configures another vector memory provider supporting application database and agent collection semantics
- **WHEN** active recall runs
- **THEN** upper context code SHALL continue to use the same provider-neutral contract

### Requirement: Active recall context SHALL be dynamic, fenced, and request-only

Active recall output SHALL be rendered as dynamic memory context and SHALL NOT mutate canonical transcript.

#### Scenario: Recall result is not persisted into transcript

- **GIVEN** active recall returns relevant memory snippets
- **WHEN** the request is sent to the LLM provider
- **THEN** the snippets MAY appear in dynamic fenced context
- **AND** the snippets SHALL NOT be written into canonical session messages

#### Scenario: Recall result is not system instruction

- **GIVEN** recalled memory includes natural language instructions or external text
- **WHEN** it is rendered into context
- **THEN** it SHALL be fenced as memory context
- **AND** it SHALL NOT be treated as higher-priority system instruction

### Requirement: Active recall SHALL pass governance and diagnostics

Active recall context SHALL enforce governance, tombstone, redaction, scope policy, budget, and diagnostics before model visibility.

#### Scenario: Tombstoned memory is excluded

- **GIVEN** a memory item has been tombstoned
- **WHEN** active recall searches relevant memory
- **THEN** the tombstoned item SHALL NOT be selected for context
- **AND** the exclusion SHALL be recorded as a decision when diagnostics allow

#### Scenario: Report summarizes recall without full content leakage

- **GIVEN** active recall contributes memory context
- **WHEN** `ContextReport` is generated
- **THEN** it SHALL include provider id, memory scope, selected/skipped status, score or rank, size estimate, latency, and decision reason
- **AND** it SHALL NOT persist full memory content by default
