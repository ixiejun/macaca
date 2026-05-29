## 1. Discovery And Contract Confirmation

- [x] 1.1 Document the existing `service.llm` tool-call protocol fields used by
  the Workbench loop: `tools`, `tool_calls`, assistant tool-call transcript
  messages, tool result messages, and continuation validation.
- [x] 1.2 Confirm the Workbench manifest declares every service family that can
  become model-visible.
- [x] 1.3 Confirm unavailable optional services produce structured unavailable
  tool results rather than hidden fallback behavior.

## 2. App-Owned Loop Design

- [x] 2.1 Split Workbench UI logic into focused app-owned modules for loop
  state, LLM transcript construction, tool schema generation, tool routing,
  bridge dispatch, result sanitization, and rendering.
- [x] 2.2 Add a finite-state execution controller with bounded iteration,
  timeout, tool-call, and output-size budgets.
- [x] 2.3 Generate model-visible tool schemas from manifest-declared
  capabilities and runtime service availability.
- [x] 2.4 Convert LLM tool calls into declared service calls through the existing
  app bridge without hardcoded task templates.
- [x] 2.5 Convert service results, denials, unavailable states, and failures into
  bounded tool-result messages for the next `llm.chat` continuation.

## 3. Observability And Safety

- [x] 3.1 Emit bounded Workbench events for LLM calls, tool calls, tool results,
  approvals, failures, final answers, and loop budget stops.
- [x] 3.2 Ensure logs and UI events exclude raw secrets, provider payloads,
  credentials, unbounded stdout/stderr, and unsanitized file content.
- [x] 3.3 Preserve selected model/provider route metadata from the existing
  `service.llm` model-selection bridge.

## 4. Verification

- [x] 4.1 Add app-owned tests for tool schema generation and manifest capability
  filtering.
- [x] 4.2 Add app-owned tests for successful tool-call continuation and blocked
  tool-call result generation.
- [x] 4.3 Run `node --check` against all Workbench UI modules.
- [x] 4.4 Run Workbench package validation.
- [x] 4.5 Run a real frontend/backend Hello World task and prove created files
  came from LLM tool calls, not static templates. The previous blocker was
  resolved by `fix-workbench-file-service-host-import-availability`.
- [x] 4.6 If `service.llm` cannot represent a required continuation, stop and
  open a separate `service.llm` contract补足 proposal before implementing any
  workaround.

## 5. Workspace Scope Correction

- [x] 5.1 Remove `workspace_root` from model-visible Workbench tool schemas and
  require relative workspace paths instead.
- [x] 5.2 Add app-owned router tests proving model-supplied `workspace_root`
  and absolute paths are rejected before dispatch.
- [x] 5.3 Add generic app UI bridge tests proving workspace-scoped service
  payloads are decorated from the registered application workspace.
- [x] 5.4 Inject `workspace_root` in the generic app UI bridge for file, git,
  process/sandbox, and code-intelligence payload shapes without branching on
  application names, workflows, providers, or models.
- [x] 5.5 Re-run OpenSpec, JS unit tests, Rust focused bridge tests, package
  validation, and a real Workbench Hello World task.
