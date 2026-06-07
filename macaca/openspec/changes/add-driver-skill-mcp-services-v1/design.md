# Design: Driver / Skill / MCP Services v1

## Context

S6 is part of Route C serviceization. The microkernel boundary says kernel owns system invariants and dispatch primitives, while Driver, Skill, and MCP are replaceable capabilities. These capabilities currently have existing runtime/facade implementations and upper-layer consumers, so the safest path is additive serviceization: preserve behavior, introduce provider-neutral contracts, migrate consumers to SDK clients, and keep deprecated direct paths searchable for later cleanup.

This proposal follows the S6 plan and the Route C governance documents:

- `macaca/docs/agent-os-microkernel-boundaries.md`
- `macaca/docs/route-c-serviceization-allowlist.md`
- `macaca/docs/route-c-architecture-governance.md`
- `macaca/docs/route-c-regression-matrix.md`

## Goals

- Establish focused `DriverService`, `SkillService`, and `McpService` contracts with explicit lifecycle, status, inventory, catalog, invocation, snapshot, and cleanup semantics.
- Share sanitized tool metadata through `CapabilityToolDescriptor` without making a generic Tool Service own all tool lifecycles.
- Adapt existing runtimes/facades into `SystemService` through runtime-host provider wrappers.
- Expose focused SDK clients and SystemFacade methods so upper layers do not construct or directly call provider runtimes.
- Preserve existing app loading, driver status/reload, skill snapshot, skill-backed MCP, framework toolkit, `/api/chat/v2`, trace viewer, and no-network tests.
- Ensure all service calls are trace-required, policy-aware, logged at key execution nodes, and auditable through sanitized events/snapshots.
- Keep implementation flexible, provider-neutral, pluggable, and free of app/workflow/driver/skill/MCP hardcoding.

## Non-Goals

- Do not collapse Driver, Skill, and MCP into a generic tool macro-service.
- Do not move domain ownership into `macaca-runtime-host`; runtime-host only adapts lifecycle, dispatch, decorators, and snapshot wiring.
- Do not remove old direct runtime APIs during S6.
- Do not introduce new external dependencies unless a later implementation slice proves no existing abstraction can satisfy the requirement.
- Do not change user-facing response shapes unless compatibility serializers map service results back to the existing shape.

## Design Patterns

- **Facade**: each service and SDK focused client exposes a stable capability boundary to upper layers.
- **Adapter / Bridge**: runtime-host wrappers adapt existing `DriverRuntime`, `SkillRuntimeFacade`, executable skill loaders, and `McpRuntimeFacade` to `SystemService`.
- **Abstract Factory**: built-in, package-installed, plugin, and future remote providers are created through service/provider factories instead of direct construction by Web/CLI/kernel.
- **Command**: lifecycle, inventory, status, attach, catalog, and invocation requests are typed commands before conversion to generic `ServiceCommand`.
- **Strategy**: provider selection, skill source selection, MCP definition source selection, tool conflict policy, resource scope policy, and unavailable/fallback behavior are replaceable strategies.
- **Resource Manager / Mediator**: runtime-host coordinates leases and cleanup for driver processes, browser/workspace resources, skill runtime handles, and MCP sessions.
- **State**: driver/MCP lifecycle and skill snapshot readiness are explicit service status values, not implicit nullable runtime fields.
- **Null Object**: missing or disabled providers return structured unavailable clients/providers instead of panics or implicit construction.
- **Observer**: service lifecycle, provider lifecycle, tool invocation, dependency missing, and policy denial emit structured logs/events.
- **Memento**: snapshots expose sanitized inventory, status, counts, and failure summaries without leaking secrets, headers, environment, or full payloads.
- **Specification**: command constructors and provider admission validate trace, scope, permission hints, tool allowlists, dependency readiness, and resource scope.

## Service Ownership

### Driver Service

`macaca-driver` owns provider-neutral driver service constants, typed commands/results, status/snapshot DTOs, and service adapter helpers. It may reuse existing driver runtime types when they are provider-neutral, but it must not depend on kernel, Web, CLI, runtime-host, application-specific behavior, or concrete driver names.

`DriverService` owns driver load/reload, inventory, provider status, tool catalog, tool invocation, service snapshot, and cleanup. Tool invocation requires explicit trace and scope, carries tool input as data, and dispatches through the selected driver provider/runtime strategy.

### Skill Service

`macaca-skill` owns provider-neutral skill service constants, typed commands/results, sanitized snapshot DTOs, executable skill catalog semantics, and invocation DTOs. Existing `SkillSnapshotRequest`, `SkillSnapshot`, `SkillRegistrySnapshot`, and executable skill definitions should be reused where they remain provider-neutral.

`SkillService` owns skill discovery/snapshot, executable skill loading, skill tool catalog, skill invocation, service status, and sanitized snapshot. It must expose entitlement/package readiness as hooks only; full entitlement serviceization remains outside S6.

### MCP Service

`macaca-runtime-host` owns the MCP service contract if MCP runtime remains host-owned; the contract should be split under focused files if needed to keep files below 500 LOC. It adapts existing `McpServerDefinition`, `McpRuntimeContext`, `McpToolPolicy`, and `McpRuntimeStatus` into provider-neutral service commands/results.

`McpService` owns MCP definition registration, dependency probe, tool catalog, tool attachment metadata, tool invocation, status, cleanup, and skill-backed MCP integration through definitions supplied by Skill Service rather than direct Web aggregation.

### Capability Tool Descriptor

`CapabilityToolDescriptor` is common sanitized metadata, not ownership transfer. It provides service id, provider id, capability id, tool name, description, JSON schema, origin kind, permission hints, resource scope hints, conflict namespace, and display name. Invocation DTOs include trace, application/session/agent scope, tool name, JSON input, policy hints, and resource scope.

The descriptor must never include environment variables, headers, API keys, credentials, full command lines with secrets, full files, or unsanitized tool payloads.

## Runtime and SDK Boundaries

`macaca-runtime-host` owns `SystemService` wrappers and runtime resource coordination. It receives generic `ServiceCommand`, validates/decorates through existing service runtime policy and trace paths, translates to typed domain commands, dispatches to injected runtimes/facades, and returns structured results or structured unavailable.

`macaca-sdk` owns focused clients: `SystemDriverClient`, `SystemSkillClient`, and `SystemMcpClient`. SDK clients translate typed commands into generic service calls. They do not construct drivers, skill loaders, MCP processes, registries, package managers, or provider runtimes.

Web and CLI remain presentation adapters. Their primary paths should call SDK/SystemFacade service clients. Direct runtime fields and route implementations can remain only as deprecated compatibility anchors until all callers migrate and dependency gates prove removal is safe.

## Trace, Policy, Logging, and Privacy

Every service call must carry trace context. Service providers and clients must log key execution nodes: command accepted, policy checked, provider selected, dispatch started, dispatch completed, dispatch failed, cleanup started, cleanup completed, and snapshot emitted.

Logs/events must include service id, operation, trace id, application/session/agent scope where applicable, status, counts, duration or error summary. They must not include raw secrets, full environment, headers, credentials, unsanitized file bodies, or raw tool payloads unless a future trusted debug policy explicitly permits it.

## Migration Strategy

1. Create OpenSpec proposal, design, tasks, and delta specs.
2. Add common capability tool DTOs.
3. Add Driver Service contract and adapter.
4. Add Skill Service contract and adapter.
5. Add MCP Service contract.
6. Add runtime-host service providers.
7. Add SDK focused clients and unavailable/null-object clients.
8. Register services and clients in Web state/startup.
9. Migrate framework toolkit and skill-backed MCP assembly to service-backed adapters.
10. Migrate routes, capability catalog, and CLI status paths.
11. Update allowlist/governance only when dependency gates prove the direct edges are gone or explicitly documented as remaining debt.
12. Run OpenSpec, cargo, regression, and GitNexus verification.

## Risks and Mitigations

- **Risk: Driver/Skill/MCP lifecycle semantics are different enough that one abstraction becomes vague.** Mitigation: keep three focused services and only share sanitized tool DTOs.
- **Risk: Web toolkit behavior regresses during migration.** Mitigation: keep deprecated direct runtime fallback paths with warning logs and trace events until service-backed paths are verified.
- **Risk: skill-backed MCP creates ownership ambiguity.** Mitigation: Skill Service supplies provider-neutral definitions/snapshots; MCP Service owns protocol lifecycle and attachment.
- **Risk: allowlist rows are removed before dependency edges disappear.** Mitigation: only update allowlist removals after `cargo metadata` / boundary tests prove they are unnecessary.
- **Risk: snapshots leak sensitive tool/provider data.** Mitigation: require Memento-style sanitized snapshots and privacy checks in command/result construction.
- **Risk: service clients become provider factories.** Mitigation: SDK clients are translators over `SystemServiceClient`; provider factories remain inside host/provider composition.

## Open Questions

- Whether common tool DTOs should live in `macaca-proto` immediately or remain in domain-local shared modules until remote/cross-process service transport requires proto-level ownership.
- Whether Web can remove all direct `macaca-driver` / `macaca-skill` dependencies in S6, or whether DTO/compat fields require documented remaining allowlist debt until S8/S12.
