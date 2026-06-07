# Per-Agent Heartbeat Profiles Design

## Context

Macaca currently lets an application manifest declare heartbeat agents through
`autonomy.heartbeat.agents[]`, but runtime-host collapses those declarations
into one application-scoped Heartbeat profile. That makes cadence editing
ambiguous: operators can change one application profile, but they cannot give
different agents different native heartbeat cadence policies.

Heartbeat must remain separate from Scheduler. Scheduler owns application-created
scheduled jobs, while Heartbeat owns native cadence, gate evaluation, wake
mementos, and profile policy. Agent Execution owns actual agent work.

## Goals

- Register one native Heartbeat profile for each valid manifest-declared
  heartbeat agent.
- Let each agent profile carry its own fixed interval and optional cooldown
  policy without hardcoding application or business names.
- Keep Application Service as the owner of sanitized manifest projection.
- Keep runtime-host as the Adapter between manifest declarations and
  Heartbeat-owned profile registration.
- Keep Web/frontend as thin operators of Heartbeat profile commands.
- Preserve trace, audit, memento, and structured unavailable behavior.

## Non-Goals

- Do not reintroduce Heartbeat as a Scheduler job or Scheduler target kind.
- Do not make Web or frontend edit raw application manifests.
- Do not read or interpret `HEARTBEAT.md` while registering profiles.
- Do not encode `wasm-crypto-signal-app`, `technical_analyst`, finance, crypto,
  or any application-specific workflow in OS-layer code.

## Design

The selected approach is an additive per-agent profile model.

`AppHeartbeatAgentConfig` gains optional policy fields for cadence and gates.
The first policy supports a fixed interval in seconds plus an optional cooldown
in seconds. Missing values use runtime-host defaults, preserving current
manifests.

Application Service projection exposes safe computed fields:

- `profile_id`: the manifest selector, kept for compatibility and traceability.
- `native_profile_id`: the concrete Heartbeat profile id registered by
  runtime-host.
- `wake_scope_key`: the concrete Heartbeat scope key for this agent.
- optional interval/cooldown seconds.

Runtime-host registers each enabled, known heartbeat agent as:

```text
profile.application.{app_id}.agent.{agent_name}.heartbeat
application:{app_id}.agent:{agent_name}.heartbeat
```

The profile metadata carries bounded service-safe keys such as `application_id`,
`agent_name`, and `manifest_profile_id`. During native cadence ticks, Heartbeat
copies profile metadata into the wake command before gate evaluation. The local
gate Strategy can therefore evaluate per-profile cooldown while still staying
provider-neutral.

The Heartbeat dispatch Strategy filters declarations by `native_profile_id` or
`wake_scope_key`, so one accepted agent profile dispatches only its owning
agent. If older application-scoped profiles are still present, the strategy
falls back to the legacy all-declarations behavior for compatibility.

## Patterns

- **Command:** profile updates and declaration queries remain typed commands.
- **Facade:** Web routes continue through SDK clients.
- **Adapter:** runtime-host maps manifest declarations to Heartbeat profiles.
- **Strategy:** Heartbeat gate evaluation reads profile policy generically.
- **Memento:** profiles and runs remain bounded replayable summaries.
- **Observer:** logs include trace id, profile id, scope key, and audit id.

## Risks And Mitigations

- Public DTO expansion has high blast radius. Mitigation: add optional fields
  and update all known constructors/tests in one slice.
- Existing running profiles may remain application-scoped until restart.
  Mitigation: dispatch keeps a compatibility fallback.
- Cooldown policy can be confused with cadence. Mitigation: expose both fields
  separately in API/UI and keep comments explicit.

## Verification

- OpenSpec strict validation for this change.
- Rust formatting and focused tests for application projection, heartbeat
  provider, runtime-host supervisor/dispatch, SDK, and Web route aggregation.
- Frontend lint and TypeScript checks.
- GitNexus detect changes before completion.
