## ADDED Requirements

### Requirement: Shells Construct No Prompts Or Autonomy Semantics

Web/CLI shells SHALL NOT construct execution, retry, or replan prompts, and SHALL
NOT own task decomposition, role classification, review-retry, fallback planning,
or terminal-state repair. These SHALL be typed Task/Autonomy service commands the
shell submits and whose results it renders.

#### Scenario: Execution prompt is service-owned
- **WHEN** a worker task is dispatched from the shell
- **THEN** the shell SHALL submit structured fields and the execution/retry/replan
  prompt SHALL be produced by a Task/Autonomy service command, not assembled in
  shell code

#### Scenario: Terminal-state repair is service-owned
- **WHEN** goal evaluation cannot be parsed or built
- **THEN** the shell SHALL surface an explicit service-returned outcome and SHALL
  NOT mark the goal complete by default or directly write task/goal persistence

#### Scenario: Partial-goal cancellation is a service command
- **WHEN** a goal decomposition fails and partial tasks must be cancelled
- **THEN** cancellation SHALL be a Task/Autonomy service command, not a shell-side
  direct persistence mutation

### Requirement: CLI Holds No Direct Backend HTTP Client

The CLI SHALL reach the backend only through SDK clients. It SHALL NOT construct
direct HTTP clients, hardcode backend REST route topology, error protocols, or
backend addresses, and SHALL NOT depend on an HTTP client crate for backend
access.

#### Scenario: CLI uses SDK not reqwest
- **WHEN** the CLI performs a skill/tool/workbench operation
- **THEN** it SHALL call an SDK client
- **AND** the CLI crate SHALL NOT depend on a direct HTTP client crate for backend
  calls nor hardcode the backend address
