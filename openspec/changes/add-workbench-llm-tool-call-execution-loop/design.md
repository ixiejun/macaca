# Design: Workbench LLM Tool-Call Execution Loop

## Context

`service.llm` already carries a provider-neutral tool-call transcript:
`LlmOptions.tools`, `LlmResponse.tool_calls`, assistant messages with tool calls,
tool-role result messages, and `llm.continuation.validate`. CODEX-WASM-WORKBENCH
currently does not use this protocol as an autonomous coding loop; it delegates
to the agentless WASM chat path and therefore cannot execute arbitrary coding
tasks through model-requested tools.

## Goals

- Build a real model-driven coding loop inside the Workbench application layer.
- Use only declared Macaca services through the existing app UI bridge.
- Keep tool definitions generic, provider-neutral, and generated from
  application-declared capabilities rather than from hardcoded demo workflows.
- Make each loop iteration observable through bounded UI events and service
  audit evidence.
- Fail truthfully when a required service or provider capability is unavailable.

## Non-Goals

- No Macaca OS service changes in this proposal.
- No Web shell semantic ownership of coding workflows.
- No hardcoded generated files, project templates, model names, provider names,
  or business-domain branches.
- No raw prompt, provider payload, credentials, unbounded stdout/stderr, or
  unsanitized file content in logs, diagnostics, or UI events.

## Architecture

The application-owned flow is:

```text
Workbench UI
  -> LlmToolLoopController
  -> service.llm/llm.chat with tools
  -> LlmToolCallRouter
  -> app bridge service.call
  -> declared Macaca services
  -> sanitized tool result transcript
  -> service.llm/llm.chat continuation
```

The Workbench owns orchestration because it is application behavior. Macaca OS
services own model dispatch, file/process/git/review/diagnostics side effects,
policy, trace, audit, sandboxing, and structured unavailable behavior.

## Design Patterns

- **Command:** every model-visible tool maps to a service command envelope with
  `service_id`, `operation`, `arguments`, and trace metadata.
- **Adapter/Bridge:** the Workbench bridge adapter converts app tool requests to
  the existing `service.call` iframe bridge without learning service internals.
- **Strategy:** a tool strategy registry maps generic model tool names to
  declared service families and can be extended by manifest capability, MCP, or
  skill metadata.
- **State:** the loop explicitly models `idle`, `llm_call`, `tool_dispatch`,
  `tool_result`, `approval_wait`, `failed`, and `complete` states.
- **Observer:** each state transition and service result emits a bounded
  Workbench event for UI progress and replay.
- **Memento:** the transcript keeps sanitized assistant/tool-result turns so the
  loop can be replayed, resumed, or diagnosed without storing secrets.
- **Specification:** service availability, manifest-declared capabilities,
  iteration budgets, and tool argument schemas are executable gates.

## Tool Schema Source

The initial tool set is generated from Workbench manifest capabilities and
current service availability:

- File tools: read, write, patch, list, stat when `service.file` is available.
  Model-visible file arguments use workspace-relative paths only; the
  application UI bridge injects the platform-owned application workspace root
  before dispatch.
- Process tools: exec/status when `service.process` and `service.sandbox` are
  available. Model-visible process arguments may include a relative `cwd`; the
  bridge injects the workspace-scoped sandbox root before dispatch.
- Git tools: status/diff/commit metadata when `service.git` is available. The
  model must not provide `workspace_root`; the bridge derives it from the
  application workspace registered by Macaca Web at startup.
- Review and diagnostics tools when `service.review` and `service.diagnostics`
  are available.
- Optional MCP/skill tools only when the optional services are available and
  declare model-visible descriptors.

The registry must not include task-specific templates. A Hello World request is
just user input; all files and commands must be produced by model tool calls.

## Loop Semantics

1. Build an LLM transcript from the system prompt, selected model route, user
   task, workspace policy, and generated tool schemas.
2. Call `service.llm/llm.chat`.
3. If the response has no tool calls, render the final answer and stop.
4. If the response has tool calls, append the assistant tool-call message to the
   transcript.
5. For each tool call, validate the tool name, declared service access, argument
   schema, resource budget, and approval requirements.
6. Normalize model arguments into service DTO payloads. Workspace-scoped
   services receive `workspace_root` only from the host-side application UI
   bridge, never from the model or prompt.
7. Dispatch allowed calls through `service.call`; blocked calls produce
   structured tool-result errors.
8. Sanitize outputs into bounded `LlmMessage::tool_result(...)` messages.
9. Continue through `service.llm/llm.chat` until final answer, approval wait,
   structured failure, or iteration limit.

## Workspace Scope Injection

The application UI bridge already receives the `app_id` route parameter and can
read the generic `state.config.app_workspaces` registry populated from
`workspace.root_dir`. It therefore acts as a host-side Decorator around
workspace-scoped service calls:

```text
model tool call with relative path/cwd
  -> Workbench router validates relative input
  -> app UI bridge resolves app_id -> AppWorkspace.root
  -> bridge injects workspace_root into file/git/process/code-intelligence DTOs
  -> service runtime dispatches with trace, policy, audit, and provider-owned side effects
```

This keeps the model from selecting arbitrary host directories such as a
developer checkout path. It also keeps workspace ownership in the platform
runtime instead of duplicating `workspace.root_dir` parsing inside the
application bundle.

## Risk And Mitigation

- **Risk:** provider-specific continuation metadata is insufficiently expressed
  by existing `service.llm` DTOs.
  **Mitigation:** use `llm.continuation.validate`; if it rejects a legitimate
  continuation because the contract is missing a field, stop and open a
  `service.llm`补足 proposal.
- **Risk:** the app bridge accidentally becomes a generic OS planner.
  **Mitigation:** keep all changes under `apps/codex-wasm-workbench`; OS
  services remain the only owners of side effects and policy.
- **Risk:** model-visible tools leak raw service payloads.
  **Mitigation:** bound and sanitize tool results before appending transcript or
  UI events.
- **Risk:** tool execution loops forever.
  **Mitigation:** enforce iteration, token, tool-call, timeout, and output-size
  budgets with explicit terminal states.
- **Risk:** host-side workspace injection accidentally becomes
  application-specific routing.
  **Mitigation:** inject only for generic workspace-scoped service DTO shapes,
  keyed by service command families, and never branch on application id, app
  name, workflow, provider, or model.

## Validation

- Validate OpenSpec with `openspec validate add-workbench-llm-tool-call-execution-loop --strict`.
- Add app-owned unit tests for tool schema generation, tool-call routing,
  transcript continuation, denial results, and iteration limits.
- Run `node --check` and package validation for the Workbench bundle.
- Run a real Workbench task that asks for a frontend/backend Hello World app and
  verify that files and commands are produced by model tool calls, not static
  templates.
