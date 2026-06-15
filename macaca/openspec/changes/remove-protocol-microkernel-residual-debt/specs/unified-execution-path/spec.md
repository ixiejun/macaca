## ADDED Requirements

### Requirement: Protocol Call Path SHALL Be Exclusive At Terminal State

Macaca OS SHALL treat the protocol/service path as the only production path for any capability side effect. Public SDK clients, presentation shells, application adapters, WASM host imports, plugins, and gateways SHALL NOT call in-process kernel, framework, runtime manager, provider, or task-loop implementations directly.

#### Scenario: Application shape does not select an alternate path
- **WHEN** YAML, WASM, GenUI, headless, gateway, or optional-module application code invokes an OS capability
- **THEN** the invocation SHALL enter through `SystemFacade` or a focused SDK client
- **AND** the invocation SHALL continue through `ServiceRouter.route`, `ServiceRuntime.call`, `ServiceBus`, `SystemServiceBusHandler`, `ServiceCallExecutor`, and `SystemService.call`
- **AND** application type SHALL NOT select a separate backend

#### Scenario: Direct in-process path is rejected
- **WHEN** production code outside the protocol endpoint or service runtime constructs an in-process kernel/framework/provider call for an OS capability
- **THEN** a terminal static gate SHALL fail with file, line, symbol, and canonical service-client replacement guidance

### Requirement: Historical Route Labels SHALL Not Drive Runtime Behavior

Runtime behavior, docs for active architecture, and production tests SHALL use stable protocol/microkernel/service terminology. Historical migration route labels SHALL NOT appear as active runtime path names, module names, function names, or test names.

#### Scenario: Active code contains no route-migration vocabulary
- **WHEN** terminal debt scans run over production and integration-test Rust source
- **THEN** they SHALL report zero old-route vocabulary matches that describe active runtime behavior
- **AND** any historical mention SHALL be confined to archived design records or this active OpenSpec proposal until it is archived

### Requirement: Old Chat Route SHALL Be Deleted

Macaca Web SHALL expose `/api/chat/v2` as the only production chat execution route. The old chat route implementation and re-export SHALL be deleted rather than left as a deprecated wrapper.

#### Scenario: Chat v2 is the only production chat path
- **WHEN** route modules are inspected
- **THEN** production code SHALL register and export only the `/api/chat/v2` chat execution path
- **AND** any caller of the old symbol SHALL be migrated to the v2 route/client before deletion

#### Scenario: Deleted route is not replaced by wrapper
- **WHEN** the old route symbol is searched after migration
- **THEN** production and integration-test Rust source SHALL contain no deprecated wrapper or allow-deprecated caller for that route

## MODIFIED Requirements

### Requirement: Unified Agent Execution Service Ownership

Macaca OS SHALL own "run one agent" semantics exclusively in the Agent Execution system service. The microkernel SHALL only hold provider-neutral execution identity and typed service-call primitives; presentation shells SHALL NOT construct framework agents, build runtime agents, hold execution loops, or execute models/tools directly.

#### Scenario: Kernel delegates agent execution through service primitives
- **WHEN** the kernel is asked to execute or track a registered agent
- **THEN** it SHALL record provider-neutral identity/state evidence and delegate side-effecting execution to the Agent Execution service path
- **AND** it SHALL NOT hold provider handles, framework runner handles, prompt-routing logic, tool parsers, or task delegation implementations

#### Scenario: Shell does not construct framework agents
- **WHEN** a presentation shell handles chat, delegation, resume, YAML, WASM, or GenUI execution
- **THEN** it SHALL call the facade/client and only adapt transport DTOs or subscribe to events
- **AND** it SHALL NOT invoke framework construction APIs, model providers, tool providers, task loops, or local execution wakers
