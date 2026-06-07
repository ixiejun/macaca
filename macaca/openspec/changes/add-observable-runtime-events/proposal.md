# Change: Add Observable Runtime Events

## Why

Skill loading, skill-backed MCP registration, and generic data retrieval can run for a while before the Web UI shows useful progress. The user sees an opaque wait even though Macaca already has durable session EventLog and SSE machinery.

## What Changes

- Add generic session runtime events for skill snapshot/cache/build outcomes.
- Add session-visible, audit-safe `service.call` lifecycle events derived from the existing service-call audit chain.
- Require all live Web notifications to be persisted in EventLog before SSE delivery.
- Keep Web as a thin adapter; service/runtime layers remain semantic owners.

## Impact

- Affected specs: `session-event-log`, `skill-service`, `service-runtime-audit`
- Affected code: `macaca-web` runtime event helpers, skill snapshot wiring, service-call audit bridging
