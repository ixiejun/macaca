# macaca-runtime Design Pattern Refactor Brainstorm and Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:brainstorm before changing architecture and superpowers:write-plan before implementation. This document is the planning artifact only; do not implement runtime code from this step without a follow-up OpenSpec proposal.

**Goal:** Refactor `macaca-runtime` incrementally so the agentic execution loop becomes a small, explicit template over replaceable runtime primitives while preserving current behavior 1:1.

**Architecture:** Keep the public `AgenticLoop`, `PausableAgenticLoop`, `RuntimeConfig`, `LoopResult`, `ContextWindowManager`, `LoopDetector`, and `PermissionChecker` APIs compatible during the first slices. Add internal primitives first, migrate tests and consumers later, then deprecate old direct extension points only after replacements exist.

**Tech Stack:** Rust, `tokio`, `async-trait`, `macaca-llm`, `macaca-proto`, `macaca-tools`, current runtime unit tests, `pipeline_dry_run` integration tests.

---

## Current Context

`macaca-runtime` is in phase 2 of the global refactor order, after `macaca-task`, `macaca-tools`, `macaca-driver`, and `macaca-skill`. It depends on `macaca-llm`, `macaca-proto`, and `macaca-tools`, and is consumed mainly by `macaca-web` for `ResumeReason` compatibility plus `macaca-integration-tests` for `AgenticLoop`.

Current source layout:

- `agentic_loop.rs`: 1219 lines; contains loop orchestration, LLM calls, tool execution, event forwarding, pause/resume, helper functions, and tests.
- `context_window.rs`: context token estimation and trimming.
- `loop_detector.rs`: repeated tool-call circuit breaker.
- `permission.rs`: tool allow-list, path, and network checks.
- `lib.rs`: public exports.

Primary risks:

- `agentic_loop.rs` violates the project 500-line file guideline and mixes multiple responsibilities.
- Loop variants duplicate setup and termination behavior across `run`, `run_with_events`, and `run_with_pause`.
- Tool execution, permission checking, event forwarding, context compaction, and loop detection are coupled inside `AgenticLoop::run_iteration`.
- `permission.rs` embeds tool-name heuristics and shell command network detection directly in the default checker.
- `ResumeReason` is still consumed by `macaca-web`; removing or moving it early would create cross-crate churn.

## Applicable Design Patterns

- **Template Method:** Define the fixed runtime iteration skeleton while extracting named steps such as prepare options, compact context, call model, record response, handle tool calls, and finish.
- **Strategy:** Make context compaction and loop detection replaceable without changing loop orchestration.
- **Chain of Responsibility:** Compose permission decisions from independent rules: tool allow-list, path scope, network policy, and future approval gates.
- **State:** Represent pause/resume/completed/iteration-limit transitions explicitly in later slices.
- **Observer:** Keep runtime events as pluggable emission sinks instead of branching every step on optional channels.
- **Command:** Wrap tool calls as executable runtime commands with timeout and trace forwarding.
- **Facade:** Keep `AgenticLoop` as the compatibility facade while internals move into smaller primitives.

## Brainstorm: Options

### Option A: Minimal Template Extraction First

Add internal runtime context and named template step helpers while keeping public signatures and behavior unchanged.

Pros:

- Lowest migration risk.
- Tests can prove 1:1 behavior before introducing new public APIs.
- Directly attacks the oversized `agentic_loop.rs` problem.
- Fits the global plan's first slice: "把 `AgenticLoop` 执行阶段命名为 template steps".

Cons:

- Does not immediately solve permission extensibility or context strategy injection.
- May require temporary internal helper structs before final public API is clear.

Risk:

- Low to medium. The core loop is critical, but behavior can be guarded by existing deterministic tests.

### Option B: Runtime Strategy Set First

Introduce a public `RuntimeStepStrategy` or `RuntimePrimitives` trait bundle immediately, then wire `AgenticLoop` through it.

Pros:

- Creates a clear extension boundary early.
- Unblocks future model-specific compaction and configurable loop detection.
- Reduces future churn if designed correctly.

Cons:

- Easy to over-design before consumers exist.
- Public trait mistakes become long-lived compatibility debt.
- Async trait object lifetimes may increase complexity.

Risk:

- Medium to high. Good long-term shape, but too much public API surface for the first slice.

### Option C: Permission Chain First

Extract `PermissionDecisionChain` from `DefaultPermissionChecker` and keep `PermissionChecker` as compatibility facade.

Pros:

- Removes hardcoded rule coupling from `permission.rs`.
- Aligns with tools/driver/skill policy migration.
- Small files and tests make it relatively isolated.

Cons:

- Does not reduce `agentic_loop.rs` first.
- Permission semantics are security-sensitive; accidental allow/deny regressions are high impact.

Risk:

- Medium. Behavior is testable, but must preserve current allow-list, path, and network decisions exactly.

### Option D: Pause/Resume State Machine First

Extract pausable loop state and transition helpers before touching normal loop execution.

Pros:

- Targets the resume path consumed by `macaca-web`.
- Makes future task delegation and coordinator resume behavior more explicit.

Cons:

- `PausableAgenticLoop` appears to be a compatibility path while web moves toward framework runner.
- Prematurely changing this path could destabilize cross-crate resume handling.

Risk:

- Medium to high. Cross-crate coupling through `ResumeReason` makes this a bad first slice.

## Recommended Approach

Choose Option A first, then Option C, then Strategy extraction.

Reasoning:

- The first implementation slice should reduce `agentic_loop.rs` complexity without changing public behavior.
- Template Method gives the safest backbone for later Strategy, Chain, Observer, Command, and State extraction.
- Permission chain should follow once the runtime execution command boundary is clearer.
- Pause/resume state machine should be delayed until the normal loop skeleton and event sink are stable.

## Risk Controls

- Do not remove or rename public compatibility types in the first refactor: `AgenticLoop`, `PausableAgenticLoop`, `ResumeReason`, `RuntimeConfig`, `LoopResult`, `PermissionChecker`.
- Do not introduce new dependencies.
- Do not hardcode application, workflow, driver, or provider names.
- Every new source file must stay under 500 lines.
- All behavior-changing claims must be backed by existing or new deterministic tests.
- Run GitNexus impact before editing runtime symbols during implementation.
- Create OpenSpec proposal/design/tasks/spec before code implementation.

---

# Implementation Plan

## File Map

Expected future implementation files:

- Modify: `macaca/crates/macaca-runtime/src/lib.rs` to export additive primitives.
- Modify: `macaca/crates/macaca-runtime/src/agentic_loop.rs` to become a facade over smaller modules.
- Create: `macaca/crates/macaca-runtime/src/template.rs` for loop template state and step outcomes.
- Create: `macaca/crates/macaca-runtime/src/execution.rs` for tool command execution and timeout handling.
- Create: `macaca/crates/macaca-runtime/src/events.rs` for runtime event emission / observer boundary.
- Create: `macaca/crates/macaca-runtime/src/permission_chain.rs` after first slice, if OpenSpec approves.
- Create: `macaca/crates/macaca-runtime/src/context_strategy.rs` after first slice, if OpenSpec approves.
- Create: `macaca/crates/macaca-runtime/src/runtime_state.rs` in a later pause/resume slice.
- Modify: `macaca/crates/macaca-integration-tests/src/pipeline_dry_run.rs` only if public replacement APIs are introduced.
- Modify: `macaca/crates/macaca-web/src/*` only in a later consumer migration; not in the first runtime refactor.

## Task 1: OpenSpec Proposal and Contract

**Files:**

- Create: `openspec/changes/refactor-macaca-runtime-template-primitives/proposal.md`
- Create: `openspec/changes/refactor-macaca-runtime-template-primitives/design.md`
- Create: `openspec/changes/refactor-macaca-runtime-template-primitives/tasks.md`
- Create: `openspec/changes/refactor-macaca-runtime-template-primitives/specs/runtime-template-primitives/spec.md`

- [ ] **Step 1: Create proposal**

Proposal must state:

- Why `agentic_loop.rs` needs responsibility splitting and template steps.
- What additive primitives are introduced.
- What remains compatible.
- What is explicitly out of scope: changing LLM behavior, changing tool semantics, changing pause/resume public API, changing web/framework runner behavior.

- [ ] **Step 2: Create design**

Design must include:

- Pattern mapping: Template Method, Facade, Observer, Command, then future Strategy/Chain/State.
- Compatibility rules for `AgenticLoop`, `PausableAgenticLoop`, and `ResumeReason`.
- File-size and module-boundary plan.
- Test strategy for 1:1 behavior.
- Risk mitigation for permission and resume paths.

- [ ] **Step 3: Create tasks**

Tasks must be small and reversible:

- Baseline and impact analysis.
- Extract template context and step outcome types.
- Extract tool execution command helper.
- Extract event emission helper.
- Keep public loop methods behavior-equivalent.
- Run tests and deprecated/consumer scans.

- [ ] **Step 4: Create delta spec**

Spec should add requirements for:

- Runtime loop template primitives.
- Behavior-compatible `AgenticLoop` facade.
- Runtime event observer boundary.
- Tool command execution boundary.
- No application-specific branching.

- [ ] **Step 5: Validate OpenSpec**

Run:

```bash
openspec validate refactor-macaca-runtime-template-primitives --strict
```

Expected: validation passes before any runtime code changes.

## Task 2: Baseline and Impact Analysis

**Files:**

- Read-only: runtime, web consumers, integration tests.

- [ ] **Step 1: Confirm clean baseline**

Run:

```bash
git status --short
cargo test -p macaca-runtime -- --nocapture
cargo test -p macaca-integration-tests pipeline_dry_run -- --nocapture
```

- [ ] **Step 2: Run GitNexus impact before edits**

Run impact analysis for at least:

```text
AgenticLoop
run
run_with_events
run_iteration
execute_tool_call
execute_tool_call_with_events
PausableAgenticLoop
ResumeReason
PermissionChecker
ContextWindowManager
LoopDetector
```

Report direct callers, affected flows, and risk before editing symbols. Stop and warn if HIGH or CRITICAL appears.

- [ ] **Step 3: Identify consumer constraints**

Confirm:

- `macaca-web` imports `ResumeReason` but should not require loop execution changes in the first slice.
- `macaca-integration-tests` constructs `AgenticLoop` directly.
- No upper crate should need new hardcoded runtime branches.

## Task 3: Extract Template Method Primitives

**Files:**

- Create: `macaca/crates/macaca-runtime/src/template.rs`
- Modify: `macaca/crates/macaca-runtime/src/lib.rs`
- Modify: `macaca/crates/macaca-runtime/src/agentic_loop.rs`

- [ ] **Step 1: Add internal runtime template types**

Create small internal types such as:

- `RuntimeLoopContext`
- `RuntimeIterationInput`
- `RuntimeIterationOutcome`
- `RuntimeStopReason`

Keep them crate-private unless OpenSpec explicitly requires public export.

- [ ] **Step 2: Name existing template steps**

Refactor `run_iteration` into named helpers without changing behavior:

- `emit_thinking`
- `prepare_llm_messages`
- `call_llm`
- `record_assistant_response`
- `detect_tool_loop`
- `execute_requested_tools`
- `append_tool_result`
- `finish_iteration`

- [ ] **Step 3: Keep facade methods stable**

Ensure these remain callable:

- `AgenticLoop::new`
- `AgenticLoop::run`
- `AgenticLoop::run_with_events`
- `PausableAgenticLoop::run_with_pause`

## Task 4: Extract Tool Command Execution Boundary

**Files:**

- Create: `macaca/crates/macaca-runtime/src/execution.rs`
- Modify: `macaca/crates/macaca-runtime/src/agentic_loop.rs`
- Modify: `macaca/crates/macaca-runtime/src/lib.rs`

- [ ] **Step 1: Move command execution helpers**

Move `execute_tool_command`, timeout wrapping, and trace forwarding into a small execution module.

- [ ] **Step 2: Preserve permission behavior**

Continue using `PermissionChecker::check_tool_with_args` exactly as today. Do not introduce permission chain in this slice unless it is separately specified.

- [ ] **Step 3: Preserve error-as-tool-result semantics**

Tool errors must still be fed back as tool result messages instead of failing the whole loop.

## Task 5: Extract Event Observer Boundary

**Files:**

- Create: `macaca/crates/macaca-runtime/src/events.rs`
- Modify: `macaca/crates/macaca-runtime/src/agentic_loop.rs`

- [ ] **Step 1: Add event sink wrapper**

Add a small wrapper over `Option<mpsc::Sender<AgentExecutionEvent>>` so template steps do not branch on raw `Option` repeatedly.

- [ ] **Step 2: Preserve event ordering**

Keep current ordering:

1. thinking
2. assistant
3. tool_call
4. driver trace events where applicable
5. tool_result
6. completed

- [ ] **Step 3: Add deterministic event test**

Use a fake LLM and tool to assert event ordering for one tool-call loop.

## Task 6: Permission Chain Follow-up Slice

**Files:**

- Create later: `macaca/crates/macaca-runtime/src/permission_chain.rs`
- Modify later: `macaca/crates/macaca-runtime/src/permission.rs`

- [ ] **Step 1: Add rule trait**

Introduce `PermissionRule` or equivalent after template extraction is stable.

- [ ] **Step 2: Wrap existing rules**

Rules should initially reproduce current behavior:

- allowed tool names
- file path scope
- network access

- [ ] **Step 3: Keep `DefaultPermissionChecker` as facade**

Existing callers should continue using `DefaultPermissionChecker` and `PermissionChecker`.

## Task 7: Context and Loop Strategy Follow-up Slice

**Files:**

- Create later: `macaca/crates/macaca-runtime/src/context_strategy.rs`
- Modify later: `macaca/crates/macaca-runtime/src/context_window.rs`
- Modify later: `macaca/crates/macaca-runtime/src/loop_detector.rs`

- [ ] **Step 1: Add context compaction strategy**

Wrap current `ContextWindowManager::trim_if_needed` as the default strategy.

- [ ] **Step 2: Add loop detection strategy**

Wrap current `LoopDetector` as the default strategy with configurable thresholds.

- [ ] **Step 3: Avoid public trait expansion until used**

Keep strategy traits crate-private unless consumer migration needs runtime customization.

## Task 8: Pause/Resume State Follow-up Slice

**Files:**

- Create later: `macaca/crates/macaca-runtime/src/runtime_state.rs`
- Modify later: `macaca/crates/macaca-runtime/src/agentic_loop.rs`

- [ ] **Step 1: Add explicit state transitions**

Model states such as:

- `Running`
- `Paused`
- `WaitingResume`
- `Completed`
- `IterationLimitReached`

- [ ] **Step 2: Preserve `ResumeReason` compatibility**

Do not move or rename `ResumeReason` until web/framework consumers are migrated.

- [ ] **Step 3: Add transition tests**

Cover manual resume, delegate completion, delegate failure, and timeout message injection.

## Task 9: Verification

- [ ] Run `cargo fmt`.
- [ ] Run `cargo test -p macaca-runtime -- --nocapture`.
- [ ] Run `cargo test -p macaca-integration-tests pipeline_dry_run -- --nocapture`.
- [ ] Run `cargo check -p macaca-runtime -p macaca-web -p macaca-integration-tests`.
- [ ] Run `openspec validate refactor-macaca-runtime-template-primitives --strict`.
- [ ] Run deprecated/consumer scan for any newly deprecated runtime APIs if deprecations are introduced.
- [ ] Run `npx gitnexus detect-changes --repo agent --scope all`.

## Completion Criteria

- `agentic_loop.rs` is reduced toward the 500-line project limit without creating any new oversized file.
- Existing public runtime APIs remain callable.
- Agentic loop final responses, max-iteration behavior, tool error behavior, permission denial behavior, and event ordering remain unchanged.
- New primitives are generic infrastructure, not application-specific or workflow-specific code.
- OpenSpec and tests are aligned before implementation is considered complete.
