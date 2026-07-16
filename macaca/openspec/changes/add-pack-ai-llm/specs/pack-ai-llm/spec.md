## ADDED Requirements

### Requirement: Macaca SHALL provide the AI LLM Pack as a serviceized capability

Macaca SHALL provide `pack.ai.llm.v1` as a provider-neutral industrial pack for chat, completion, routing, policy, budget, tool-call metadata, and model invocation diagnostics. The pack SHALL be declared by applications, resolved by admission/catalog services, and invoked only through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.ai.llm.v1` as required and LLM service provider is registered, healthy, entitled, and policy-admissible
- **THEN** admission SHALL expose `pack.ai.llm.v1` in the effective capability set with command schemas, permission scopes, policy template, health, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing provider secrets or raw provider payloads

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.ai.llm.v1` as required but provider, permission, entitlement, resource, or host support is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.ai.llm.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report
- **AND** SDK command helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: AI LLM Pack commands SHALL use typed canonical service calls

Every `pack.ai.llm.v1` operation SHALL be represented as a typed command/result DTO and SHALL traverse the canonical service runtime path with trace, policy, resource, entitlement, approval, health, snapshot, and structured error behavior.

#### Scenario: Command succeeds through service runtime
- **WHEN** a declared and policy-allowed command such as `llm.chat` is invoked
- **THEN** Macaca SHALL route the command through SDK/facade helpers into the service runtime and LLM service provider
- **AND** it SHALL emit sanitized admission, policy, service-call, result, and replay events with stable trace identifiers

#### Scenario: Command is denied before side effects
- **WHEN** policy, permission, entitlement, approval, or resource checks reject a `pack.ai.llm.v1` command
- **THEN** Macaca SHALL return a typed denied or quota result before invoking the concrete provider
- **AND** the audit trail SHALL include the bounded reason code without raw user data or provider payloads

#### Scenario: Command is unsupported by the active provider
- **WHEN** a descriptor exists but the active provider does not support a requested command
- **THEN** Macaca SHALL return a typed unsupported result with descriptor and provider capability diagnostics
- **AND** SDK discovery SHALL report the command as non-callable for the current effective capability set

### Requirement: AI LLM Pack SHALL expose concrete industrial metadata

`pack.ai.llm.v1` SHALL expose descriptor metadata for command schemas, permission scopes, policy templates, resource budgets, SDK examples, lifecycle state, compatibility, health probes, snapshots, and unavailable diagnostics.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.ai.llm.v1`
- **THEN** it SHALL return the command namespace `llm.*`, supported commands, permissions, policy templates, examples, lifecycle, availability, health, diagnostics, and compatibility metadata
- **AND** examples SHALL use generic handles or synthetic data rather than application-specific workflows

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a `pack.ai.llm.v1` snapshot
- **THEN** the snapshot SHALL include descriptor version, provider class, health, command availability, policy template hash, resource counters, and sanitized replay pointers
- **AND** it SHALL exclude raw secrets, credentials, prompts, manifests, package bytes, private keys, signatures, raw provider payloads, and unbounded output

### Requirement: AI LLM Pack implementation SHALL preserve Macaca boundaries

The `pack.ai.llm.v1` implementation SHALL remain owned by LLM service provider; the microkernel, SDK, shells, and generic application framework SHALL remain provider-neutral and free of application-specific or provider-specific routing branches.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.ai.llm.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class and descriptor metadata rather than provider-specific business branches

### Requirement: AI LLM Pack SHALL model industrial generation requests and streams

`pack.ai.llm.v1` SHALL expose provider-neutral invocation, message, content block, tool-call, structured-output, stream-frame, usage, and budget DTOs.

#### Scenario: Streaming frames are ordered
- **WHEN** `llm.stream_chat` emits partial output, tool-call deltas, usage updates, and finalization
- **THEN** every frame SHALL carry a stable sequence number, trace id, frame kind, and redaction profile
- **AND** replay SHALL reproduce frame order without storing raw prompt or provider payloads

#### Scenario: Cancellation reaches terminal state
- **WHEN** `llm.cancel_generation` targets an active invocation
- **THEN** Macaca SHALL return a structured cancelled, completed, or already-terminal result
- **AND** late provider frames SHALL be ignored or attached as sanitized diagnostics according to the stream contract

#### Scenario: Structured output is validated
- **WHEN** a generation declares a structured-output schema
- **THEN** Macaca SHALL validate the result or return a typed schema-mismatch result
- **AND** the diagnostic SHALL include schema id, validation path, and bounded reason code without raw sensitive output

#### Scenario: Tool call cannot bypass capability policy
- **WHEN** an LLM result contains a tool-call envelope
- **THEN** Macaca SHALL treat it as proposed metadata only until the target tool or capability service validates declaration, permission, policy, and trace context
- **AND** the LLM provider SHALL NOT execute the tool directly through the LLM pack

### Requirement: AI LLM Pack SHALL enforce budget, usage, and provider-neutral capability discovery

`pack.ai.llm.v1` SHALL expose preflight estimation, postflight accounting, and provider-neutral capability descriptors without provider-name routing.

#### Scenario: Budget preflight denies invocation
- **WHEN** `llm.estimate_tokens` or invocation preflight predicts token, cost, rate, or retained-output limits would be exceeded
- **THEN** Macaca SHALL return `quota_exceeded` or `denied` before provider invocation
- **AND** no concrete provider SHALL be called

#### Scenario: Usage is recorded after completion
- **WHEN** an invocation completes successfully or partially
- **THEN** Macaca SHALL record sanitized usage counters, finish reason, latency band, capability class, and budget delta
- **AND** prompts, raw provider responses, and credentials SHALL NOT enter trace, audit, snapshot, or SDK diagnostics

#### Scenario: Capability discovery is provider neutral
- **WHEN** SDK discovery inspects LLM capabilities
- **THEN** Macaca SHALL expose features such as streaming, tool-call support, structured output, multimodal input, and context-window bands
- **AND** OS-layer code SHALL NOT branch on concrete provider or model names
