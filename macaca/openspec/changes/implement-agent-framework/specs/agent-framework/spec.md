## ADDED Requirements

### Requirement: Agent Framework Core Primitives
Macaca SHALL provide provider-neutral agent framework primitives for structured
messages, content blocks, agent identity, session identity, and selective state
serialization.

#### Scenario: Structured message is serialized and restored
- **GIVEN** an agent framework message containing typed content blocks and metadata
- **WHEN** the message is serialized and restored through the framework contract
- **THEN** block type, role, identity, metadata, and timestamp information remain deterministic and provider-neutral

### Requirement: Agent Lifecycle Abstraction
Macaca SHALL expose an asynchronous agent abstraction for reply, observe, and
interrupt behavior without coupling the framework to concrete LLM, tool,
workflow, application, driver, gateway, or business implementations.

#### Scenario: Agent reply runs through a generic lifecycle
- **GIVEN** an agent implementation that satisfies the framework abstraction
- **WHEN** a caller asks the agent to reply to a message
- **THEN** the framework invokes the agent through typed lifecycle methods and returns a structured message or structured error

### Requirement: Extensible Agent Hooks
Macaca SHALL provide hook and wrapper extension points for pre/post lifecycle
behavior so tracing, auditing, policy, cancellation, and shell adapters can be
added without rewriting concrete agent implementations.

#### Scenario: Reply hooks decorate execution
- **GIVEN** an agent wrapped with pre-reply and post-reply hooks
- **WHEN** the agent handles a reply request
- **THEN** hooks run in deterministic order around the inner agent and failures are returned as structured errors

### Requirement: Provider-Neutral Model Formatting
Macaca SHALL separate internal agent messages from external model wire formats
through formatter and model adapter boundaries.

#### Scenario: Model provider receives formatted messages
- **GIVEN** an internal framework message list and a model adapter
- **WHEN** the framework calls a model
- **THEN** the adapter formats messages through a formatter and parses the response back into framework response types

### Requirement: Framework Memory and Toolkit Boundaries
Macaca SHALL provide working-memory and toolkit abstractions that support
tagged messages, tool registration, grouped tools, middleware, and structured
tool responses without embedding application-specific behavior.

#### Scenario: Tool execution is mediated by the toolkit
- **GIVEN** a registered tool with schema, handler, group, and middleware
- **WHEN** an agent requests a tool call through the toolkit
- **THEN** middleware and handler execution produce a structured tool response with auditable metadata

### Requirement: Framework Orchestration Primitives
Macaca SHALL provide reusable orchestration primitives for sequential,
fan-out, message-hub, planning-notebook, session, and tracing behavior while
leaving OS-level task scheduling and persistence ownership outside the
framework.

#### Scenario: Orchestration primitive runs without owning OS scheduling
- **GIVEN** a caller composes agents with a framework orchestration primitive
- **WHEN** the primitive executes
- **THEN** it coordinates framework messages and agents without replacing kernel, task-board, or shell ownership

