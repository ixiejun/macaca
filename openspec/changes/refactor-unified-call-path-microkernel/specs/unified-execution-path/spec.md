## ADDED Requirements

### Requirement: Single Protocol Call Path For All Service Capabilities

Macaca OS SHALL route every service-capability invocation (LLM, tool, driver, skill, MCP, task, memory, context, agent execution, payment, web3, EVM, gateway) through one canonical protocol path: `SystemFacade`/SDK client → `ServiceRouter.route` → `ServiceRuntime.call` → `ServiceBus` → `SystemServiceBusHandler` → `ServiceCallExecutor` → `SystemService.call`. No production code path SHALL invoke a provider directly outside this chain.

#### Scenario: Capability invocation flows through the canonical path
- **WHEN** any production code invokes a service capability
- **THEN** the call SHALL be expressed as a typed `ServiceCommand` and dispatched through `ServiceRouter`/`ServiceRuntime`
- **AND** it SHALL carry a `TraceContext` and be rejected if trace is missing
- **AND** it SHALL produce a replayable service-call audit event

#### Scenario: Direct provider bypass is rejected
- **WHEN** production code attempts to call an LLM, tool, driver, skill, MCP, payment, web3, or EVM provider without going through the canonical service path
- **THEN** the no-direct-provider-call audit gate SHALL fail with file, line, capability, and replacement service-client guidance

### Requirement: Unified Agent Execution Service Ownership

Macaca OS SHALL own "run one agent" semantics in a single Agent Execution system service. The microkernel SHALL only hold a provider-neutral `AgentExecutionPort` abstraction whose sole production implementation delegates to the Agent Execution service; presentation shells SHALL NOT construct framework agents or execute models/tools directly.

#### Scenario: Kernel delegates agent execution to the port
- **WHEN** the kernel is asked to execute a registered agent
- **THEN** it SHALL delegate to `AgentExecutionPort`, record start/finish/unavailable logs, and preserve status transitions
- **AND** it SHALL NOT read LLM or tool provider handles directly

#### Scenario: Shell does not own execution semantics
- **WHEN** a presentation shell handles a chat or delegated execution request
- **THEN** it SHALL call the Agent Execution service through the facade and only adapt SSE/HTTP DTOs
- **AND** it SHALL NOT build a framework agent or call model/tool providers in shell code

### Requirement: All Application Types Converge To One Path

Macaca OS SHALL execute YAML, WASM, GenUI, and headless applications through the same Application ABI and the same canonical service path. Application type SHALL NOT select a separate execution backend.

#### Scenario: YAML and WASM produce one execution chain
- **WHEN** a YAML application and a WASM application each run one agent execution
- **THEN** service-call audit replay by session id SHALL show exactly one execution chain per run through the canonical path
- **AND** both SHALL reuse the same trace/audit correlation and replay path

### Requirement: Removal Of Multi-Path Reconciliation Markers

Once execution paths are unified, Macaca OS SHALL remove the multi-path reconciliation markers (`graph_owner`/`execution.graph_owner` discrimination, `authoritative`/`non_authoritative`/`legacy_unmarked` classification, `suppress_executor_lifecycle`, `legacy_chat_main_thread_goal_pause`). Terminal-state determination SHALL treat all host commands as equally authoritative.

#### Scenario: No reconciliation markers remain in production
- **WHEN** the codebase is scanned after path convergence
- **THEN** production code SHALL contain zero occurrences of `legacy_unmarked`, `non_authoritative`, `suppress_executor_lifecycle`, and `legacy_chat_main_thread_goal_pause`
- **AND** terminal completion/failure SHALL be computed without authoritative-vs-compat branching

#### Scenario: Markers are removed only after replay proves a single chain
- **WHEN** a reconciliation marker is proposed for removal
- **THEN** removal SHALL proceed only if audit replay first proves the related capability resolves through a single canonical chain
- **AND** the change SHALL keep deterministic terminal-state semantics
