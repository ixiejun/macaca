# serviceization-escape-hatches Specification

## ADDED Requirements

### Requirement: Reject Shell-Owned Scheduler and Heartbeat Semantics

Serviceization gates SHALL reject new scheduler or heartbeat semantic ownership
inside Web shell, CLI shell, frontend, or other presentation adapters.

#### Scenario: Shell code adds cron parsing

Given a change adds cron parsing, due-run calculation, heartbeat coalescing, or
wake gate evaluation inside shell or frontend code
When serviceization boundary gates evaluate the change
Then the gates fail with guidance to route behavior through `service.scheduler`
or `service.heartbeat`

### Requirement: Reject Kernel-Owned Concrete Scheduler and Heartbeat Providers

Serviceization gates SHALL reject concrete scheduler or heartbeat provider
construction inside microkernel code.

#### Scenario: Kernel code constructs a heartbeat provider

Given a change adds concrete heartbeat provider construction to kernel code
When serviceization boundary gates evaluate the change
Then the gates fail because provider construction belongs in runtime-host
composition and service provider crates

### Requirement: Reject Application-Specific Autonomy Branches

Serviceization gates SHALL reject scheduler and heartbeat code that branches on
application, workflow, provider, driver, model, gateway, chain, payment, or
business-domain names.

#### Scenario: Scheduler provider branches on an application name

Given a scheduler or heartbeat implementation adds a branch for a specific
application or workflow name
When serviceization boundary gates evaluate the change
Then the gates fail because OS-layer autonomy services must remain generic

### Requirement: Allow Generic Facade and Service Calls

Serviceization gates SHALL allow shells, applications, and plugins to request
scheduler or heartbeat behavior through declared capabilities and focused
SystemFacade or SDK clients.

#### Scenario: CLI requests a manual heartbeat wake

Given CLI code calls a focused heartbeat facade client with a typed wake command
When serviceization boundary gates evaluate the change
Then the gates pass the ownership check
And heartbeat coalescing, gate evaluation, and lifecycle semantics remain owned
by `service.heartbeat`
