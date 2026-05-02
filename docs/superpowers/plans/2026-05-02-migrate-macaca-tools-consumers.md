# Migrate macaca-tools Consumers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate upper-layer crates to the additive-first, design-pattern-based `macaca-tools` command/schema/catalog primitives while preserving runtime behavior 1:1.

**Architecture:** Keep `macaca-tools::Tool` and `ToolSet` as deprecated compatibility shells, but make upper layers depend on canonical interfaces: `ToolCommandExecutor`, `ToolSchemaProvider`, `ToolCatalog`, `CompositeToolSet`, and a small adapter boundary where old framework/toolkit contracts still exist. This follows Command, Adapter, Decorator/Chain, and Composite patterns from `macaca/docs/design_patterns.md`.

**Tech Stack:** Rust workspace, `macaca-tools`, `macaca-runtime`, `macaca-agent`, `macaca-sdk`, `macaca-kernel`, `macaca-framework`, `macaca-driver`, `macaca-skill`, `macaca-web`, OpenSpec, GitNexus.

---

## Context Read

Current `refactor-macaca-tools-patterns` already introduced the producer-side primitives:

- `ToolCommandContext`
- `ToolCommand`
- `ToolSchemaProvider`
- `ToolCommandExecutor`
- `ToolCommandMiddleware`
- `ToolCommandPipeline`
- `TraceToolCommandMiddleware`
- `ToolCatalog`
- `CompositeToolSet`

Current upper-layer consumer status:

- `macaca-runtime` mostly uses `ToolCatalog::definitions`, `ToolCatalog::find_tool`, and `ToolCommandExecutor::execute_command`, but still accepts `&dyn ToolSet`.
- `macaca-framework` bridge mostly uses canonical execution/schema/catalog, but its type names and adapter naming still preserve legacy framing.
- `macaca-driver` SDK uses canonical schema/execution in FFI bridge, but driver APIs still expose `tools()` as a driver-owned contract.
- `macaca-skill` tests still call `SkillTool::execute(...)` directly.
- `macaca-agent`, `macaca-sdk`, and `macaca-kernel` still expose `&dyn ToolSet` in public or internal run contracts.
- `macaca-web::AppState` still stores `Arc<dyn ToolSet>`.

Search commands used:

```bash
rg -n "\.parameters_schema\(|\.execute\(|\.execute_streaming\(|\.to_definitions\(|\.get_tool\(|\.tools\(" macaca/crates --glob '*.rs'
rg -n "macaca_tools::ToolSet|dyn ToolSet|Arc<dyn ToolSet>|Box<dyn ToolSet>" macaca/crates --glob '*.rs'
rg -n "macaca_tools::ToolCatalog|ToolCatalog::|ToolCommandExecutor|ToolSchemaProvider|ToolCommand::|ToolCommandContext|CompositeToolSet" macaca/crates --glob '*.rs'
```

## Brainstorm

### Option A: Strict Full Cutover Now

Change every upper-layer function signature from `ToolSet` to `ToolCatalog`, and all call sites from `Tool::execute` to `ToolCommandExecutor::execute_command`.

Benefits:

- Strongest compile-time enforcement.
- Most deprecated references disappear immediately.
- Makes the new design patterns visible at all upper layers.

Risks:

- High blast radius because `Agent`, `Kernel`, `AgenticLoop`, `AppState`, framework bridge, driver tests, and skill tests are all touched.
- Trait-object compatibility can become noisy because some structs still only implement `ToolSet`, relying on blanket `ToolCatalog for T where T: ToolSet`.
- Public API churn can trigger unrelated fixes in many crates.

### Option B: Canonical Consumer Facade First

Add a small `ToolRuntime`/`ToolCatalogRef` facade in `macaca-tools` or use existing `ToolCatalog` as the explicit consumer type, then migrate runtime/framework/web/driver/skill tests in slices. Keep public `Agent` and `Kernel` signatures temporarily on `ToolSet` only where external API stability matters.

Benefits:

- Preserves behavior while reducing deprecated calls.
- Allows small, reviewable migrations.
- Keeps additive-first rule: old APIs remain, consumers progressively move.

Risks:

- Some public signatures still mention `ToolSet` until a later compatibility-removal phase.
- Requires clear OpenSpec scope so “migration” does not accidentally become a breaking API cleanup.

### Option C: Dual-Trait Bridge Layer

Introduce helper functions such as `tool_definitions(&dyn ToolCatalog)` and `execute_tool_command(&dyn ToolCatalog, ...)`, then migrate all consumers to helper functions rather than directly to traits.

Benefits:

- Centralizes behavior and makes trace/correlation decisions uniform.
- Lowest duplication across runtime/framework/web.

Risks:

- Can become another abstraction layer if too broad.
- If helpers are not minimal, it violates the “不可过度设计” rule.

## Recommendation

Use Option B with one narrow helper from Option C only where it removes duplicated execution boilerplate. The guiding rule is: upper-layer crates should stop calling deprecated `macaca-tools` methods directly, but public system contracts do not need a breaking signature change in the first migration pass unless the change is mechanically safe.

Design patterns to apply:

- Command: `ToolCommand` is the canonical execution request.
- Adapter: framework and driver boundaries adapt foreign tool contracts into `ToolCommandExecutor`.
- Decorator / Chain of Responsibility: `ToolCommandPipeline` and middleware own trace wrapping.
- Composite: `CompositeToolSet` owns tool aggregation.
- Facade: if needed, introduce minimal facade helpers for repeated runtime execution flow, not a broad subsystem.

## Migration Plan

### Task 1: Freeze The Contract In OpenSpec

**Files:**

- Modify: `openspec/changes/refactor-macaca-tools-patterns/tasks.md`
- Modify: `openspec/changes/refactor-macaca-tools-patterns/design.md`
- Modify: `openspec/changes/refactor-macaca-tools-patterns/specs/macaca-tools-core/spec.md`

- [ ] **Step 1: Add consumer migration scope to tasks**

Add a new section after the current verification section:

```markdown
## 7. Upper-Layer Consumer Migration

- [ ] 7.1 Migrate direct deprecated `macaca-tools` execution/schema calls in `macaca-skill` tests
- [ ] 7.2 Migrate runtime execution helpers to canonical `ToolCommand` construction
- [ ] 7.3 Migrate `macaca-agent` / `macaca-sdk` / `macaca-kernel` run contracts where safe to `ToolCatalog`
- [ ] 7.4 Keep framework/driver native `tools()` APIs only where they are not `macaca-tools::ToolSet` compatibility calls
- [ ] 7.5 Add compile checks that fail on new deprecated `macaca-tools` consumer calls outside compatibility modules
```

- [ ] **Step 2: Add design decision for migration boundary**

Add to `design.md`:

```markdown
### 6. Upper-layer migration boundary

Upper-layer crates SHALL use canonical `ToolCatalog`, `ToolSchemaProvider`, and `ToolCommandExecutor` entry points when consuming `macaca-tools`.

The migration SHALL NOT require immediate removal of `ToolSet` from public compatibility signatures where that would create unnecessary API churn. Such signatures may remain temporarily if their internal implementation delegates to canonical primitives and if no direct deprecated method call remains outside compatibility modules.
```

- [ ] **Step 3: Add spec scenario for deprecated-call containment**

Add to the delta spec:

```markdown
#### Scenario: Upper-layer consumer avoids deprecated macaca-tools calls

- **GIVEN** an upper-layer crate consumes `macaca-tools`
- **WHEN** it needs tool schema, lookup, definitions, or execution
- **THEN** it SHALL use `ToolSchemaProvider`, `ToolCatalog`, or `ToolCommandExecutor`
- **AND** direct calls to deprecated `macaca-tools` methods SHALL be limited to compatibility adapters inside `macaca-tools` itself or explicitly documented bridge shims
```

- [ ] **Step 4: Validate OpenSpec**

Run:

```bash
openspec validate refactor-macaca-tools-patterns --strict
```

Expected:

```text
Change 'refactor-macaca-tools-patterns' is valid
```

### Task 2: Migrate macaca-skill Test Consumers

**Files:**

- Modify: `macaca/crates/macaca-skill/src/tool.rs`

- [ ] **Step 1: Replace direct test execution calls**

In tests, replace:

```rust
let result = tool.execute(serde_json::json!({})).await.unwrap();
```

with:

```rust
let result = macaca_tools::ToolCommandExecutor::execute_command(
    &tool,
    macaca_tools::ToolCommand::new(serde_json::json!({})),
)
.await
.unwrap();
```

Replace the action/args test with:

```rust
let result = macaca_tools::ToolCommandExecutor::execute_command(
    &tool,
    macaca_tools::ToolCommand::new(serde_json::json!({
        "action": "init",
        "args": ["--verbose"]
    })),
)
.await
.unwrap();
```

Replace the error test with:

```rust
let result = macaca_tools::ToolCommandExecutor::execute_command(
    &tool,
    macaca_tools::ToolCommand::new(serde_json::json!({})),
)
.await;
```

- [ ] **Step 2: Run focused tests**

Run:

```bash
cargo test -p macaca-skill -- --nocapture
```

Expected: tests pass and no new deprecation warning from these test call sites.

### Task 3: Tighten Runtime Tool Execution Helpers

**Files:**

- Modify: `macaca/crates/macaca-runtime/src/agentic_loop.rs`

- [ ] **Step 1: Add local helper for tool definitions**

Add near `accumulate_usage`:

```rust
fn tool_definitions(tools: &dyn macaca_tools::ToolCatalog) -> Option<Vec<macaca_proto::ToolDefinition>> {
    let defs = macaca_tools::ToolCatalog::definitions(tools);
    if defs.is_empty() {
        None
    } else {
        Some(defs)
    }
}
```

- [ ] **Step 2: Replace repeated definitions construction**

Replace:

```rust
let defs = macaca_tools::ToolCatalog::definitions(tools);
if !defs.is_empty() {
    opts.tools = Some(defs);
}
```

with:

```rust
opts.tools = tool_definitions(tools);
```

Replace:

```rust
tools: Some(macaca_tools::ToolCatalog::definitions(tools)),
```

with:

```rust
tools: tool_definitions(tools),
```

- [ ] **Step 3: Add local helper for command execution**

Add:

```rust
async fn execute_tool_command(
    tool: &dyn macaca_tools::Tool,
    input: serde_json::Value,
    context: macaca_tools::ToolCommandContext,
) -> MacacaResult<serde_json::Value> {
    macaca_tools::ToolCommandExecutor::execute_command(
        tool,
        macaca_tools::ToolCommand::with_context(input, context),
    )
    .await
}
```

- [ ] **Step 4: Use helper in event and non-event execution**

Replace event path:

```rust
macaca_tools::ToolCommandExecutor::execute_command(
    tool,
    macaca_tools::ToolCommand::with_context(
        tool_call.arguments.clone(),
        macaca_tools::ToolCommandContext {
            event_tx: trace_tx,
            ..Default::default()
        },
    ),
)
```

with:

```rust
execute_tool_command(
    tool,
    tool_call.arguments.clone(),
    macaca_tools::ToolCommandContext {
        event_tx: trace_tx,
        ..Default::default()
    },
)
```

Replace non-event path:

```rust
macaca_tools::ToolCommandExecutor::execute_command(
    tool,
    macaca_tools::ToolCommand::new(tool_call.arguments.clone()),
)
```

with:

```rust
execute_tool_command(
    tool,
    tool_call.arguments.clone(),
    macaca_tools::ToolCommandContext::default(),
)
```

- [ ] **Step 5: Run runtime checks**

Run:

```bash
cargo check -p macaca-runtime
```

Expected: exit code 0.

### Task 4: Migrate Public Run Contracts Where Safe

**Files:**

- Modify: `macaca/crates/macaca-agent/src/agent.rs`
- Modify: `macaca/crates/macaca-agent/src/basic.rs`
- Modify: `macaca/crates/macaca-sdk/src/builder.rs`
- Modify: `macaca/crates/macaca-kernel/src/kernel.rs`
- Modify: `macaca/crates/macaca-kernel/src/registry.rs`
- Modify: `macaca/crates/macaca-kernel/src/scheduler.rs`
- Modify: `macaca/crates/macaca-web/src/state.rs`
- Modify: `macaca/crates/macaca-web/src/lib.rs`

- [ ] **Step 1: Change imports from ToolSet to ToolCatalog where signatures only consume catalog behavior**

For files that currently import:

```rust
use macaca_tools::ToolSet;
```

replace with:

```rust
use macaca_tools::ToolCatalog;
```

- [ ] **Step 2: Change run signatures from ToolSet to ToolCatalog in pure consumers**

Replace signatures like:

```rust
tools: &dyn ToolSet,
```

with:

```rust
tools: &dyn ToolCatalog,
```

Use this only in contracts where the implementation never calls `ToolSet::tools()` directly.

- [ ] **Step 3: Keep producer storage on concrete composite where possible**

In `macaca-web/src/lib.rs`, prefer:

```rust
let tools: Arc<CompositeToolSet> = Arc::new(CompositeToolSet::new(all_tools));
```

If `AppState` requires trait object storage, prefer:

```rust
pub tools: Arc<dyn macaca_tools::ToolCatalog>,
```

instead of:

```rust
pub tools: Arc<dyn macaca_tools::ToolSet>,
```

- [ ] **Step 4: Run incremental checks**

Run:

```bash
cargo check -p macaca-agent -p macaca-sdk -p macaca-kernel -p macaca-web
```

Expected: exit code 0.

### Task 5: Classify Remaining `.tools()` Calls

**Files:**

- Review: `macaca/crates/macaca-driver/src/builtin/filesystem_driver.rs`
- Review: `macaca/crates/macaca-driver/src/builtin/shell_driver.rs`
- Review: `macaca/crates/macaca-driver/src/registry.rs`
- Review: `macaca/crates/macaca-driver/src/sdk.rs`
- Review: `macaca/crates/macaca-framework/src/mcp.rs`

- [ ] **Step 1: Keep driver-owned `Driver::tools()` calls**

Do not migrate calls where `tools()` belongs to the driver trait, not `macaca_tools::ToolSet`.

Document these as intentionally retained in OpenSpec design:

```markdown
Driver-owned `Driver::tools()` remains a driver API and is not part of deprecated `macaca-tools::ToolSet::tools()`.
```

- [ ] **Step 2: Keep framework `Toolkit::get_tool()` calls**

Do not migrate `Toolkit::get_tool()` in `macaca-framework/src/mcp.rs`; it is framework toolkit API, not deprecated `macaca_tools::ToolSet::get_tool()`.

Document:

```markdown
Framework `Toolkit::get_tool()` remains framework API and is not a deprecated `macaca-tools` consumer path.
```

- [ ] **Step 3: Add a grep-based verification note**

Use:

```bash
rg -n "\.parameters_schema\(|\.execute\(|\.execute_streaming\(|\.to_definitions\(|\.get_tool\(|\.tools\(" macaca/crates --glob '*.rs'
```

Expected remaining matches are only:

- compatibility internals in `macaca-tools/src/tool.rs`
- concrete `Tool` implementations that must implement deprecated compatibility methods until the producer-side trait changes
- driver-owned `Driver::tools()`
- framework-owned `Toolkit::get_tool()` / `ToolHandler::execute()`
- documented compatibility fallback shims

### Task 6: Add Deprecation-Containment Verification

**Files:**

- Modify: `openspec/changes/refactor-macaca-tools-patterns/tasks.md`

- [ ] **Step 1: Add explicit grep verification command**

Add:

```markdown
- [ ] 7.6 Run deprecated-call containment grep and classify all remaining matches
```

- [ ] **Step 2: Run targeted checks**

Run:

```bash
cargo test -p macaca-tools -- --nocapture
cargo test -p macaca-skill -- --nocapture
cargo check -p macaca-runtime -p macaca-agent -p macaca-sdk -p macaca-kernel -p macaca-framework -p macaca-driver -p macaca-web -p macaca-integration-tests
```

Expected: all exit code 0.

- [ ] **Step 3: Run workspace check**

Run:

```bash
cargo check
```

Expected: exit code 0. Existing unrelated warnings may remain, but no new deprecation warning should originate from migrated consumer call sites.

- [ ] **Step 4: Run GitNexus change detection**

Run:

```text
gitnexus_detect_changes(scope: "all")
```

Expected: affected scope is centered on tool consumption, runtime execution, and framework/driver/web bridge surfaces. HIGH risk is acceptable only if caused by `Tool` / `ToolSet` / `ToolCatalog` global contract migration and must be reported.

## Risks And Mitigations

- Risk: Changing public `Agent::run` signatures may ripple across kernel, scheduler, and tests.
  Mitigation: Do this in one small slice with `cargo check -p macaca-agent -p macaca-sdk -p macaca-kernel` immediately after.

- Risk: Grep returns false positives because different crates define their own `execute`, `tools`, and `get_tool`.
  Mitigation: Classify by owner: deprecated only means `macaca-tools::Tool` / `ToolSet` methods, not framework toolkit or driver APIs.

- Risk: `ToolCommandPipeline::with_default_trace()` plus runtime-level `AgentExecutionEvent::tool_call` can create duplicate visible events.
  Mitigation: Do not change event semantics in this migration unless a test proves duplication. This plan only migrates consumer APIs.

- Risk: Replacing `ToolSet` with `ToolCatalog` everywhere may be too broad for the current additive-first phase.
  Mitigation: Keep `ToolSet` in producer compatibility and change upper signatures only where the code truly consumes catalog behavior.

## Self-Review

- Spec coverage: The plan covers canonical execution, schema, middleware trace containment, composite toolset, and deprecated compatibility boundaries.
- Placeholder scan: No `TBD`, `TODO`, or open-ended implementation step remains.
- Type consistency: The plan uses existing types from the current implementation: `ToolCommand`, `ToolCommandContext`, `ToolCommandExecutor`, `ToolSchemaProvider`, `ToolCatalog`, and `CompositeToolSet`.
- Scope: This is a migration plan only; it does not remove old compatibility APIs or change concrete tool JSON behavior.
