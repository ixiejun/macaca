# Observable Runtime Events Design

## Context

Macaca already treats `EventLog` as the durable session event source and Web SSE as a thin live adapter. Skill snapshot loading, skill-backed MCP registration, and generic `service.call` data retrieval still have gaps where users see only a later assistant response or an opaque wait. This design makes those runtime steps visible without moving semantic ownership into Web.

## Goals

- Persist skill snapshot/cache/load events before notifying Web UI.
- Persist generic data retrieval and `service.call` audit lifecycle events before notifying Web UI.
- Keep every payload bounded and sanitized: no prompts, raw provider payloads, secrets, package bytes, WASM bytes, or full `SKILL.md` bodies.
- Preserve microkernel/service/application/shell boundaries.
- Reuse existing `EventLog`, SSE, and indexed event query contracts.

## Non-Goals

- Do not add application-specific UI branches.
- Do not change service routing policy or allowlist semantics.
- Do not expose raw service outputs in events.
- Do not replace the existing `service.audit` replay service.

## Design

The implementation uses three existing design patterns from the architecture governance docs:

- Observer: runtime lifecycle events remain subscribable through EventLog notifications and SSE.
- Memento: every event is stored as a bounded `EventEntry` that can be replayed by session, source, agent, or event type.
- Adapter: `macaca-web` adapts service/runtime events into EventLog and SSE without owning their semantics.

`macaca-web` gets one small helper for session-visible runtime events. The helper appends to `EventLog` first and only then sends the matching SSE frame to any active session. Existing call sites in `framework_toolkit` and `skill_mcp` use this helper instead of open-coded append/send pairs.

Skill snapshot loading emits:

- `skill_snapshot_cache_hit`
- `skill_snapshot_build_started`
- `skill_snapshot_ready`
- `skill_snapshot_failed`
- `skill_snapshot_cached`

The payload includes only agent name, skill counts, filtered count, truncated/compact flags, source labels, trace id, and error summaries.

Generic `service.call` visibility reuses the existing service-call audit chain. A Web-level bridge converts audit-safe events for the active session into `service_call_audit` EventLog rows and live SSE notifications. Payloads include stage, trace id, service id, provider id, decision, retry count, latency, and input/output hashes only.

## Risks

- Service-call audit events may already be visible through `system.service_audit`; duplicating them into session EventLog could create noise. Mitigation: use one event type, `service_call_audit`, and only bridge active session-scoped events.
- Skill snapshots may be built before an SSE channel exists. Mitigation: EventLog remains canonical, and SSE is best-effort live notification.
- Current files already contain unrelated user edits. Mitigation: keep changes limited to new helper/spec files and the narrow event call sites.

## Validation

- `openspec validate add-observable-runtime-events --strict`
- `cargo test -p macaca-web skill_mcp`
- `cargo check -p macaca-web`
- `gitnexus detect_changes(scope=all)`
