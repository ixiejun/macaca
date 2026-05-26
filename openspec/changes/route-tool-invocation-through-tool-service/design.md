## Context

This proposal turns `ToolPlan` visible entries into executable tools. `service.tool` becomes the canonical invocation facade for framework agents and service callers. It does not replace owning services; it routes to them using descriptor metadata.

The implementation must close the gap between "tool is visible" and "tool can safely execute in a 7x24 OS." Every invocation needs trace, explicit scope, policy, resource gates, entitlement hooks, result budget, redaction, telemetry, and audit.

## Goals

- Route all production framework tool calls through `SystemToolClient`.
- Enforce policy, approval, resource, entitlement, timeout, cancellation, and result-budget gates before privileged side effects.
- Route concrete calls to the owning service or provider adapter without parsing visible tool names.
- Normalize tool results into bounded inline responses, artifacts, background handles, approval requests, or structured failures.
- Emit sanitized invocation lifecycle logs, events, and audit mementos.

## Non-Goals

- Do not add managed runtime environment providers in this proposal.
- Do not add rich industrial provider families in this proposal.
- Do not move concrete provider lifecycle into Web, CLI, frontend, or SDK.
- Do not remove compatibility paths until migration coverage is complete.

## Decisions

### Facade

`SystemToolClient` is the invocation entrypoint for framework tools and SDK callers.

### Adapter

Web/framework assembly converts `ToolPlanEntry` values into model-visible framework tools. The adapter owns no provider runtime. It only projects a model tool call into a `tool.invoke` command with explicit scope.

### Decorator

Invocation uses decorators for trace, policy, approval, resource, entitlement, timeout, redaction, metering, result budget, and audit. Decorators run before side effects where applicable.

### Strategy

Routing and result-budget behavior are strategies. Descriptor metadata chooses the owning service and route; OS code does not branch on provider product names.

### Memento and Observer

Invocation records are mementos. EventLog/SSE/telemetry are observers. They use stable refs, hashes, counts, ids, and reason codes.

## Trace, Audit, And Logging Requirements

Every invocation must log or emit sanitized events for start, policy decision, approval request/resolution, resource lease, owning-service dispatch, result normalization, artifact persistence, completion, failure, and cancellation. Logs must use trace ids, refs, hashes, counts, status, and reason codes.
