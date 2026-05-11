## ADDED Requirements

### Requirement: Macaca SHALL define typed plugin hook contracts

Macaca SHALL define typed plugin hook contracts for observer, mutating, blocking, and approval hooks, including hook name, hook kind, descriptor, invocation context, result schema, timeout policy, failure policy, priority, and trace metadata.

#### Scenario: Hook descriptor identifies safe semantics

- **WHEN** a plugin declares a hook descriptor
- **THEN** the descriptor SHALL state the hook name, hook kind, priority, timeout policy, failure policy, required permissions, resource hints, and trace schema
- **AND** the descriptor SHALL NOT expose provider-specific implementation details

### Requirement: Macaca SHALL execute plugin hooks through a bounded Hook Bus

Macaca SHALL execute plugin hooks through a runtime-host-owned Hook Bus that applies deterministic priority ordering, timeout policies, failure policies, result validation, structured logs, and trace/audit events.

#### Scenario: Observer hook timeout fails open

- **WHEN** an observer hook exceeds its timeout and its failure policy is fail-open
- **THEN** Hook Bus SHALL log and trace the timeout
- **AND** the host operation SHALL continue without applying a hook mutation

#### Scenario: Blocking hook can require approval

- **WHEN** a blocking hook returns `require_approval`
- **THEN** Hook Bus SHALL return a structured approval decision to the owning service
- **AND** the owning service SHALL route the approval through normal approval policy rather than letting the hook bypass policy

### Requirement: Macaca SHALL expose a core OS hook set

Macaca SHALL expose initial hook points for agent lifecycle, application lifecycle, task lifecycle, tool call, prompt/context build, memory ingest, LLM call, gateway message, approval lifecycle, and session lifecycle.

#### Scenario: Tool call hook is traceable

- **WHEN** a plugin participates in `before_tool_call`
- **THEN** the hook invocation SHALL include trace id, scoped tool metadata, plugin id, hook name, and bounded input metadata
- **AND** the trace/audit record SHALL include duration, decision, status, and structured error code when applicable

### Requirement: Macaca SHALL validate mutating hook results before use

Macaca SHALL validate every mutating hook result against its declared result schema before applying any contribution or rewrite.

#### Scenario: Invalid mutating result is ignored or rejected by policy

- **WHEN** a mutating hook returns a result that does not match the expected schema
- **THEN** Hook Bus SHALL reject that hook result
- **AND** it SHALL follow the hook failure policy
- **AND** it SHALL emit a structured validation failure event

### Requirement: Macaca SHALL protect sensitive data in hook payloads

Hook payloads and trace records SHALL exclude secret values, API keys, private keys, raw provider credentials, package bytes, unbounded raw prompts, and unbounded raw memory bodies.

#### Scenario: LLM hook receives bounded metadata

- **WHEN** a plugin participates in `before_llm_call`
- **THEN** the hook payload SHALL expose bounded metadata needed for policy and routing
- **AND** it SHALL NOT expose raw API keys or unbounded prompt bodies
