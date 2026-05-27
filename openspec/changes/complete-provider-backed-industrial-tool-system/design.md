# Design: Provider-Backed Industrial Tool System

## Context

The current industrial tool work established useful contracts, descriptors, observability seams, runtime-environment seams, and gateway seams. The missing part is the production bridge between those contracts and real provider-backed execution. The observed failures are structural: family descriptors are synthetic, route kinds are collapsed into MCP, policy is heuristic, environment and gateway providers are not on the invocation path, diagnostics are static, and integration tests prove only a synthetic happy path.

## Goals

- Make every visible industrial family descriptor traceable to a real provider contributor, owning service, runtime environment, managed gateway, plugin, MCP server, driver, skill, or structured unavailable provider.
- Dispatch tool invocations by typed executor route instead of forcing all families into MCP origin semantics.
- Enforce policy, resource, entitlement, budget, approval, and audit gates before side effects.
- Connect runtime environments and managed gateways to the real invocation path.
- Produce real operator diagnostics for toolset resolution, provider health, provider status, and audit replay.
- Preserve the complete design surface for availability expressions, result normalization, artifact references, compact Context integration, manifest integration, WASM/SDK access, and shell diagnostics.
- Prove the system through application-neutral multi-family tests without fake owners or manually injected availability.

## Non-Goals

- This change will not add application-specific workflow logic.
- This change will not hardcode concrete provider product names into OS routing.
- This change will not require every optional provider to be installed locally; unsupported providers must be explicit diagnostics.
- This change will not bypass existing driver, skill, MCP, memory, task, scheduler, gateway, or entitlement service boundaries.

## Selected Architecture

### Provider Contributor Strategy

Introduce a `ToolFamilyProviderContributor` strategy interface. Each contributor reports descriptors, route metadata, provider health, provider status, and unavailable diagnostics for one or more families. This follows the Strategy and Abstract Factory patterns: `service.tool` asks contributors for capabilities, but contributors own provider-specific construction and health details.

Visible callable descriptors must not use synthetic `service.tool.family.{family}` owners. If a family is optional and absent, the contributor emits a hidden diagnostic or unavailable provider summary with a stable reason code.

### Typed Executor Routes

Industrial descriptors need route metadata that represents how work is executed. The route kind set is application-neutral:

- `Driver`
- `Skill`
- `Mcp`
- `OwningServiceCommand`
- `RuntimeEnvironment`
- `ManagedGateway`
- `Plugin`
- `Unavailable`

`service.tool` dispatches by route kind, not by family name or provider product name. This preserves microkernel boundaries because `service.tool` owns orchestration, while concrete behavior remains in the owning service or adapter.

### Invocation Admission Chain

The invocation path must become a decorator chain:

1. Trace and caller scope validation.
2. Application capability and service allowlist validation.
3. Family/toolset policy validation.
4. Resource lease and sandbox readiness validation.
5. Entitlement and budget validation.
6. Approval validation for side-effecting work.
7. Timeout and result budget validation.
8. Audit, metering, and redaction wrapping.

Every rejection returns typed reason codes and audit events. No owner dispatch may occur before this chain succeeds or returns an explicit approval-required state.

### Runtime Environment And Gateway Execution

Runtime-environment routes execute through `tool_service_environment.rs`. This path covers file, shell, and code-execution style work where sandboxing, cleanup, metering, and resource accounting are mandatory.

Managed-gateway routes execute through `tool_service_gateway.rs`. This path covers browser/web, document, media, communication, enterprise API, or other provider-mediated work when the gateway is configured. Missing gateways remain unavailable diagnostics rather than fake healthy providers.

### Diagnostics And Bootstrap

`tool.toolset.resolve` returns a real provider plan with selected route kinds, filtered providers, unavailable families, policy decisions, and audit references. `tool.provider.health` reports real registered provider counts and reason-code summaries. Web startup calls a runtime-host composition helper that builds the industrial planner from actual contributors; Web must not define provider semantics.

### Complete Surface Integration

The final system must not stop at invocation routing. Provider-backed execution must flow through the rest of the industrial Tools surface:

- Availability expressions use the Specification pattern, bounded TTL caches, and explicit invalidation on provider/config changes.
- Results are normalized into bounded inline content, artifact references, background handles, approval requests, streaming progress, or structured failures.
- Context receives compact capability indexes only; it does not receive raw schemas, raw provider payloads, or unbounded tool documentation.
- Application manifests select toolsets, families, individual tools, approval profiles, and result budget profiles through generic policy data.
- WASM guests and SDK callers use the same `service.tool` service-call path and cannot bypass service runtime, trace, policy, or payload bounds.
- Web, CLI, and frontend surfaces render diagnostics, approvals, artifacts, and audit replay as thin clients.

## Validation Strategy

Validation is contract-first:

- Unit tests reject synthetic owners and all-MCP family descriptors.
- Route tests cover every route kind and unavailable reason.
- Admission tests prove denial and approval happen before side effects.
- Environment and gateway tests prove invocation reaches registered providers and records audit/metering.
- Result tests prove large, binary, streaming, background, approval, and failure results are normalized and sanitized.
- Context, manifest, WASM, SDK, Web, CLI, and frontend boundary tests prove every caller uses the same service-owned surface.
- Integration tests prove an application-neutral multi-family workflow using real registered contributors or explicit unavailable diagnostics.
- Boundary tests prove shells remain thin and `service.tool` does not inspect service internals.

## Risks And Mitigations

- **Risk: provider readiness differs by local machine.** Mitigation: tests must distinguish required in-repo contributors from optional external providers and assert structured unavailable diagnostics for absent optional providers.
- **Risk: route-kind expansion breaks older descriptors.** Mitigation: preserve compatibility mapping for existing driver, skill, and MCP descriptors while requiring industrial families to provide typed executor routes.
- **Risk: admission chain becomes too broad.** Mitigation: implement it as small decorators with single-purpose decision objects and typed reason codes.
- **Risk: Web bootstrap becomes semantic owner.** Mitigation: add boundary tests that Web only calls runtime-host composition helpers.
