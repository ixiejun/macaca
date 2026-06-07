# Add Workbench LLM Tool-Call Execution Loop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn CODEX-WASM-WORKBENCH into an application-owned, model-driven coding workbench that executes LLM tool calls through declared Macaca services.

**Architecture:** Keep orchestration inside `apps/codex-wasm-workbench` and use existing Macaca service boundaries for all side effects. The Workbench calls `service.llm/llm.chat` with generated tool schemas, executes returned tool calls through the app bridge, appends sanitized tool results, and continues until a final answer or bounded terminal state.

**Tech Stack:** Vanilla app-owned browser UI, Macaca app iframe bridge, `service.llm`, `service.file`, `service.process`, `service.git`, `service.review`, `service.diagnostics`, OpenSpec.

---

## File Structure

- Modify: `apps/codex-wasm-workbench/ui/index.html`
  - Load focused JavaScript modules instead of one large controller file.
- Modify: `apps/codex-wasm-workbench/ui/app.js`
  - Keep DOM wiring and rendering orchestration only.
- Create: `apps/codex-wasm-workbench/ui/loop/state.js`
  - Define loop states, budgets, and state transition helpers.
- Create: `apps/codex-wasm-workbench/ui/loop/llm_client.js`
  - Wrap `service.llm/llm.chat` and `llm.continuation.validate` bridge calls.
- Create: `apps/codex-wasm-workbench/ui/loop/tool_registry.js`
  - Generate model-visible tool definitions from declared service capabilities.
- Create: `apps/codex-wasm-workbench/ui/loop/tool_router.js`
  - Validate and dispatch model tool calls to declared services through `service.call`.
- Create: `apps/codex-wasm-workbench/ui/loop/transcript.js`
  - Build LLM messages, assistant tool-call turns, and bounded tool-result turns.
- Create: `apps/codex-wasm-workbench/ui/loop/sanitize.js`
  - Bound and sanitize service output before logs, UI events, or LLM continuation.
- Create: `apps/codex-wasm-workbench/ui/loop/controller.js`
  - Run the finite-state LLM/tool loop.
- Create: `apps/codex-wasm-workbench/ui/tests/*.test.mjs`
  - Test registry filtering, routing, transcript continuation, and budget stops.

## Task 1: Contract Evidence

- [x] Record the existing `service.llm` tool-call fields in the implementation notes:
  - `macaca/crates/foundation/macaca-proto/src/types.rs` has `LlmOptions.tools`, `LlmResponse.tool_calls`, `LlmMessage::assistant_with_tool_calls`, and `LlmMessage::tool_result`.
  - `macaca/crates/services/macaca-llm/src/service_contract.rs` has `LLM_CHAT_COMMAND`.
  - `macaca/crates/services/macaca-llm/src/hardening_contract.rs` has `LLM_CONTINUATION_VALIDATE_COMMAND`.
- [x] Do not modify these service files for this change.

## Task 2: Loop Modules

- [x] Create `state.js` with explicit states: `idle`, `llm_call`, `tool_dispatch`, `tool_result`, `approval_wait`, `failed`, `complete`.
- [x] Create `sanitize.js` with byte and line limits for service outputs.
- [x] Create `transcript.js` that constructs user, assistant-tool-call, and tool-result messages using the same JSON shape as `LlmMessage`.
- [x] Create `llm_client.js` that calls `service.llm` operations through the existing bridge wrapper.

## Task 3: Tool Registry And Router

- [x] Create generic tool descriptors for file, process, git, review, diagnostics, tool, MCP, and skill families.
- [x] Filter descriptors by manifest-declared required/optional services and runtime catalog availability.
- [x] Reject unknown tool names with structured tool-result errors.
- [x] Dispatch allowed tools through `service.call` with trace metadata and bounded payloads.

## Task 4: Execution Controller

- [x] Replace `macaca.chat.start` task execution with the app-owned LLM loop.
- [x] Preserve existing provider/model catalog loading and route resolution.
- [x] Emit bounded events for every LLM call, tool call, result, denial, failure, and final answer.
- [x] Stop on final answer, approval wait, structured failure, or loop budget.

## Task 5: Tests And Validation

- [x] Add unit tests for manifest capability filtering.
- [x] Add unit tests for successful assistant tool-call continuation.
- [x] Add unit tests for unknown tool denial and unavailable optional services.
- [x] Add a regression test proving no static Hello World templates exist in Workbench UI code.
- [x] Run `node --check` on all Workbench UI modules.
- [x] Run the Workbench package validation script.
- [ ] Run a real frontend/backend Hello World task and verify file creation came from model tool calls. Blocked by `service.file/file.write` unavailable through app host import; see `fix-workbench-file-service-host-import-availability`.

## Task 6: Stop Condition For OS Contract Gap

- [x] Confirm implementation did not discover a `service.llm` continuation contract gap.
- [ ] Create a new OpenSpec proposal for the missing `service.llm` contract field or command if such a gap appears in later validation.
- [x] Do not add provider-specific or task-specific workaround logic to CODEX-WASM-WORKBENCH.
