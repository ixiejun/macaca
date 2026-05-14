## MODIFIED Requirements

### Requirement: Application adapters SHALL produce orchestration commands, not execution semantics

Macaca application adapters, including YAML workflow adapters, WASM ABI adapters, chat adapters, task/goal adapters, SDK adapters, and future gateway adapters, SHALL convert application intent into provider-neutral service commands. They SHALL NOT own independent agent execution paths.

#### Scenario: WASM app delegates agent work

- **GIVEN** a WASM guest invokes `macaca:agent/delegate`
- **WHEN** the WASM host import bridge validates app/session/trace/capability scope
- **THEN** it SHALL produce an `AgentExecutionCommand`
- **AND** dispatch it through `ServiceRuntime`
- **AND** it SHALL NOT dispatch directly to `ApplicationExecutor::delegate_task` as the final semantic execution path.

#### Scenario: Task worker starts agent work

- **GIVEN** a task or goal worker needs agent execution
- **WHEN** the worker claims or receives a task
- **THEN** it SHALL produce `AgentExecutionCommand`
- **AND** rely on `service.agent_execution` for runtime execution
- **AND** keep task state, ordering, cancellation, and result storage separate from agent construction semantics.

### Requirement: ApplicationExecutor SHALL own scheduling and lifecycle, not agent runtime construction

`ApplicationExecutor` SHALL own app-scoped queueing, priority, parallelism, task lifecycle, cancellation, event correlation, and result storage. It SHALL NOT be the semantic owner of context-aware agent runtime construction.

#### Scenario: Delegated task is queued

- **GIVEN** an application delegates work to an agent
- **WHEN** the work is queued through application execution infrastructure
- **THEN** queueing and task status SHALL remain app-scoped
- **AND** runtime execution SHALL proceed through `service.agent_execution`
- **AND** trace and audit SHALL connect the queue event to the service execution event.
