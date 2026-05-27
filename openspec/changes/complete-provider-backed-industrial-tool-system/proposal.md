# Change: Complete provider-backed industrial tool system

## Why

The previous `complete-industrial-tool-family-providers` change completed a descriptor catalog, but it did not complete the industrial provider-backed tool system required by the design documents. The current implementation can describe many families, yet most families are not connected to real owning services, runtime adapters, managed gateways, policy/resource gates, or operator diagnostics.

This change corrects that gap in one implementation scope. The target is not another catalog expansion; the target is a production execution plane where generic applications can plan, invoke, audit, and diagnose real industrial tools through service-owned boundaries.

## What Changes

- Replace synthetic `service.tool.family.{family}` ownership with provider-backed family contributors for file, shell, browser/web, memory, knowledge, task, scheduler, document, code execution, skill, MCP, gateway-backed, plugin-backed, and entitlement-backed families.
- Stop representing every industrial family as `CapabilityToolOriginKind::Mcp`; add typed executor route metadata for driver, skill, MCP, owning service command, runtime environment, managed gateway, plugin, and unavailable routes.
- Wire `service.tool` invocation through route-kind dispatch so existing services and runtime adapters can execute work without application-specific branches in OS code.
- Replace metadata-string denial and approval heuristics with an admission/decorator chain covering policy, resource leases, entitlement, budget, side-effect class, approval, timeout, audit, and metering before side effects.
- Connect runtime environments and managed gateways to the real `tool.invoke` path for file, shell, code execution, browser/web, document, media, communication, and enterprise API style work.
- Implement real `tool.toolset.resolve`, `tool.provider.health`, and provider status snapshots based on registered contributors and availability signals.
- Complete result normalization, artifact persistence, bounded output, background handles, approval-request results, and sanitized audit/query surfaces for provider-backed invocations.
- Wire compact tool capability indexes, manifest-declared toolsets/families, and WASM/SDK service-call paths into the same `service.tool` planning and invocation surface.
- Wire Web/runtime bootstrap to the industrial planner composition root instead of registering an empty planner.
- Replace the synthetic integration proof with provider-backed multi-family tests that use real contributors or structured unavailable diagnostics, not fake owners or manually injected availability.
- Add governance and boundary validation for `macaca-os-architecture-governance.md`, `macaca-os-microkernel-boundaries.md`, and `macaca-os-serviceization-allowlist.md`.

## Impact

- Affected specs: `industrial-tool-system-completion`, `industrial-tool-families`, `tool-capability-planning`, `tool-service-invocation`, `tool-runtime-environments`, `tool-observability`, `service-runtime`, `sdk-system-facade`
- Affected code:
  - `macaca/crates/proto/src/capability_tool.rs`
  - `macaca/crates/runtime/macaca-runtime-host/src/tool_family_providers.rs`
  - `macaca/crates/runtime/macaca-runtime-host/src/tool_service_invocation.rs`
  - `macaca/crates/runtime/macaca-runtime-host/src/tool_service_environment.rs`
  - `macaca/crates/runtime/macaca-runtime-host/src/tool_service_gateway.rs`
  - `macaca/crates/runtime/macaca-runtime-host/src/tool_service_provider.rs`
  - `macaca/crates/shells/macaca-web/src/lib.rs`
  - `macaca/crates/tests/macaca-integration-tests/tests/industrial_tool_system.rs`
- Architecture constraints:
  - `service.tool` remains the orchestration and policy boundary.
  - Concrete execution remains owned by services, MCP servers, plugins, runtime environments, or managed gateways.
  - Shells remain thin clients and cannot define tool semantics.
  - No application-specific workflow, app name, driver name, provider product name, or business logic may be hardcoded into OS routing.
