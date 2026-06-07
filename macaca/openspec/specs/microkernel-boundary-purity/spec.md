# microkernel-boundary-purity Specification

## Purpose
Constitutional microkernel surface and dependency invariants for Macaca Agent OS. Established by refactor-unified-call-path-microkernel (2026-06-07).

## Requirements
### Requirement: Kernel Holds Only System Invariants

The microkernel (`macaca-kernel`) SHALL contain only system invariants: identity, service registry, capability registry, IPC/service-call facade, policy facade, trace/audit bus, scheduler primitive, resource manager facade, session/task state contracts, package runtime guard, and the provider-neutral `AgentExecutionPort` abstraction. The kernel SHALL NOT contain Web3, EVM, payment/A2A, planner/worker-loop execution, or provider compatibility implementations.

#### Scenario: Kernel module surface excludes non-kernel capabilities
- **WHEN** `macaca-kernel/src/lib.rs` is inspected
- **THEN** it SHALL NOT declare or export `web3`, `evm`, `a2a`, `payment_policy`, `provider_compat`, or an `executor` worker-loop module
- **AND** it SHALL only expose system-invariant primitives and the agent execution port

#### Scenario: Kernel constructs no concrete providers
- **WHEN** kernel code is reviewed
- **THEN** it SHALL NOT construct concrete LLM, tool, driver, skill, MCP, payment, web3, or EVM providers
- **AND** provider construction SHALL occur only in the approved runtime-host composition root

### Requirement: Non-Kernel Capabilities Are Serviceized Or Modularized

Web3, EVM, payment/A2A, and agent-execution orchestration SHALL live as system services or optional modules behind the canonical service path, each with descriptor, lifecycle, health, typed command/result/error, policy/trace/audit, and built-in/plugin/remote/mock/unavailable replacement.

#### Scenario: Optional module absence degrades structurally
- **WHEN** an optional module (web3, EVM, or a payment provider) is absent
- **THEN** the base OS SHALL still start, execute tasks, recover sessions, and answer audit queries
- **AND** calls to the absent capability SHALL return a structured unavailable/disabled/denied state without crash, hang, silent fallback, or fake success

### Requirement: Kernel Dependency Purity

`macaca-kernel` SHALL depend only on `macaca-proto` and `macaca-ipc`. It SHALL NOT depend on application-framework, facade, presentation, or concrete service-provider crates.

#### Scenario: Kernel dependency tree is minimal
- **WHEN** `cargo tree -e normal -p macaca-kernel --depth 1` is evaluated
- **THEN** the only internal workspace dependencies SHALL be `macaca-proto` and `macaca-ipc`
- **AND** the dependency gate SHALL contain zero kernel allowlist rows

### Requirement: Foundation Persistence Independence

`macaca-persist` (foundation layer) SHALL NOT depend on `macaca-context` (service layer). Shared persistence contracts SHALL live in `macaca-proto` or be expressed by inverting the dependency direction.

#### Scenario: Persistence does not depend on context service
- **WHEN** `cargo metadata --no-deps` is evaluated
- **THEN** there SHALL be no direct dependency edge `macaca-persist -> macaca-context`
- **AND** persistence and context unit tests SHALL pass
