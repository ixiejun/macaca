## ADDED Requirements

### Requirement: Upper crates use KernelBuilder for kernel construction

Upper production crates SHALL construct kernels through `KernelBuilder` instead of deprecated `Kernel::new`.

#### Scenario: Web startup constructs kernel

- **WHEN** the web application initializes kernel state
- **THEN** it constructs the kernel through `KernelBuilder`
- **AND** LLM provider, tools, app registry, session, trace, and task behavior remain unchanged.

#### Scenario: CLI commands construct kernel

- **WHEN** CLI commands need a kernel
- **THEN** they construct the kernel through a builder-backed helper
- **AND** command output and startup behavior remain unchanged.

### Requirement: Deprecated kernel APIs remain definitions only for consumers

Deprecated kernel construction APIs SHALL remain callable but existing upper consumer call sites SHALL migrate to additive kernel primitives.

#### Scenario: Existing consumer call sites are migrated

- **WHEN** source code is audited for existing `Kernel::new` consumer calls
- **THEN** production upper crates and regular upper tests no longer call `Kernel::new`
- **AND** the deprecated constructor definition remains available in `macaca-kernel`.

### Requirement: Scheduler tests use SchedulerFactory

Kernel scheduler behavior tests SHALL use `SchedulerFactory` instead of direct `SimpleScheduler` construction.

#### Scenario: Simple scheduler behavior remains covered

- **WHEN** scheduler tests need the current simple strategy
- **THEN** they construct it with `SchedulerFactory::build(SchedulerKind::Simple)`
- **AND** selection behavior remains equivalent to the previous direct `SimpleScheduler` path.

### Requirement: Scheduler behavior remains unchanged

Migrating upper consumers to `KernelBuilder` SHALL NOT change default scheduler behavior.

#### Scenario: Builder default scheduler

- **WHEN** an upper crate constructs a kernel with `KernelBuilder::new(...).build()`
- **THEN** the default scheduler remains equivalent to the previous `Kernel::new` default.
