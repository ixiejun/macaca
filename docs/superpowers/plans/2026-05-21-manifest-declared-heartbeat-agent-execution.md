# Manifest-Declared Heartbeat Agent Execution Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the provider-neutral bridge that lets manifest-declared application agents execute `HEARTBEAT.md` work after an accepted Heartbeat wake.

**Architecture:** Application manifests declare heartbeat agents; Application Service projects those declarations through sanitized DTOs; Runtime Host `HeartbeatLane` uses a `HeartbeatAgentDispatchStrategy` to call `service.agent_execution`. Heartbeat owns cadence/wake state only, Agent Execution owns the actual run, and Agent Context proves `HEARTBEAT.md` was present before model/tool execution.

**Tech Stack:** Rust workspace under `macaca/`, OpenSpec, `macaca-proto` typed command DTOs, `macaca-app` manifest projection, `macaca-runtime-host` service strategies, `macaca-web` Agent Execution backend, integration tests.

---

### Task 1: OpenSpec Change

**Files:**
- Create: `openspec/changes/add-manifest-heartbeat-agent-execution/proposal.md`
- Create: `openspec/changes/add-manifest-heartbeat-agent-execution/design.md`
- Create: `openspec/changes/add-manifest-heartbeat-agent-execution/tasks.md`
- Create: `openspec/changes/add-manifest-heartbeat-agent-execution/specs/application-service/spec.md`
- Create: `openspec/changes/add-manifest-heartbeat-agent-execution/specs/heartbeat-service/spec.md`
- Create: `openspec/changes/add-manifest-heartbeat-agent-execution/specs/agent-execution-service/spec.md`
- Create: `openspec/changes/add-manifest-heartbeat-agent-execution/specs/autonomous-runtime/spec.md`
- Modify: `openspec/changes/add-manifest-heartbeat-agent-execution/tasks.md`

- [ ] **Step 1: Write proposal, design, tasks, and delta specs**

Create the change id `add-manifest-heartbeat-agent-execution`. The proposal must state that manifest declarations, not `HEARTBEAT.md` scanning, select heartbeat agents. The design must cite the Command, Strategy, Facade, Observer, Memento, and Specification patterns. The tasks checklist must cover DTOs, manifest parsing, projection, runtime-host dispatch, agent-execution guard, integration proof, and boundary gates.

- [ ] **Step 2: Validate OpenSpec**

Run: `openspec validate add-manifest-heartbeat-agent-execution --strict`

Expected: `Change 'add-manifest-heartbeat-agent-execution' is valid`

### Task 2: Manifest and Application Projection Contract

**Files:**
- Modify: `macaca/crates/application/macaca-app/src/model.rs`
- Modify: `macaca/crates/application/macaca-app/src/service_projection.rs`
- Modify: `macaca/crates/foundation/macaca-proto/src/application_service.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/application_service_provider.rs`

- [ ] **Step 1: Run GitNexus impact analysis**

Run impact checks before editing symbols:

```text
target=AppManifest
target=app_manifest_to_metadata_view
target=ApplicationMetadataQueryCommand
target=ApplicationSystemServiceProvider
```

- [ ] **Step 2: Add manifest-owned autonomy heartbeat types**

In `model.rs`, add `AppAutonomyConfig`, `AppHeartbeatConfig`, and `AppHeartbeatAgentConfig`. Add `autonomy: Option<AppAutonomyConfig>` to `AppManifest`. All fields are serde-defaulted and data-only.

- [ ] **Step 3: Add Application Service heartbeat projection DTOs**

In `application_service.rs`, add:

```rust
pub const APPLICATION_HEARTBEAT_AGENTS_QUERY_COMMAND: &str =
    "application.heartbeat.agents.query";

pub struct ApplicationHeartbeatAgentsQueryCommand {
    pub trace: TraceContext,
    pub scope: ApplicationServiceScope,
}

pub struct ApplicationHeartbeatAgentView {
    pub application_id: ApplicationId,
    pub agent_name: String,
    pub enabled: bool,
    pub profile_id: String,
    pub metadata: BTreeMap<String, String>,
    pub diagnostics: Vec<String>,
}
```

The command requires trace and `application_id`; the view never carries prompt bodies, raw manifests, or `HEARTBEAT.md`.

- [ ] **Step 4: Project heartbeat declarations**

In `service_projection.rs`, add `app_manifest_to_heartbeat_agent_views`. It must validate declared agent names against inline manifest agents, emit diagnostics for unknown declarations, and return no rows when `autonomy.heartbeat.enabled` is false or absent.

- [ ] **Step 5: Expose the projection through Application Service**

In `application_service_provider.rs`, handle `APPLICATION_HEARTBEAT_AGENTS_QUERY_COMMAND` by looking up the app in the registry and returning `ApplicationHeartbeatAgentView` rows.

### Task 3: Agent Execution Heartbeat Intent and Guard

**Files:**
- Modify: `macaca/crates/foundation/macaca-proto/src/agent_execution_service.rs`
- Modify: `macaca/crates/shells/macaca-web/src/agent_execution_backend.rs`
- Modify: `macaca/crates/shells/macaca-web/src/agent_execution_backend/tests.rs`

- [ ] **Step 1: Run GitNexus impact analysis**

Run impact checks before editing symbols:

```text
target=AgentExecutionIntent
target=WebAgentExecutionBackend
target=AgentExecutionResult
```

- [ ] **Step 2: Add heartbeat execution intent and skipped status**

Add `AgentExecutionIntent::Heartbeat` and `AgentExecutionStatus::Skipped`. Update `as_str()`.

- [ ] **Step 3: Add result constructor for structured skips**

Add `AgentExecutionResult::skipped(command, reason_code, context_snapshot)` that returns bounded metadata and no raw prompt content.

- [ ] **Step 4: Guard heartbeat execution on source evidence**

After `build_context_snapshot`, if intent is `Heartbeat` and no `AgentContextSource` has `kind == "profile_file"` and `name == "HEARTBEAT.md"` or equivalent `profile_file/HEARTBEAT.md` source evidence, return `Skipped` with `heartbeat_profile_missing` before building the runtime agent or invoking model/tool calls.

- [ ] **Step 5: Add focused unit tests**

Add tests proving heartbeat intent skips before model invocation when the source is absent, and non-heartbeat intents are unchanged.

### Task 4: Runtime Host Dispatch Strategy

**Files:**
- Create: `macaca/crates/runtime/macaca-runtime-host/src/autonomy_supervisor/heartbeat_agent_dispatch.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/autonomy_supervisor/heartbeat_lane.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/autonomy_supervisor.rs`
- Modify: `macaca/crates/tests/macaca-integration-tests/tests/autonomy_scheduler_heartbeat_services.rs`

- [ ] **Step 1: Run GitNexus impact analysis**

Run impact checks before editing symbols:

```text
target=HeartbeatLane
target=AutonomySupervisor
target=ServiceRuntime
```

- [ ] **Step 2: Add `HeartbeatAgentDispatchStrategy`**

The strategy queries `service.application` with `ApplicationHeartbeatAgentsQueryCommand`, filters enabled declarations, builds `AgentExecutionCommand` with `AgentExecutionIntent::Heartbeat`, and calls `service.agent_execution`.

- [ ] **Step 3: Wire strategy into `HeartbeatLane`**

`HeartbeatLane` receives `ServiceRuntime`. After native cadence wake acceptance, it dispatches heartbeat agents. It logs declaration count, enabled count, dispatched count, skipped count, and trace id.

- [ ] **Step 4: Keep Scheduler out of the path**

Do not call Scheduler from the heartbeat-agent bridge. Scheduler compatibility `HeartbeatWake` remains unrelated to native heartbeat agent execution.

- [ ] **Step 5: Add integration tests**

Add tests proving no declarations produce no agent execution, enabled declarations dispatch `AgentExecutionIntent::Heartbeat`, and Application Service/Agent Execution unavailable states are structured and do not crash the lane.

### Task 5: WASM App Proof Fixture

**Files:**
- Modify: `/Users/quantum/.macaca/workspaces/apps/wasm-crypto-signal-app/app.yaml`
- Ensure existing: `/Users/quantum/.macaca/workspaces/apps/wasm-crypto-signal-app/personas/technical_analyst/HEARTBEAT.md`

- [ ] **Step 1: Add manifest declaration**

Add:

```yaml
autonomy:
  heartbeat:
    enabled: true
    agents:
      - name: technical_analyst
        enabled: true
        profile_id: default
        metadata:
          purpose: operational_probe
```

- [ ] **Step 2: Run manual proof**

Trigger a heartbeat tick through the running local autonomy supervisor or a focused integration harness. Verify the sentinel file appears under the `technical_analyst` private workspace and trace evidence includes heartbeat wake, declaration query, agent context build, and agent execution result.

### Task 6: Verification and Checklist

**Files:**
- Modify: `openspec/changes/add-manifest-heartbeat-agent-execution/tasks.md`

- [ ] **Step 1: Format**

Run: `cargo fmt`

- [ ] **Step 2: Compile focused crates**

Run: `cargo check -p macaca-proto -p macaca-app -p macaca-runtime-host -p macaca-web -p macaca-integration-tests`

- [ ] **Step 3: Run focused tests**

Run:

```bash
cargo test -p macaca-proto heartbeat -- --nocapture
cargo test -p macaca-app heartbeat -- --nocapture
cargo test -p macaca-runtime-host heartbeat -- --nocapture
cargo test -p macaca-web agent_execution_backend -- --nocapture
cargo test -p macaca-integration-tests --test autonomy_scheduler_heartbeat_services -- --nocapture
```

- [ ] **Step 4: Run boundary gates**

Run:

```bash
cargo test -p macaca-integration-tests --test serviceization_escape_hatches -- --nocapture
cargo test -p macaca-integration-tests --test route_c_dependency_boundaries -- --nocapture
openspec validate add-manifest-heartbeat-agent-execution --strict
```

- [ ] **Step 5: Mark tasks truthful**

Only mark `tasks.md` items complete after the corresponding verification passes.
