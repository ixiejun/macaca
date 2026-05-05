## ADDED Requirements

### Requirement: Macaca SHALL provide a Memory Fabric core inside macaca-memory

Macaca SHALL provide Memory Fabric core abstractions inside the existing `macaca-memory` crate. The implementation SHALL use module directories under `macaca-memory/src/` and SHALL NOT introduce additional memory crates for core, index, tools, governance, artifacts, providers, embedding, or vector organization.

#### Scenario: Core abstractions live in macaca-memory

- **GIVEN** the Memory Fabric core is implemented
- **WHEN** a developer inspects the workspace crate list
- **THEN** there SHALL NOT be new crates such as `macaca-memory-core`, `macaca-memory-index`, `macaca-memory-tools`, `macaca-memory-governance`, or `macaca-memory-artifacts`
- **AND** the core abstractions SHALL be available from the existing `macaca-memory` crate

#### Scenario: Modules are organized by responsibility

- **GIVEN** Memory Fabric files grow beyond simple single-file modules
- **WHEN** code is split for maintainability
- **THEN** it SHALL be split under `macaca-memory/src/` by responsibility
- **AND** examples include `core/`, `index/`, `vector/`, `embedding/`, `tools/`, `governance/`, `artifacts/`, and `providers/`

### Requirement: Memory operations SHALL use strong MemoryScope

New Memory Fabric operations SHALL carry a strong `MemoryScope` that identifies application, optional tenant/user, optional session/project, optional agent, namespace, and visibility.

#### Scenario: AgentPrivate operation requires agent scope

- **GIVEN** a write, search, get, delete, prefetch, or status operation has `visibility = AgentPrivate`
- **WHEN** the operation is validated or routed
- **THEN** it SHALL include `application_id`
- **AND** it SHALL include `agent_id` or `agent_name`
- **AND** it SHALL NOT be routed to another agent private store

#### Scenario: SessionShared operation requires session or project scope

- **GIVEN** a memory operation has `visibility = SessionShared`
- **WHEN** the operation is validated or routed
- **THEN** it SHALL include `application_id`
- **AND** it SHALL include `session_id` or `project_id`
- **AND** authorized agents in the same session/project MAY access it through shared routing

#### Scenario: Provider cannot infer scope from globals

- **GIVEN** a memory provider receives a request
- **WHEN** the provider handles the request
- **THEN** it SHALL use the `MemoryScope` supplied in the request
- **AND** it SHALL NOT infer application, agent, session, project, user, or namespace from process-global mutable state

### Requirement: Every agent SHALL have private memory

Macaca SHALL provide each application agent with an `AgentPrivate` memory route that is isolated from other agents by default.

#### Scenario: Agent writes private memory

- **GIVEN** agent `planner` and agent `coder` are in the same application
- **WHEN** `planner` writes a memory with `visibility = AgentPrivate`
- **THEN** the memory SHALL be stored under planner private scope
- **AND** `coder` SHALL NOT retrieve it through default private memory search

#### Scenario: Agent private memory can use existing isolated manager

- **GIVEN** the builtin implementation is selected
- **WHEN** an agent private memory route is created
- **THEN** Macaca MAY adapt the existing `IsolatedMemoryManager`
- **AND** it SHALL preserve per-application and per-agent isolation

### Requirement: Session or project SHALL have shared memory

Macaca SHALL provide `SessionShared` memory for session/project facts, decisions, constraints, and handoff context shared by authorized agents.

#### Scenario: Shared memory is visible to authorized session agents

- **GIVEN** two agents are authorized in the same application session
- **WHEN** one writes a memory with `visibility = SessionShared`
- **THEN** the other MAY retrieve it through session shared routing
- **AND** the result SHALL include provenance identifying source agent/session/turn when available

#### Scenario: Private memory is not automatically shared

- **GIVEN** an agent has `AgentPrivate` memory
- **WHEN** another agent queries `SessionShared` memory
- **THEN** the private memory SHALL NOT appear unless an explicit promotion, user instruction, or policy event copied it into shared scope

### Requirement: MemoryFacade SHALL be the stable upper-layer entry point

Macaca SHALL expose a `MemoryFacade` or equivalent stable facade for upper crates. Upper application, runtime, framework, web, and context integration code SHOULD depend on this facade instead of concrete memory manager implementations.

#### Scenario: Upper layer performs scoped search

- **GIVEN** a framework or runtime component needs memory recall
- **WHEN** it calls the memory system
- **THEN** it SHALL call the facade with a scoped request
- **AND** the facade SHALL route the request according to `MemoryScope` and routing policy

#### Scenario: Existing APIs remain compatible during migration

- **GIVEN** older code still uses existing `MemoryManager`, `IsolatedMemoryManager`, `MemoryStore`, `EmbeddingProvider`, or `VectorStore` APIs
- **WHEN** Memory Fabric core is added
- **THEN** those APIs SHALL remain available during migration
- **AND** replacement paths MAY be marked deprecated only after equivalent facade paths exist

### Requirement: MemoryRouter SHALL route by visibility and policy

Macaca SHALL provide routing that maps scoped operations to agent private memory, session shared memory, application shared memory, user-scoped memory, global system memory, or composite recall according to explicit policy.

#### Scenario: Agent recall uses composite route

- **GIVEN** an agent requests contextual recall for a model turn
- **WHEN** the default recall policy is used
- **THEN** the router SHOULD query `AgentPrivate`
- **AND** it SHOULD query relevant `SessionShared`
- **AND** it MAY query `ApplicationShared`, `UserScoped`, or supplements according to policy and budget

#### Scenario: Explicit route limits search

- **GIVEN** a request explicitly targets `AgentPrivate`
- **WHEN** the router processes the request
- **THEN** it SHALL NOT query `SessionShared`, `ApplicationShared`, or external supplements unless policy explicitly permits expansion
