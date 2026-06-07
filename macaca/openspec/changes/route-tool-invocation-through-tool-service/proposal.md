# Change: Route tool invocation through service.tool

## Why

Macaca needs one industrial invocation path for tools while preserving concrete lifecycle ownership in Driver, Skill, MCP, Memory, Task, Scheduler, Gateway, runtime environment, and provider services. A tool plan alone is not enough: model-visible tools must execute through trace-required, policy-governed, resource-aware, result-bounded, and audit-backed service calls.

## What Changes

- Implement `tool.invoke`, `tool.invoke.cancel`, `tool.invocation.status`, `tool.result.get`, and artifact-aware responses.
- Route invocations to owning services through descriptor routes.
- Add decorators for trace, policy, approval, resource admission, entitlement, timeout, cancellation, result budget, redaction, telemetry, and audit.
- Migrate framework toolkit invocation to `SystemToolClient`.
- Keep compatibility adapters as deprecated or compatibility-only until all callers migrate.
- Normalize results into bounded inline content, multimodal content, artifact refs, background handles, approval requests, or structured failures.

## Impact

- Affected specs: `tool-service-invocation`, `execution-control-service`, `service-runtime`, `sdk-system-facade`
- Affected code: `macaca-runtime-host`, `macaca-sdk`, `macaca-web`, `macaca-framework`, provider service adapters
- Depends on: `add-tool-capability-contracts`, `add-tool-capability-planning-service`

## Constraints

- `service.tool` coordinates invocation routing but does not own concrete provider lifecycle.
- MCP tools must route through `service.mcp`.
- Skill tools must route through `service.skill`.
- Driver tools must route through `service.driver`.
- Memory, Task, Scheduler, Gateway, and runtime tools must route through their focused services or provider adapters.
- Web, CLI, and frontend must not evaluate policy or own provider lifecycle.
