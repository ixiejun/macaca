## Context

Macaca OS governance requires trace, audit, event log, and service events to remain subscribable and replayable. Current skill and data retrieval flows already emit structured logs or audit records, but not every meaningful step reaches the session EventLog and live Web SSE stream.

## Goals

- Make skill loading and data retrieval progress visible to users.
- Persist every event before live SSE notification.
- Keep payloads bounded and sanitized.
- Preserve serviceization, microkernel, and thin-shell boundaries.

## Non-Goals

- Do not move Skill Service, service routing, or audit semantics into Web.
- Do not expose raw provider responses, prompts, manifests, WASM bytes, package bytes, credentials, private keys, or full `SKILL.md` bodies.
- Do not change service policy allowlists.

## Decisions

### Event Bridge Boundary

`macaca-web` owns only the adapter from already-sanitized runtime facts into session EventLog rows and SSE frames. The bridge accepts plain event type, source, optional agent, and JSON payload data. It appends through `AppendEventCommand` before sending any SSE frame.

### Skill Snapshot Visibility

Skill snapshot events describe lifecycle state, counts, source labels, and error summaries. They do not serialize prompt content or instruction bodies.

### Data Retrieval Result Visibility

The Web chat adapter mirrors generic `host_command_results` evidence produced by host dispatch after WASM or skill-driven data retrieval. Session-visible Web events include only audit-safe fields: stage, result index, status, trace id, service id, operation, provider id, and output hash.

The deeper runtime/service audit chain remains owned by the runtime and service layers. Web does not become a service router, policy engine, provider selector, or audit authority.

### Replay

Because events are normal `EventLog` rows, clients can query them with existing session event endpoints and filters. SSE remains a live convenience, not the source of truth.

## Risks

- Additional events may be noisy. Keep event types specific and payloads compact.
- Service-call audit events can occur outside a known session. Bridge only events with session id.
- Existing user edits touch nearby runtime files. Keep this change narrow and avoid broad refactors.
