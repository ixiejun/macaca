# Manifest-Declared Heartbeat Agent Execution Design

## Context

Macaca OS now has serviceized Scheduler and Heartbeat lanes. The latest runtime
check proved that `service.heartbeat` can accept wakes, write bounded mementos,
and expose audit evidence, but a heartbeat wake does not currently execute an
agent's `HEARTBEAT.md` instructions. That is correct for the current boundary:
Heartbeat owns cadence and wake state, while Agent Execution owns task
execution.

The missing capability is a generic bridge from an accepted heartbeat wake to
app-scoped agent execution. The bridge must not reintroduce Scheduler-owned
heartbeat cadence, must not make Web or frontend own autonomy semantics, and
must not hardcode application, workflow, provider, model, driver, gateway,
chain, payment, or business-domain names.

## Problem Statement

Operators need a way to declare that selected application agents should run
heartbeat work. Today `HEARTBEAT.md` can be loaded as agent profile context, but
the system has no typed contract that says which app agents participate in
heartbeat execution. Scanning for `HEARTBEAT.md` would make filesystem layout
the owner of execution scope, which is not auditable enough for Macaca OS.

## Goals

- Let applications explicitly declare heartbeat-participating agents in their
  manifest.
- Keep `HEARTBEAT.md` as task content only, never as the discovery mechanism.
- Keep Heartbeat responsible for cadence, gates, wake coalescing, snapshots, and
  mementos.
- Keep Agent Execution responsible for actually running agents.
- Route the bridge through runtime-host as the approved composition root.
- Preserve trace, audit, structured unavailable behavior, capability policy, and
  replayable evidence across the heartbeat-to-agent chain.
- Support YAML, WASM, GenUI, and headless applications with the same generic
  contract.

## Non-Goals

- Do not implement application-specific heartbeat tasks.
- Do not branch on app names, agent names, model names, provider names, or
  business domains in OS-layer code.
- Do not make Scheduler own heartbeat cadence again.
- Do not make Web, frontend, CLI, SDK, or the microkernel dispatch heartbeat
  agent execution.
- Do not expose raw `HEARTBEAT.md` content, raw prompts, raw manifests, WASM
  bytes, secrets, credentials, or unbounded provider payloads in logs or
  snapshots.

## Recommended Approach

Use manifest-declared heartbeat agents and a runtime-host dispatch strategy.

```text
Application manifest
  `-- autonomy.heartbeat.agents[]
        |
        v
Application Service sanitized projection
        |
        v
Runtime Host HeartbeatLane
        |
        v
service.heartbeat accepted wake
        |
        v
HeartbeatAgentDispatchStrategy
        |
        v
service.agent_execution typed command
        |
        v
service.agent_context loads agent profile files, including HEARTBEAT.md
```

The application manifest declares participation. The Application Service
projects only sanitized declaration data. Runtime Host observes accepted
heartbeat wakes, resolves declarations through service boundaries, and dispatches
provider-neutral `AgentExecutionCommand` values. Agent Context then loads the
agent's profile directory; `HEARTBEAT.md` supplies instructions only after the
agent has already been selected by manifest contract.

## Manifest Contract

Add an application-framework-owned manifest section:

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

Semantics:

- `autonomy.heartbeat.enabled` disables all heartbeat agent dispatch for the
  application when false.
- `agents[].name` must reference a manifest-declared agent.
- `agents[].enabled` allows an app package to ship a declaration while disabling
  it by default.
- `profile_id` is provider-neutral metadata for future profile selection; first
  implementation may accept it and record it without adding multi-profile
  behavior.
- `metadata` is bounded and sanitized. It must never contain prompts, secrets,
  raw payloads, package bytes, or WASM bytes.

This contract belongs to the application framework because it describes
application-owned agents and app-scoped capability intent. It does not grant
execution by itself; runtime policy and service availability still gate every
run.

## Application Service Projection

Application Service should expose a typed projection such as
`ApplicationHeartbeatAgentsQueryCommand` returning
`ApplicationHeartbeatAgentView` rows.

Projection rules:

- Return application id, agent name, enabled state, profile id, and sanitized
  metadata only.
- Validate that each declared heartbeat agent exists in the manifest.
- Return structured invalid-manifest diagnostics for unknown agents.
- Never return raw manifest text or `HEARTBEAT.md` content.
- Require trace context and emit sanitized query logs.

This keeps Runtime Host from parsing manifests directly and keeps Web/frontend
out of the semantic path.

## Runtime Host Bridge

Add `HeartbeatAgentDispatchStrategy` under runtime-host. It is a Strategy object
owned by `HeartbeatLane`, not by `service.heartbeat`.

Responsibilities:

- Run only after a heartbeat wake is accepted by `service.heartbeat`.
- Query Application Service for manifest-declared heartbeat agents.
- Build one typed `AgentExecutionCommand` per enabled declaration.
- Set `AgentExecutionIntent::Heartbeat` or an equivalent provider-neutral
  heartbeat execution intent.
- Derive trace ids from the heartbeat trace and carry correlation metadata.
- Generate deterministic, app-scoped session/task identifiers suitable for
  audit replay and idempotency.
- Record bounded dispatch summaries into heartbeat mementos or a linked
  autonomy audit record.
- Log key execution nodes: declaration query, filtered declarations, dispatch
  requested, dispatch skipped, dispatch completed, dispatch failed.

The strategy must not read profile files, parse `HEARTBEAT.md`, decide business
work, or call application-specific code.

## Agent Context And Execution

Agent Execution remains the only owner of agent runtime execution. Agent Context
continues to assemble trusted context snapshots through the existing context
boundary.

For heartbeat intent:

- Agent Context may include `HEARTBEAT.md` according to
  `AgentProfileContextConfig.inject_heartbeat`.
- Heartbeat intent requires Agent Context source evidence for
  `profile_file/HEARTBEAT.md`; when that source is absent, Agent Execution must
  return a structured skipped result before invoking a model or tool.
- Agent Execution must emit replayable events with trace id, app id, target
  agent, task id, execution intent, status, and bounded error code.
- Raw prompt bodies and raw `HEARTBEAT.md` content must not appear in logs,
  snapshots, or audit summaries.

## Design Patterns

- **Command:** Manifest projection, heartbeat wake, and agent execution all use
  typed command/result DTOs.
- **Strategy:** `HeartbeatAgentDispatchStrategy` can later be replaced by remote,
  plugin-backed, or policy-aware dispatch variants.
- **Facade:** Runtime Host talks to focused Application and Agent Execution
  clients; callers do not construct providers.
- **Decorator:** Policy, resource, entitlement, metering, and trace decorators
  can wrap the service calls before side effects.
- **Observer:** Event log and audit streams record every boundary transition.
- **Memento:** Heartbeat runs store bounded links to dispatched agent execution
  ids and safe status classes.
- **Specification:** Boundary gates prevent Scheduler, Web, frontend, or
  Heartbeat service providers from owning direct agent execution semantics.

## Error Handling

- Application heartbeat disabled: skip with `heartbeat_application_disabled`.
- No declared heartbeat agents: skip with `heartbeat_no_declared_agents`.
- Unknown declared agent: invalid manifest diagnostic; skip that declaration.
- Missing `HEARTBEAT.md`: structured skip with
  `heartbeat_profile_missing`.
- Agent Execution unavailable: heartbeat run remains accepted, dispatch summary
  records `agent_execution_unavailable`, and no fake success is emitted.
- Policy denied: record `agent_execution_denied` with sanitized reason code.
- Dispatch timeout: record retryable or degraded status according to policy.

Every path must keep the heartbeat lane alive unless policy says the whole
supervisor should stop.

## Testing And Verification

Focused tests should prove:

- A WASM app with `autonomy.heartbeat.agents[]` dispatches a heartbeat
  `AgentExecutionCommand` for the declared agent.
- `HEARTBEAT.md` is read only after manifest selection, not by filesystem
  discovery.
- An app with no declarations produces no agent execution.
- A declaration for an unknown agent is reported as structured invalid manifest
  evidence.
- Scheduler due-run materialization is not required for heartbeat agent
  dispatch.
- Web and frontend schedule routes cannot create heartbeat-agent execution
  semantics.
- Boundary gates still reject presentation-shell ownership, service-provider
  imports of Web, and app-specific OS branches.

Manual proof should use `wasm-crypto-signal-app`, declare `technical_analyst`,
place a sentinel-producing `HEARTBEAT.md` in that agent profile, trigger a
heartbeat tick, and verify the trace chain:

```text
heartbeat wake accepted
  -> application heartbeat declarations queried
  -> agent_execution command dispatched
  -> agent_context_built
  -> heartbeat task completed or skipped with structured evidence
```

## OpenSpec Impact

The implementation plan should create or update an OpenSpec change covering:

- Application manifest heartbeat-agent declaration contract.
- Application Service sanitized heartbeat-agent projection.
- Heartbeat Service requirement to expose accepted wake evidence without owning
  agent execution.
- Runtime Host requirement for the heartbeat-to-agent dispatch strategy.
- Agent Execution heartbeat intent contract and structured unavailable behavior.
- Serviceization and dependency-boundary gates for the new ownership line.

## Acceptance Criteria

- No OS-layer code branches on application-specific names or business domains.
- `service.scheduler` remains unrelated to heartbeat native cadence.
- `service.heartbeat` does not execute agents directly.
- Runtime Host owns the bridge and calls services through typed commands.
- `service.agent_execution` owns actual agent work.
- `HEARTBEAT.md` is task content only and never the source of agent discovery.
- Trace and audit evidence can replay the full chain.
- Logs and snapshots are bounded and sanitized.
