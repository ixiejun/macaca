## Context

This proposal completes operator visibility for the industrial Tools system. It builds on planning, invocation, environments, and gateway contracts.

The system must be operable in a 7x24 OS: operators need to know what tools are available, why some are hidden, what policies were applied, whether providers are healthy, what invocations happened, where artifacts live, and how to replay evidence safely.

## Goals

- Expose bounded and sanitized diagnostic surfaces.
- Keep Web/CLI/frontend as shell adapters.
- Provide audit replay for plan and invocation decisions.
- Render approval, provider health, invocation lifecycle, and artifact states without owning policy.
- Add API and frontend surfaces that summarize stable refs and aggregate counts rather than raw model/provider output.

## Non-Goals

- Do not let Web, CLI, or frontend make policy decisions.
- Do not expose raw provider payloads, secrets, prompts, credentials, headers, env values, private keys, or unbounded output.
- Do not implement new provider families in this proposal.

## Decisions

### Observer

Tool planning and invocation emit EventLog/SSE/telemetry events for plan, hidden diagnostics, policy, approvals, resource leases, invocation lifecycle, artifacts, and provider health.

### Memento

Tool plan snapshots and invocation audit records are replayable mementos. They use stable refs, hashes, reason codes, counts, and timestamps.

### Facade and Adapter

Web/CLI/frontend consume `SystemToolClient` and thin Web routes. UI components are adapters over sanitized DTOs and must not duplicate policy or routing semantics.

## Trace, Audit, And Logging Requirements

Every diagnostic surface must be bounded and sanitized. Events must include enough stable metadata for replay: trace id, application id, session id, agent name, service id, provider id, tool id, reason code, status, counts, hashes, refs, and timestamps.
