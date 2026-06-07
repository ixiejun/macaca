## 1. Contract DTOs

- [x] 1.1 Extend or wrap `CapabilityToolDescriptor` for industrial metadata: stable tool id, visible name, title, family, toolsets, output shape, executor route, lifecycle, side-effect class, approval profile, result budget profile, artifact policy, trust level, telemetry labels, and sanitized metadata.
- [x] 1.2 Add `ToolPlan`, `ToolPlanEntry`, `HiddenToolPlanEntry`, `ToolConflictDiagnostic`, and stable hidden reason DTOs.
- [x] 1.3 Add tool family and toolset DTOs.
- [x] 1.4 Add availability expression DTOs for config, secret, auth, env, binary, service health, platform, resource, entitlement, plugin, manifest, agent policy, and session context signals.
- [x] 1.5 Add policy ref, approval ref, result class, artifact ref, provider status, audit ref, and invocation ref DTOs.

## 2. Service Contract

- [x] 2.1 Add `macaca-proto/src/tool_service.rs` constants for `service.tool` and all command names.
- [x] 2.2 Add typed commands/results for `tool.catalog.plan`, `tool.catalog.snapshot`, `tool.toolset.resolve`, `tool.invoke`, `tool.invoke.cancel`, `tool.invocation.status`, `tool.result.get`, `tool.artifact.open`, `tool.provider.status`, `tool.provider.health`, `tool.policy.explain`, and `tool.audit.query`.
- [x] 2.3 Export `tool_service` from `macaca-proto`.
- [x] 2.4 Add service descriptor coverage for `service.tool`.

## 3. SDK Facade

- [x] 3.1 Add `SystemToolClient` trait.
- [x] 3.2 Add service-backed client implementation.
- [x] 3.3 Add unavailable Null Object client implementation.
- [x] 3.4 Export tool clients through `macaca-sdk`.

## 4. Validation

- [x] 4.1 Add unit tests for DTO serialization and redaction-by-construction expectations.
- [x] 4.2 Add unit tests for `ToolPlan` visible/hidden serialization.
- [x] 4.3 Add unit tests for unavailable `SystemToolClient` behavior.
- [x] 4.4 Run `cargo test -p macaca-proto -- --nocapture`.
- [x] 4.5 Run `cargo test -p macaca-sdk -- --nocapture`.
- [x] 4.6 Run `openspec validate add-tool-capability-contracts --strict`.
- [x] 4.7 Run `git diff --check`.

## 5. Governance Notes

- [x] 5.1 Record GitNexus `CRITICAL` and `HIGH` warnings as notes per user instruction; do not block solely on those warnings. GitNexus impact returned LOW for `CapabilityToolDescriptor` and `SystemFacade`; file-level targets were not found by the index, and no CRITICAL/HIGH warning was returned.
- [x] 5.2 Confirm contracts do not create kernel, SDK, Web, CLI, or frontend ownership of provider runtime behavior.
- [x] 5.3 Confirm all non-obvious code has English comments and future runtime hooks identify required structured logging points.
