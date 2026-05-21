# Design: Manifest-Declared Heartbeat Agent Execution

## Context

The approved Superpowers design is
`docs/superpowers/specs/2026-05-21-manifest-declared-heartbeat-agent-execution-design.md`.
This OpenSpec change implements that design. The architecture must follow
`macaca-os-architecture-governance.md`,
`macaca-os-microkernel-boundaries.md`, and
`macaca-os-serviceization-allowlist.md`.

## Decisions

1. Application manifests, not filesystem scanning, declare which agents
   participate in heartbeat execution.
2. Application Service owns the sanitized declaration projection.
3. Heartbeat Service owns cadence, gates, wake coalescing, snapshots, and
   mementos only.
4. Runtime Host owns the generic bridge strategy because it is the approved
   composition root for local provider orchestration.
5. Agent Execution owns model/tool invocation and must structure-skip heartbeat
   intent when Agent Context did not include `HEARTBEAT.md` source evidence.

## Design Patterns

- **Command:** Manifest projection, Heartbeat wake, and Agent Execution calls are
  typed command/result DTOs.
- **Strategy:** `HeartbeatAgentDispatchStrategy` is replaceable and policy-ready.
- **Facade:** Runtime Host calls services through focused clients/ServiceRuntime,
  never through shell state.
- **Observer:** Event log, trace ids, and service logs record every transition.
- **Memento:** Heartbeat snapshots link to bounded dispatch summaries.
- **Specification:** Boundary tests prevent Scheduler/Web/Heartbeat providers
  from becoming agent execution semantic owners.

## Risk Controls

- Missing Application Service or Agent Execution Service returns structured
  unavailable/skipped evidence and keeps the Heartbeat lane alive.
- Missing `HEARTBEAT.md` returns `heartbeat_profile_missing` before model/tool
  invocation.
- Logs and snapshots contain only ids, counts, statuses, trace ids, audit ids,
  and bounded reason codes.
- No OS-layer branch may depend on app names, business domains, provider names,
  model names, driver names, gateway names, chains, or payment names.
