# Change: Add Workbench LLM Tool-Call Execution Loop

## Why

CODEX-WASM-WORKBENCH must behave like a real Codex-class coding application:
the user supplies a coding goal, the workbench calls an LLM with a declared
tool schema, the model requests generic service-backed tool calls, the workbench
executes those calls through Macaca service boundaries, and the tool results are
returned to the model until the task completes.

The current workbench starts `/api/chat/v2` and receives agentless WASM dispatch
events, but it does not own a real model-driven tool loop. Hardcoded demo file
generation is explicitly out of scope and would violate the application
boundary.

## Current `service.llm` Finding

The existing LLM service already exposes the minimum provider-neutral tool-call
wire protocol needed by an application-level loop:

- `LlmOptions.tools` sends tool definitions to `llm.chat`.
- `LlmResponse.tool_calls` returns assistant-requested tool calls.
- `LlmMessage::assistant_with_tool_calls(...)` preserves assistant tool-call
  turns in the transcript.
- `LlmMessage::tool_result(...)` sends tool results back to the model.
- `llm.continuation.validate` validates provider continuation requirements
  before dispatching a tool-result continuation transcript.

Therefore this proposal does not require Macaca OS service changes. If
implementation later proves that a provider-specific tool continuation cannot
be represented by the existing contract, the implementation must stop and open a
separate `service.llm` contract补足 proposal instead of adding application-side
special cases.

## What Changes

- Add an app-owned Workbench execution loop that calls `service.llm/llm.chat`
  with a generic coding-tool schema.
- Translate model tool calls into declared Macaca service calls such as
  `service.file`, `service.process`, `service.git`, `service.review`,
  `service.diagnostics`, `service.tool`, `service.mcp`, or `service.skill`
  only when the application manifest declares the capability.
- Append assistant tool-call messages and sanitized tool-result messages to the
  LLM transcript, then continue the loop until the model returns a final answer,
  a policy denial, an approval wait state, a structured unavailable state, or a
  bounded iteration limit.
- Emit app-owned progress events for each loop state, service invocation,
  result, failure, and final summary so the UI is traceable and auditable.
- Keep model/provider selection owned by the existing `service.llm` catalog and
  route-resolution path.

## Non-Goals

- Do not modify Macaca OS service contracts in this change.
- Do not add application-specific branches to Macaca OS, Web shell, SDK,
  runtime host, or generic services.
- Do not hardcode project templates, Hello World files, workflows, app names,
  provider names, model names, or business-domain logic.
- Do not bypass `service.tool`, service policy, approval, trace, audit,
  sandbox, or manifest-declared capability checks.

## Impact

- Affected specs: `application-ui-runtime`
- Affected code if approved: `apps/codex-wasm-workbench/ui/**`,
  `apps/codex-wasm-workbench/app.yaml`, and app-owned validation scripts/tests.
- No Macaca OS service crate, kernel crate, SDK crate, runtime-host provider, or
  Web shell semantic change is planned for this proposal.
