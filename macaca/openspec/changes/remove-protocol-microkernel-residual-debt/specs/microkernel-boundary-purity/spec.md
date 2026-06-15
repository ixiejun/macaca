## ADDED Requirements

### Requirement: Kernel SHALL Not Own Network Or Alert Transport Providers

The microkernel SHALL NOT depend on network/http client crates and SHALL NOT implement webhook, remote alert delivery, notification transport, retry transport, or concrete alert provider behavior. Kernel alert responsibilities SHALL be limited to provider-neutral identity, severity, policy, trace, and audit evidence.

#### Scenario: Kernel network transport is rejected
- **WHEN** `macaca-kernel` dependency metadata or production source is scanned
- **THEN** the gate SHALL fail on network/http client dependencies, webhook sender implementations, remote alert transport code, or concrete notification providers
- **AND** diagnostics SHALL direct maintainers to the alert/notification system service provider

#### Scenario: Alert provider absence is structured
- **WHEN** no alert/notification provider is registered
- **THEN** alert side effects SHALL return a structured unavailable state through the service path
- **AND** kernel trace/audit evidence SHALL still record the attempted provider-neutral alert event without fabricating delivery success

### Requirement: Kernel SHALL Not Own Agent Or Task Orchestration Semantics

The microkernel SHALL NOT contain agent/task orchestration modules, agent matching, task delegation command parsing, prompt keyword routing, worker-loop implementations, tool-name parsers, or result aggregation behavior. Those behaviors SHALL live in task, execution-control, agent-execution, or application-framework services.

#### Scenario: Orchestration module is rejected
- **WHEN** `macaca-kernel/src/lib.rs` and kernel production modules are inspected
- **THEN** no production module or export SHALL provide agent/task orchestration behavior
- **AND** any delegation, matching, parsing, or aggregation behavior SHALL be owned by service-layer commands and providers

#### Scenario: Tool command parsing is not kernel behavior
- **WHEN** a tool command such as delegation or aggregation is executed
- **THEN** parsing and execution SHALL occur through typed service commands outside the kernel
- **AND** the kernel SHALL only record trace-required service-call evidence

## MODIFIED Requirements

### Requirement: Kernel Holds Only System Invariants

The microkernel (`macaca-kernel`) SHALL contain only system invariants: identity, service registry, capability registry, IPC/service-call facade, policy facade, trace/audit bus, scheduler primitive, resource manager facade, session/task state contracts, package runtime guard, and provider-neutral service-call primitives. The kernel SHALL NOT contain network transports, alert delivery providers, agent/task orchestration, Web3, EVM, payment/A2A, planner/worker-loop execution, provider compatibility implementations, framework construction, or provider-specific logic.

#### Scenario: Kernel module surface excludes non-kernel capabilities
- **WHEN** `macaca-kernel/src/lib.rs` is inspected
- **THEN** it SHALL NOT declare or export modules for concrete optional modules, provider compatibility, alert transports, agent orchestrators, tool parsers, or worker-loop implementations
- **AND** it SHALL only expose system-invariant primitives and typed service-call boundaries

#### Scenario: Kernel constructs no concrete providers
- **WHEN** kernel code is reviewed
- **THEN** it SHALL NOT construct concrete LLM, tool, driver, skill, MCP, payment, web3, EVM, alert, notification, framework, or runtime-host providers
- **AND** provider construction SHALL occur only in approved runtime-host/service composition roots
