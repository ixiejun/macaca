## ADDED Requirements

### Requirement: Framework SHALL close AgentScope 2.0 agent-runtime contract gaps

Macaca SHALL expose AgentScope Java 2.0-equivalent agent-runtime behavior through provider-neutral framework contracts, with event-stream execution as the canonical surface and no AgentScope 1.0 runtime fallback.

#### Scenario: Pure event stream is canonical
- **GIVEN** a caller invokes a framework-backed agent
- **WHEN** the run executes
- **THEN** the primary contract SHALL expose callable, streamable, and observable agent behavior through ordered typed `AgentEvent` values
- **AND** final `reply` helpers SHALL be deterministic projections over the same event stream
- **AND** `reply` SHALL NOT own a separate execution path

#### Scenario: Middleware replaces hook-primary execution
- **GIVEN** framework behavior needs pre-call, reasoning, acting, model, tool, system-prompt, trace, or RAG processing
- **WHEN** the behavior is registered
- **THEN** it SHALL register through a middleware-first ABI
- **AND** any remaining consumer-facing hook API SHALL be annotated with a migration note to the canonical middleware contract
- **AND** AgentScope 1.0 hook-primary runtime code SHALL NOT remain as an internal fallback

#### Scenario: User and HITL input agents suspend explicitly
- **GIVEN** an agent requires human input, user confirmation, or streamed user input
- **WHEN** the framework cannot continue autonomously without that input
- **THEN** UserAgent, StreamUserInput, and HITL contracts SHALL emit typed pending-input events
- **AND** durable state SHALL include the resume token, expected input kind, trace id, policy state, and idempotency guard
- **AND** stale or duplicate resume attempts SHALL return structured denied or invalid-resume results

#### Scenario: Stream options and structured output are provider-neutral
- **GIVEN** a caller requests streaming options or structured output
- **WHEN** the framework prepares model execution
- **THEN** StreamOptions and structured-output contracts SHALL describe desired output shape, streaming mode, stop behavior, schema constraints, and validation failure behavior without naming concrete providers
- **AND** unsupported provider behavior SHALL return structured unsupported or unavailable results with trace evidence

#### Scenario: Built-in middleware contracts are generic
- **GIVEN** tracing, task reminders, system prompts, or RAG/context retrieval are enabled
- **WHEN** the agent run enters the middleware pipeline
- **THEN** framework-owned middleware SHALL expose provider-neutral contracts for trace enrichment, reminder injection, system prompt composition, and retrieval augmentation
- **AND** concrete task mutation, memory/vector retrieval, and context service execution SHALL be delegated through approved services

### Requirement: Framework SHALL close AgentScope 2.0 tool, model, MCP, and protocol contract gaps

Macaca SHALL provide provider-neutral contracts for tools, model formatting/transport/error handling, MCP content conversion, and Agent Protocol projection while delegating concrete side effects to services or runtime-host providers.

#### Scenario: ToolBase-first toolkit is canonical
- **GIVEN** a framework agent declares callable tools
- **WHEN** the declaration is serialized, inspected, or invoked
- **THEN** ToolBase, ToolSpec, ToolInvocation, ToolResult, permission metadata, and schema metadata SHALL be the canonical API
- **AND** old toolkit bridge behavior SHALL be non-primary and marked for migration where a public caller still depends on it

#### Scenario: Tool context is injected through a typed contract
- **GIVEN** a tool requires session, task, tenant, application, trace, capability, policy, budget, or resource context
- **WHEN** the framework invokes the tool through a service-backed port
- **THEN** ToolExecutionContext SHALL be injected automatically through a provider-neutral contract
- **AND** tool implementations SHALL NOT read application-specific globals or hardcoded provider/application names

#### Scenario: Tool suspend and external execution are explicit state machines
- **GIVEN** a tool requires user confirmation or external execution
- **WHEN** the framework suspends the run
- **THEN** it SHALL persist a typed pending-tool state with correlation id, idempotency key, requested side effect, policy decision, resume expectations, and trace evidence
- **AND** matching resume input SHALL complete exactly once
- **AND** duplicate, stale, unauthorized, or mismatched results SHALL be rejected with sanitized diagnostics

#### Scenario: Model formatter parity is contract-owned
- **GIVEN** a provider needs AgentScope Java 2.0-equivalent formatting or parsing for model families such as Gemini, Ollama, Kimi, xAI, DeepSeek, or future providers
- **WHEN** the framework prepares or consumes model messages
- **THEN** it SHALL expose formatter/parser strategy contracts over content blocks, tool calls, usage, generate reasons, and structured output
- **AND** concrete model clients, credentials, endpoint selection, and provider-specific transports SHALL remain service/runtime-host owned

#### Scenario: Model transport and exception taxonomy are provider-neutral
- **GIVEN** a model request is delegated to HTTP, WebSocket, streaming, or another transport
- **WHEN** the request succeeds or fails
- **THEN** the framework SHALL use typed transport command/result/stream DTOs and an exception taxonomy covering authentication, bad request, rate limit, not found, permission, timeout, unavailable, internal, provider failure, and unsupported states
- **AND** raw provider payloads SHALL NOT appear in logs, snapshots, or public errors

#### Scenario: MCP content conversion is standardized
- **GIVEN** an MCP tool returns text, resource, image, binary, embedded, or structured content
- **WHEN** the framework converts it for agent consumption
- **THEN** MCP content SHALL map into standard ContentBlock and ToolResult forms with stable ids, mime metadata, bounded payload rules, and sanitized diagnostics
- **AND** concrete MCP transport/runtime execution SHALL remain MCP service owned

#### Scenario: Agent Protocol projection observes AgentEvent
- **GIVEN** an Agent Protocol adapter is enabled
- **WHEN** the framework emits AgentEvent values
- **THEN** the adapter SHALL project those events into typed Agent Protocol messages without calling concrete framework internals
- **AND** projection failures SHALL preserve the original trace chain and return structured protocol conversion errors

### Requirement: Framework SHALL close AgentScope 2.0 Harness contract gaps

Macaca SHALL provide Harness-equivalent workspace, filesystem, sandbox, session, memory, skill, subagent, and plan-mode contracts as neutral specs and delegated ports, not as concrete backend ownership inside `macaca-framework`.

#### Scenario: Workspace reads use two-layer authorization
- **GIVEN** a Harness agent reads workspace context
- **WHEN** the requested data is available from a declared filesystem provider
- **THEN** the framework SHALL read through the filesystem-first contract
- **AND** authorized local fallback SHALL require explicit capability, policy, path, trace, and sandbox checks
- **AND** silent host filesystem fallback SHALL be rejected

#### Scenario: Filesystem specs are complete DTOs
- **GIVEN** an application or agent declares filesystem needs
- **WHEN** the framework validates Harness configuration
- **THEN** local, remote, sandbox, composite, overlay, and baked filesystem specs SHALL be represented as provider-neutral DTOs
- **AND** concrete execution, mounting, path resolution, and host access SHALL be delegated to filesystem/sandbox services or runtime-host providers

#### Scenario: Sandbox specs and states are backend-neutral
- **GIVEN** a Harness agent needs sandbox execution
- **WHEN** Docker, E2B, Kubernetes, Daytona, AgentRun, plugin, remote, mock, or unavailable backends are configured
- **THEN** the framework SHALL represent backend-neutral sandbox specs, lifecycle states, health, unavailable state, and snapshot DTOs
- **AND** it SHALL NOT construct concrete sandbox clients inside `macaca-framework`

#### Scenario: Session tree freshness and restore are mementos
- **GIVEN** a Harness session has parent/child runs, checkpoints, read caches, pending tool states, and freshness markers
- **WHEN** the session is restored after restart or resume
- **THEN** the framework SHALL restore through explicit memento DTOs with tenant/application/session isolation
- **AND** stale freshness markers SHALL trigger structured refresh-required events rather than implicit recomputation with hidden side effects

#### Scenario: Memory maintenance is delegated through contracts
- **GIVEN** Harness memory maintenance, consolidation, or session memory search is requested
- **WHEN** the memory operation runs
- **THEN** framework contracts SHALL expose MemoryConsolidator, maintenance schedule/result DTOs, and session-memory search ports
- **AND** concrete memory stores, vector indexes, RAG services, and knowledge indexing SHALL remain service-owned

#### Scenario: Skill runtime contracts cover resources and conflict resolution
- **GIVEN** Harness skills declare resources, lazy resources, load tools, visibility rules, promotion, or conflicting names
- **WHEN** the framework resolves the skill surface
- **THEN** it SHALL use provider-neutral skill resource, lazy resource, load-tool, visibility, conflict-resolution, promotion, and audit DTO contracts
- **AND** concrete skill package loading, marketplace behavior, encrypted materialization, and repository storage SHALL remain skill service/plugin owned

#### Scenario: Subagent dynamic specs are framework contracts
- **GIVEN** a Harness agent creates or delegates to a subagent
- **WHEN** dynamic spec generation or nested streaming is used
- **THEN** the framework SHALL expose contracts for subagent spec generator, workspace mode, remote stub, stream forwarding choice, nested child event projection, and trace linkage
- **AND** concrete remote execution, task delegation, and provider construction SHALL remain in approved services or runtime-host composition roots

#### Scenario: PlanMode tools do not mutate OS tasks directly
- **GIVEN** Harness plan mode is active
- **WHEN** the agent enters or exits plan mode or uses plan tools
- **THEN** the framework SHALL expose contracts for the three plan tools, programmatic enter/exit, HITL exit semantics, and local plan state
- **AND** Macaca TaskBoard mutation, planning, review, or execution-control changes SHALL occur only through task/execution-control service commands

### Requirement: Framework SHALL report capability parity through evidence-backed health and snapshots

Macaca SHALL maintain an evidence-backed AgentScope 2.0 capability matrix and provider snapshot contract that distinguishes real equivalence, contract-only support, delegated support, missing support, and policy-disabled support.

#### Scenario: Capability status is fine-grained
- **GIVEN** a framework provider reports its AgentScope 2.0 parity
- **WHEN** the health or snapshot API is queried
- **THEN** every documented core, tool, model, MCP, protocol, Harness, middleware, state, and adapter capability SHALL report one of equivalent, contract-only, delegated-verified, delegated-unverified, missing, or unsupported-by-policy
- **AND** overbroad Available statuses SHALL be rejected by contract tests

#### Scenario: Capability evidence is auditable
- **GIVEN** a capability is reported as equivalent or delegated-verified
- **WHEN** a maintainer inspects the provider snapshot
- **THEN** the snapshot SHALL include evidence refs, delegation target, test coverage refs, version/provenance refs, known limitations, and last verification time
- **AND** raw secrets, raw prompts, raw provider payloads, manifests, package bytes, WASM bytes, and unbounded output SHALL be redacted or omitted

#### Scenario: Unavailable providers are explicit
- **GIVEN** an optional model, memory, context, filesystem, sandbox, skill, MCP, protocol, tracing exporter, or RAG provider is absent
- **WHEN** an agent requests the capability
- **THEN** the framework SHALL return structured unavailable, unsupported, or denied results with trace evidence
- **AND** it SHALL NOT panic, hang, silently fall back to host behavior, or fake success

### Requirement: Framework SHALL enforce Macaca OS boundaries and upgrade hygiene

Macaca SHALL implement these AgentScope 2.0 parity contracts without application-specific behavior, provider hardcoding, kernel ownership leakage, or AgentScope 1.0 canonical code.

#### Scenario: Concrete providers remain outside framework contracts
- **GIVEN** framework parity implementation needs LLM, memory, context, vector retrieval, filesystem, sandbox, skills, MCP, task, gateway, tracing exporter, or protocol execution
- **WHEN** implementation code is reviewed
- **THEN** concrete clients and privileged side effects SHALL be owned by services, plugins, optional modules, or runtime-host composition roots
- **AND** `macaca-framework` SHALL depend on provider-neutral ports, DTOs, adapters, and null/unavailable behavior only

#### Scenario: No application-specific hardcoding is introduced
- **GIVEN** framework source, tests, and configuration are scanned
- **WHEN** generic OS-layer code is inspected
- **THEN** it SHALL NOT branch on application names, workflow names, provider names, model names, driver names, gateway names, payment names, chain names, or business-domain labels
- **AND** test fixtures SHALL use neutral names unless testing explicit user-provided configuration parsing

#### Scenario: AgentScope 1.0 code does not survive as canonical behavior
- **GIVEN** canonical framework production code is scanned
- **WHEN** the AgentScope 2.0 parity implementation is claimed complete
- **THEN** AgentScope 1.0 runtime code SHALL be removed rather than moved to internal legacy, compat, or deprecated runtime fallbacks
- **AND** canonical names SHALL NOT use version suffixes such as ReActAgent2, AgentRuntime2, AgentScope2RuntimeProvider, or equivalent naming that makes AgentScope 2.0 give way to AgentScope 1.0

#### Scenario: Observability and comments are required for new code
- **GIVEN** new or materially rewritten Rust code is added for these contracts
- **WHEN** maintainers review it
- **THEN** non-obvious state transitions, adapters, middleware, and side-effect boundaries SHALL have clear English comments explaining function and operating principles
- **AND** key execution nodes SHALL emit bounded structured logs and replayable trace/audit events

#### Scenario: AgentScope Apache-2.0 provenance is preserved
- **GIVEN** a source file is derived from or closely adapted from AgentScope Java 2.0 concepts, structures, APIs, or behavior
- **WHEN** license checks run
- **THEN** the file SHALL include an Apache-2.0 SPDX/provenance header or approved notice
- **AND** repository notice documentation SHALL identify AgentScope Java 2.0 as an upstream Apache-2.0 source
