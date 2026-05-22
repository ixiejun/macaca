## ADDED Requirements

### Requirement: Scheduled Agent Task dependency gates SHALL preserve serviceized ownership

Dependency-boundary gates SHALL prevent scheduled-agent-task implementation code
from violating Macaca's microkernel, service, runtime-host, application, and
shell ownership boundaries.

#### Scenario: Scheduler provider tries to own prompt payloads

- **WHEN** production Scheduler provider code introduces raw prompt fields, prompt parsing, prompt storage, prompt rendering, or LLM invocation for scheduled agent tasks
- **THEN** the dependency gate SHALL fail with file, line, token, and replacement guidance pointing to the Scheduled Agent Task and Agent Execution service boundaries.

#### Scenario: Runtime Host dispatch uses service boundaries

- **WHEN** Runtime Host dispatches a Scheduler `AgentExecution` target
- **THEN** it SHALL depend on provider-neutral DTOs and `ServiceRuntime` calls
- **AND** it SHALL NOT import Web, frontend, concrete application modules, or application-specific business code.

#### Scenario: Service provider imports presentation shell

- **WHEN** the Scheduled Agent Task service provider is compiled or scanned
- **THEN** it SHALL NOT import `macaca-web`, frontend code, CLI presentation state, or gateway presentation adapters.
