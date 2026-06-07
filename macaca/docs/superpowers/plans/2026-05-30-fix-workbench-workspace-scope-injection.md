# Fix Workbench Workspace Scope Injection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ensure CODEX-WASM-WORKBENCH uses the Macaca-registered application workspace instead of model-supplied host paths.

**Architecture:** Keep coding orchestration in the application layer, but make workspace scoping a host bridge decoration because the bridge owns the route `app_id` and can read the generic `app_workspaces` registry. The model sees only relative paths and operation intent; workspace roots are injected after manifest/capability admission and before service runtime dispatch.

**Tech Stack:** JavaScript app-owned Workbench UI modules, Axum Rust app UI bridge, OpenSpec, Node test runner, Cargo focused tests.

---

### Task 1: App Tool Schema And Router

**Files:**
- Modify: `apps/codex-wasm-workbench/ui/loop/tool_registry.js`
- Modify: `apps/codex-wasm-workbench/ui/loop/tool_router.js`
- Modify: `apps/codex-wasm-workbench/ui/tests/tool_loop.test.mjs`
- Modify: `apps/codex-wasm-workbench/ui/tests/controller.test.mjs`

- [x] **Step 1: Add failing tests**

Add tests that assert `macaca_git` no longer requires `workspace_root`, relative file paths are promoted into `target.path`, and model-supplied `workspace_root` is rejected.

- [x] **Step 2: Run the failing app tests**

Run: `node --test apps/codex-wasm-workbench/ui/tests/*.test.mjs`
Expected: FAIL because existing schemas still require `workspace_root`.

- [x] **Step 3: Implement the app-layer schema/router change**

Update tool descriptions and JSON schemas so models provide only relative paths, command arguments, and operation-specific fields. Reject absolute paths, `..` escapes, and any model-supplied `workspace_root`.

- [x] **Step 4: Verify app tests pass**

Run: `node --test apps/codex-wasm-workbench/ui/tests/*.test.mjs`
Expected: PASS.

### Task 2: Generic Host Bridge Workspace Decoration

**Files:**
- Modify: `macaca/crates/shells/macaca-web/src/app_ui_routes.rs`

- [x] **Step 1: Add focused Rust tests**

Add tests for a helper that decorates generic workspace-scoped payloads with the registered application workspace root for `service.file`, `service.git`, `service.process`, and `service.code_intelligence`.

- [x] **Step 2: Run the focused test and confirm failure**

Run: `cargo test -p macaca-web app_ui_bridge_workspace --manifest-path macaca/Cargo.toml`
Expected: FAIL before the helper exists.

- [x] **Step 3: Implement the bridge decorator**

Add a small helper in `app_ui_routes.rs` that receives `service_id`, `operation`, payload, and optional `AppWorkspace`, then injects `workspace_root` into known generic DTO shapes without changing app-specific behavior.

- [x] **Step 4: Verify focused Rust tests pass**

Run: `cargo test -p macaca-web app_ui_bridge_workspace --manifest-path macaca/Cargo.toml`
Expected: PASS.

### Task 3: Contract And Real Task Verification

**Files:**
- Modify: `openspec/changes/add-workbench-llm-tool-call-execution-loop/tasks.md`

- [x] **Step 1: Validate contracts and syntax**

Run:
`openspec validate add-workbench-llm-tool-call-execution-loop --strict`
`find apps/codex-wasm-workbench/ui -name '*.js' -print -exec node --check {} \;`
`apps/codex-wasm-workbench/scripts/validate-package.sh`

- [x] **Step 2: Sync the app bundle used by the running Macaca workspace**

Run:
`rsync -a --delete --exclude target apps/codex-wasm-workbench/ /Users/quantum/.macaca/workspaces/apps/codex-wasm-workbench/`

- [x] **Step 3: Run a real Workbench task**

Use the running front end/backend to submit a frontend/backend Hello World task. Verify that file creation attempts come from LLM tool calls, `workspace_root` resolves under `/Users/quantum/.macaca/workspaces/{app_id}`, and any remaining failure is the separately proposed `service.file` parent-directory/provider error rather than a model-supplied host path.

Evidence captured after the file service correction:
- `service.file/file.write` through the app UI bridge created `shared/workbench-smoke/frontend/src/App.js` under `/Users/quantum/.macaca/workspaces/6fbb0369-e1c9-5a98-89b7-eb01f9c9fa93`.
- `service.file/file.read` through the app UI bridge read the same file with `service_status=ok`.
- A real `WorkbenchToolLoopController` run used `service.llm/llm.chat` with `volces:deepseek-v4-flash`, received model-requested `macaca_file` tool calls, corrected one invalid tool payload after a tool result, and wrote `index.html`, `server.js`, and `package.json` into `shared/real-llm-hello-world`.
- Running the generated Node sample on `PORT=3999` returned `{"message":"Hello, World from the backend!"}` from `/api/hello`.

### Task 5: Generic File Service Parent Directory Correction

**Files:**
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/file_service_local.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/file_service_provider_tests.rs`
- Modify: `openspec/changes/fix-workbench-file-service-host-import-availability/`

- [x] **Step 1: Add failing runtime-host regression test**

Add a focused test proving `service.file/file.write` creates missing nested parent directories when `create_parent_directories=true`.

- [x] **Step 2: Implement generic path-resolution fix**

Update the local file provider so missing write targets with missing parents are not rejected by premature parent canonicalization, while preserving traversal and symlink policy checks.

- [x] **Step 3: Verify focused file service tests pass**

Run:
`cargo test -p macaca-runtime-host write_creates_missing_parent_directories_when_requested --manifest-path macaca/Cargo.toml`
`cargo test -p macaca-runtime-host file_service_provider_tests --manifest-path macaca/Cargo.toml`

### Task 4: Bootstrap Workspace Registration Correction

**Files:**
- Add: `macaca/crates/shells/macaca-web/src/app_workspace_bootstrap.rs`
- Modify: `macaca/crates/shells/macaca-web/src/lib.rs`

- [x] **Step 1: Add failing Rust test**

Add a focused test proving app workspace preparation succeeds even when the app has no resolved app-scoped agents.

- [x] **Step 2: Implement generic workspace bootstrap helper**

Move workspace preparation into a small app-agnostic helper and call it before executor registration skips UI-only/WASM-only applications.

- [x] **Step 3: Verify focused Rust test passes**

Run: `cargo test -p macaca-web app_workspace_bootstrap_prepares_workspace_without_agents --manifest-path macaca/Cargo.toml`
Expected: PASS.
