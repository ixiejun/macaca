## Context

This proposal builds the planning plane from the contracts created by `add-tool-capability-contracts`. It is plan-only: production invocation still uses existing service adapters until `route-tool-invocation-through-tool-service`.

The planning service must absorb lessons from Hermes toolsets and OpenClaw tool plans while fitting Macaca's service-owned architecture. It should unify descriptor visibility and hidden diagnostics without moving ownership away from Driver, Skill, MCP, Memory, Task, Scheduler, Gateway, or runtime provider services.

## Goals

- Build deterministic `ToolPlan` snapshots for application/session/agent scope.
- Separate visible and hidden tools with stable reason codes.
- Resolve data-driven families and toolsets.
- Evaluate availability and policy diagnostics without leaking secrets.
- Record conflicts and provider status summaries.
- Feed compact tool capability indexes into Context.
- Preserve exact `allowed_tools` compatibility while adding family/toolset policy.

## Non-Goals

- Do not route production invocation through `service.tool` yet.
- Do not implement runtime environment providers in this proposal.
- Do not add managed gateway execution in this proposal.
- Do not hardcode provider-specific or application-specific tool behavior.

## Decisions

### Adapter Contributors

Existing services contribute descriptors through adapters. Each contributor maps an owning service catalog into provider-neutral descriptors. The contributor does not bypass the owning service and does not own its lifecycle.

### Specification Availability

Availability checks use Specification-style evaluators over declarative expressions. This supports config, secret, auth, env, binary, service health, platform, resource, entitlement, plugin, manifest, agent policy, and session context checks.

### Strategy Toolset Resolution

Tool family and toolset resolution use Strategy objects so profiles can be replaced or extended without hardcoded application logic.

### Memento Plan Snapshots

Plans are immutable snapshots. They contain visible entries, hidden diagnostics, conflicts, counts, timestamps, policy refs, audit refs, and estimated schema tokens.

### Context Progressive Disclosure

Context receives compact indexes: visible families, visible tool names, hidden summary counts, conflicts, and usage discipline. Full tool docs and raw provider payloads remain outside default context.

## Trace, Audit, And Logging Requirements

Planning must log key execution nodes with trace id, application id, session id, agent name, contributor counts, visible count, hidden count, conflict count, and stable reason counts. Logs must not include raw secrets, raw provider payloads, raw prompts, or unbounded schemas.
