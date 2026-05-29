## ADDED Requirements

### Requirement: Workbench model-driven tool loop

CODEX-WASM-WORKBENCH SHALL execute coding tasks through a model-driven
LLM/tool-result loop owned by the application layer and backed by declared
Macaca services.

#### Scenario: User submits a coding task

- **GIVEN** CODEX-WASM-WORKBENCH is running as an app-owned UI with declared
  `service.llm` and workbench service access
- **WHEN** the user submits a coding requirement
- **THEN** the Workbench calls `service.llm/llm.chat` with model-visible tool
  definitions generated from declared capabilities and service availability
- **AND** the Workbench does not generate task-specific files from static
  templates or hardcoded business logic

#### Scenario: Model requests a tool call

- **GIVEN** `service.llm` returns assistant `tool_calls`
- **WHEN** the Workbench receives the response
- **THEN** it validates each tool name, argument schema, declared capability,
  service availability, resource budget, and approval requirement
- **AND** it dispatches allowed calls only through the generic app bridge
  `service.call` boundary
- **AND** denied or unavailable calls are converted into structured tool-result
  messages rather than hidden fallbacks

#### Scenario: Workspace root is platform-scoped

- **GIVEN** the model requests a workspace-scoped file, git, process, or code
  intelligence tool call
- **WHEN** the Workbench validates and dispatches the tool call
- **THEN** the model-visible schema requires only workspace-relative paths,
  command arguments, or operation intent
- **AND** the application UI bridge derives `workspace_root` from the
  Macaca-registered application workspace for the route `app_id`
- **AND** the Workbench rejects model-supplied absolute host roots or
  `workspace_root` fields before service dispatch

#### Scenario: Tool result continuation

- **GIVEN** a model-requested tool call has completed or failed structurally
- **WHEN** the Workbench continues the task
- **THEN** it appends the assistant tool-call turn and bounded tool-result
  messages to the LLM transcript
- **AND** it calls `service.llm/llm.chat` again using the existing LLM service
  continuation protocol
- **AND** it stops only on final model answer, approval wait, structured
  failure, or a bounded loop budget

### Requirement: Workbench tool-loop observability

CODEX-WASM-WORKBENCH SHALL expose bounded, sanitized progress evidence for each
LLM call, tool call, tool result, denial, approval wait, failure, and final
answer.

#### Scenario: Execution events are replayable

- **GIVEN** a coding task runs through the Workbench tool loop
- **WHEN** the UI renders the execution timeline or diagnostics
- **THEN** each loop state transition includes trace id, service id, command
  name, tool call id, status, and bounded diagnostics where available
- **AND** events exclude raw provider payloads, credentials, secrets, unbounded
  command output, and unsanitized file contents

#### Scenario: Optional service is unavailable

- **GIVEN** an optional Workbench service such as `service.mcp` or
  `service.skill` is not registered
- **WHEN** the tool schema is generated or a model requests a dependent tool
- **THEN** unavailable optional tools are either omitted from the model-visible
  schema or return structured unavailable tool results
- **AND** the Workbench does not crash, fake success, or silently route through
  undeclared capabilities
