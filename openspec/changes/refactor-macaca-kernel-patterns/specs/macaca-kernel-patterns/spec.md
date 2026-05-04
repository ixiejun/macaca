## ADDED Requirements

### Requirement: Canonical Executor Lifecycle Event Construction

Kernel SHALL provide canonical helpers for constructing executor lifecycle events and task results without changing `ExecutorEvent` or `TaskResult` payload shape.

#### Scenario: Completed event preserves fields

- **WHEN** a completed executor event is created through the kernel helper
- **THEN** it contains the original task id, agent name, success flag, output, empty error, artifacts, completion timestamp, and token usage fields compatible with existing consumers.

#### Scenario: Failed result preserves fields

- **WHEN** a failed task result is created through the kernel helper
- **THEN** it contains the original task id, `success=false`, empty output, the error message, empty artifacts, completion timestamp, and no token usage.

### Requirement: Scheduler Factory Preserves Default Scheduling

Kernel SHALL provide a scheduler factory that preserves current `SimpleScheduler` selection behavior by default.

#### Scenario: Default scheduler is simple scheduler compatible

- **WHEN** the default scheduler factory is used
- **THEN** it returns a scheduler compatible with `SimpleScheduler` behavior for the same registry and task fixture.

### Requirement: Explicit Agent Status Transition Policy

Kernel SHALL expose explicit helpers for existing agent activity/status transitions without introducing new lifecycle semantics.

#### Scenario: Thinking and idle transitions remain compatible

- **WHEN** an agent is marked thinking and then idle
- **THEN** its activity and current task fields match the current `AgentStatusTracker` behavior.

### Requirement: Executor Payload Construction Is Kernel-Owned

Application executor and worker code SHALL use kernel-owned lifecycle helpers for constructing task start, completion, failure, and result payloads.

#### Scenario: Application executor emits compatible events

- **WHEN** application executor starts, completes, or fails a delegated task
- **THEN** emitted executor events preserve the same task id, agent, result, and error fields as before the refactor.

### Requirement: Kernel Builder Is Additive

Kernel SHALL provide an additive builder/facade construction entry while keeping `Kernel::new` callable but deprecated.

#### Scenario: Builder matches Kernel::new defaults

- **WHEN** a kernel is built through the new builder with the same config, llm, and tools
- **THEN** registry capacity, scheduler behavior, and initial status behavior match the deprecated `Kernel::new` compatibility entry.

### Requirement: Deprecated Kernel Interfaces Remain Callable

Replaced legacy kernel construction interfaces SHALL remain callable and marked deprecated until upper consumers are migrated.

#### Scenario: Deprecated constructor remains available

- **WHEN** existing code calls `Kernel::new`
- **THEN** the code still compiles
- **AND** the deprecation marker points callers to `KernelBuilder`.
