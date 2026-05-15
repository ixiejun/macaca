# Execution Control Service v1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build optional, policy-selected pause/resume execution control that first works as a runtime capability and then becomes `service.execution_control`.

**Architecture:** Add provider-neutral DTOs in `macaca-proto`, resolve manifest defaults plus `AgentExecutionCommand` overrides deterministically, and let `service.agent_execution` install execution-control adapters for any enabled run. Then expose the same contract through `ServiceRuntime` as `service.execution_control`, with trace, policy, audit, health, snapshots, and unavailable behavior.

**Tech Stack:** Rust workspace, `serde`, `tokio`, Macaca `ServiceRuntime`, OpenSpec, GitNexus, cargo test/check.

---

## File Structure

- Create `macaca/crates/foundation/macaca-proto/src/execution_control_service.rs`
  - Owns provider-neutral DTOs: policy, triggers, resume sources, state, events, commands, results, service ids, and helpers.
- Modify `macaca/crates/foundation/macaca-proto/src/lib.rs`
  - Exports the new DTO module.
- Modify `macaca/crates/foundation/macaca-proto/src/agent_execution_service.rs`
  - Adds `execution_control_override: Option<ExecutionControlPolicyOverride>` to `AgentExecutionCommand`.
- Modify `macaca/crates/foundation/macaca-proto/src/application_manifest.rs`
  - Adds `execution_control: Option<ExecutionControlPolicy>` to `ApplicationManifestV1`.
- Create `macaca/crates/runtime/macaca-runtime-host/src/execution_control.rs`
  - Owns stage-1 policy resolver and in-process control state.
- Create `macaca/crates/runtime/macaca-runtime-host/src/execution_control_service_provider.rs`
  - Owns stage-2 `service.execution_control` provider and unavailable behavior.
- Modify `macaca/crates/runtime/macaca-runtime-host/src/lib.rs`
  - Exports the execution-control runtime and provider modules.
- Modify `macaca/crates/shells/macaca-web/src/agent_execution_backend.rs`
  - Replaces chat-main-thread-specific pause installation with policy-driven execution control.
- Modify `macaca/crates/shells/macaca-web/src/framework_runner.rs`
  - Replaces `RuntimeGoalPause` with generic execution-control adapter input.
- Modify OpenSpec tasks as implementation progresses:
  - `openspec/changes/add-execution-control-service-v1/tasks.md`

## Task 1: Protocol DTO Foundation

**Files:**
- Create: `macaca/crates/foundation/macaca-proto/src/execution_control_service.rs`
- Modify: `macaca/crates/foundation/macaca-proto/src/lib.rs`
- Modify: `macaca/crates/foundation/macaca-proto/src/agent_execution_service.rs`
- Modify: `macaca/crates/foundation/macaca-proto/src/application_manifest.rs`

- [ ] **Step 1: Run impact analysis**

Run:

```bash
npx gitnexus analyze
```

Then run MCP GitNexus impact for:

- `AgentExecutionCommand`
- `ApplicationManifestV1`

Expected: understand direct callers and warn before edits if risk is HIGH or CRITICAL.

- [ ] **Step 2: Write failing proto tests**

Add tests that expect:

- `ExecutionControlPolicy::disabled()` serializes.
- `ExecutionControlPolicy::enabled(...)` carries trigger/resume/checkpoint config.
- `AgentExecutionCommand::new(...).with_execution_control_override(...)` round-trips.
- `ApplicationManifestV1::new(...).execution_control(...)` round-trips.

Run:

```bash
cd macaca && cargo test -p macaca-proto execution_control -- --nocapture
```

Expected: FAIL because types and helpers do not exist.

- [ ] **Step 3: Add minimal DTOs**

Create DTOs with English comments:

- `EXECUTION_CONTROL_SERVICE_ID`
- command constants for `resolve_policy`, `register_execution`, `request_pause`, `record_checkpoint`, `await_resume`, `request_resume`, `cancel_wait`, `query_state`, `snapshot`
- `ExecutionControlMode`
- `ExecutionControlTrigger`
- `ExecutionControlResumeSource`
- `ExecutionControlCheckpointMode`
- `ExecutionControlPolicy`
- `ExecutionControlPolicyOverride`
- `ExecutionControlResolvedPolicy`
- `ExecutionControlState`
- `ExecutionControlEvent`
- command/result structs for stage-2 service calls

Keep each DTO provider-neutral and serializable.

- [ ] **Step 4: Wire DTOs into existing proto contracts**

Update:

- `AgentExecutionCommand` with `execution_control_override: Option<ExecutionControlPolicyOverride>`
- `ApplicationManifestV1` with `execution_control: Option<ExecutionControlPolicy>`
- builder-style helpers for both structs
- `lib.rs` module/export list

- [ ] **Step 5: Run proto tests**

Run:

```bash
cd macaca && cargo test -p macaca-proto execution_control -- --nocapture
cd macaca && cargo test -p macaca-proto application_manifest -- --nocapture
cd macaca && cargo test -p macaca-proto agent_execution_service -- --nocapture
```

Expected: PASS.

## Task 2: Deterministic Policy Resolver

**Files:**
- Create: `macaca/crates/runtime/macaca-runtime-host/src/execution_control.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/lib.rs`

- [ ] **Step 1: Write failing resolver tests**

Tests:

- no manifest default and no override returns disabled.
- command override narrows manifest defaults.
- command override denied when app disallows dynamic opt-in.
- unknown extension trigger returns unsupported.

Run:

```bash
cd macaca && cargo test -p macaca-runtime-host execution_control_policy -- --nocapture
```

Expected: FAIL because resolver does not exist.

- [ ] **Step 2: Implement resolver**

Create `ExecutionControlPolicyResolver` using Strategy-style typed evaluation:

- Input: optional app default, optional command override, trace metadata.
- Output: `ExecutionControlPolicyResolution`.
- Never branch on application, agent, workflow, provider, or driver names.
- Emit structured reason codes suitable for later trace/audit.

- [ ] **Step 3: Run resolver tests**

Run:

```bash
cd macaca && cargo test -p macaca-runtime-host execution_control_policy -- --nocapture
```

Expected: PASS.

## Task 3: Stage-1 Runtime Capability

**Files:**
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/execution_control.rs`
- Modify: `macaca/crates/shells/macaca-web/src/framework_runner.rs`
- Modify: `macaca/crates/shells/macaca-web/src/agent_execution_backend.rs`

- [ ] **Step 1: Write failing adapter tests**

Tests:

- Chat goal-compatible policy installs a pause adapter.
- Task worker policy can also install a pause adapter when explicitly enabled.
- Disabled policy does not install a pause adapter.
- Duplicate resume signals are ignored after first delivery.

Run:

```bash
cd macaca && cargo test -p macaca-web execution_control -- --nocapture
```

Expected: FAIL because web still has `RuntimeGoalPause` / `ChatMainThread`-specific wiring.

- [ ] **Step 2: Implement generic runtime adapter**

Replace `RuntimeGoalPause` with a generic execution-control adapter object carrying:

- pause signal
- resume receiver
- resolved policy
- execution id
- reason/source metadata

Keep the old chat behavior as a policy preset, not as a hardcoded `ChatMainThread` branch.

- [ ] **Step 3: Wire `service.agent_execution`**

Resolve policy before building the runtime agent:

- App default policy comes from available application metadata when wired.
- Command override comes from `AgentExecutionCommand`.
- Compatibility fallback may synthesize the current goal pause policy only for legacy commands that have no explicit manifest source, and it must be marked deprecated with trace metadata.

- [ ] **Step 4: Run web tests**

Run:

```bash
cd macaca && cargo test -p macaca-web execution_control -- --nocapture
cd macaca && cargo test -p macaca-web agent_execution_backend -- --nocapture
```

Expected: PASS.

## Task 4: Application Selection Entry Points

**Files:**
- Modify: YAML/application manifest projection code discovered during implementation.
- Modify: `macaca/crates/foundation/macaca-proto/src/application_manifest.rs`
- Modify: callers that create `AgentExecutionCommand` where per-run override is needed.

- [ ] **Step 1: Locate manifest projection call sites**

Run:

```bash
rg -n "ApplicationManifestV1|AgentExecutionCommand::new|AgentExecutionCommand \\{" macaca/crates macaca/examples -g '*.rs'
```

- [ ] **Step 2: Add tests for manifest default plus command override**

Tests should prove entry C:

- manifest default enables execution control.
- command override narrows policy.
- command override cannot exceed manifest permission.

- [ ] **Step 3: Implement projection and command helper usage**

Keep compatibility additive. Legacy YAML apps without manifest v1 execution-control fields continue to work.

- [ ] **Step 4: Run app/proto/web tests**

Run:

```bash
cd macaca && cargo test -p macaca-proto execution_control -- --nocapture
cd macaca && cargo test -p macaca-web agent_execution_backend -- --nocapture
```

Expected: PASS.

## Task 5: Trace, Audit, And Replay Evidence

**Files:**
- Modify: runtime execution-control module.
- Modify: web EventLog/RunTrace integration points discovered during implementation.

- [ ] **Step 1: Write failing trace/audit tests**

Tests:

- policy resolution produces sanitized metadata.
- pause/resume produces durable events before live emission.
- checkpoint reference does not contain raw prompt or raw payload.

- [ ] **Step 2: Emit structured events**

Add event names:

- `execution_control.policy_resolved`
- `execution_control.pause_requested`
- `execution_control.pause_entered`
- `execution_control.checkpoint_recorded`
- `execution_control.resume_requested`
- `execution_control.resume_delivered`
- `execution_control.resume_rejected`
- `execution_control.wait_timed_out`

- [ ] **Step 3: Run targeted tests**

Run:

```bash
cd macaca && cargo test -p macaca-runtime-host execution_control -- --nocapture
cd macaca && cargo test -p macaca-web execution_control -- --nocapture
```

Expected: PASS.

## Task 6: Stage-2 Service Provider

**Files:**
- Create: `macaca/crates/runtime/macaca-runtime-host/src/execution_control_service_provider.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/lib.rs`
- Modify: service runtime bootstrap/composition root discovered during implementation.

- [ ] **Step 1: Write failing service provider tests**

Tests:

- descriptor exposes `service.execution_control`.
- missing trace is rejected before side effects.
- unavailable provider returns structured unavailable.
- `request_pause` then `request_resume` changes state with traceable results.

Run:

```bash
cd macaca && cargo test -p macaca-runtime-host execution_control_service -- --nocapture
```

Expected: FAIL because provider does not exist.

- [ ] **Step 2: Implement provider**

Use existing service provider patterns:

- descriptor with capabilities
- health
- snapshot
- typed command dispatch
- structured errors
- sanitized metadata

- [ ] **Step 3: Route runtime capability through service**

Replace stage-1 direct capability calls with `ServiceRuntime.call(service.execution_control, ...)` where practical. Keep the in-process provider as the default built-in implementation.

- [ ] **Step 4: Run service tests**

Run:

```bash
cd macaca && cargo test -p macaca-runtime-host execution_control_service -- --nocapture
cd macaca && cargo test -p macaca-runtime-host service_runtime -- --nocapture
```

Expected: PASS.

## Task 7: Deprecation, Boundary Gates, And Full Verification

**Files:**
- Modify: path-specific pause/resume helpers after service-backed path is live.
- Modify: tests/static gates discovered during implementation.
- Modify: `openspec/changes/add-execution-control-service-v1/tasks.md`

- [ ] **Step 1: Add no-hardcoding/static gates**

Add tests that fail on new direct application-name/agent-name routing branches in execution control.

- [ ] **Step 2: Deprecate old helpers**

Mark path-specific helpers deprecated or restrict them to approved adapters.

- [ ] **Step 3: Update OpenSpec task checkboxes**

Only mark tasks complete after tests prove them.

- [ ] **Step 4: Run final verification**

Run:

```bash
cd macaca && cargo fmt --check
cd macaca && cargo test -p macaca-proto execution_control -- --nocapture
cd macaca && cargo test -p macaca-runtime-host execution_control -- --nocapture
cd macaca && cargo test -p macaca-web execution_control -- --nocapture
cd macaca && cargo check -p macaca-proto -p macaca-runtime-host -p macaca-web
openspec validate add-execution-control-service-v1 --strict
git diff --check
```

Expected: PASS.

## Self-Review Notes

- Spec coverage:
  - Optional capability: Tasks 1, 2, 3.
  - Manifest default plus command override: Tasks 1, 2, 4.
  - Strategy-driven triggers/resume sources: Tasks 1, 2, 3.
  - Trace/audit/replay: Task 5.
  - `service.execution_control`: Task 6.
  - Deprecation and boundary gates: Task 7.
- Placeholders:
  - The plan intentionally leaves exact manifest projection files for discovery because repo paths vary by active migration, but each discovery step has a concrete `rg` command and a bounded modification target.
- Type consistency:
  - DTO names use `ExecutionControl*`; service id is `service.execution_control`; command override field is `execution_control_override`.
