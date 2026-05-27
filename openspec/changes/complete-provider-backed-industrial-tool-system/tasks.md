## 1. Contract Correction

- [x] 1.1 Add a correction note that `complete-industrial-tool-family-providers` was catalog-only and is superseded by this change for production execution.
- [x] 1.2 Add failing tests proving visible industrial family descriptors cannot use synthetic `service.tool.family.{family}` owners.
- [x] 1.3 Add failing tests proving industrial family descriptors cannot all be represented as `CapabilityToolOriginKind::Mcp`.

## 2. Typed Route Contracts

- [x] 2.1 Extend capability tool DTOs with application-neutral executor route metadata.
- [x] 2.2 Add route kinds for driver, skill, MCP, owning service command, runtime environment, managed gateway, plugin, and unavailable.
- [x] 2.3 Add typed unavailable reason codes for missing provider, disabled provider, unsupported platform, denied by policy, missing entitlement, missing approval, degraded provider, and unhealthy provider.
- [x] 2.4 Add serialization and compatibility tests for every route kind and unavailable reason.

## 3. Provider Contributor Registry

- [x] 3.1 Introduce a provider contributor strategy trait for descriptors, routes, health, status, and unavailable diagnostics.
- [x] 3.2 Register real contributors for existing driver, skill, MCP, memory, task, scheduler, entitlement, runtime environment, and gateway paths.
- [x] 3.3 Connect file, shell, and code-execution families to runtime-environment routes.
- [x] 3.4 Connect browser/web, document, media, communication, and enterprise API style families to managed-gateway, MCP, plugin, or unavailable routes.
- [x] 3.5 Remove synthetic visible `service.tool.family.{family}` ownership from callable industrial descriptors.

## 4. Invocation Route Dispatch

- [x] 4.1 Refactor `tool.invoke` dispatch to use typed executor route kinds.
- [x] 4.2 Preserve existing driver, skill, and MCP invocation behavior through route adapters.
- [x] 4.3 Add owning-service command dispatch for service-backed families.
- [x] 4.4 Add runtime-environment invocation dispatch with sandbox, cleanup, metering, and audit hooks.
- [x] 4.5 Add managed-gateway invocation dispatch with health, metering, audit, timeout, and sanitized error mapping.
- [x] 4.6 Return structured unavailable results for unsupported route kinds without fake success.

## 5. Policy, Resource, Entitlement, And Approval Gates

- [x] 5.1 Replace metadata-string policy denial with typed policy decision objects.
- [x] 5.2 Replace metadata/profile approval heuristics with typed approval requirements and approval state.
- [x] 5.3 Add an invocation admission chain covering trace scope, service allowlist, family policy, resource lease, entitlement, budget, side-effect class, approval, timeout, and audit wrapping.
- [x] 5.4 Add tests proving denied, approval-required, unavailable, and entitlement-missing invocations do not dispatch to owner providers.

## 6. Operator Diagnostics

- [x] 6.1 Implement `tool.toolset.resolve` with selected providers, route decisions, filtered providers, unavailable families, and policy reason codes.
- [x] 6.2 Implement `tool.provider.health` with real provider counts, degraded counts, unavailable counts, timestamps, and reason-code summaries.
- [x] 6.3 Add provider status snapshots that Web, CLI, SDK, and audit replay can consume without shell-owned semantics.
- [x] 6.4 Implement `tool.catalog.snapshot`, `tool.policy.explain`, `tool.audit.query`, `tool.result.get`, and `tool.artifact.open` against real plan, invocation, artifact, and audit state.
- [x] 6.5 Add tests proving health and status change when contributors are registered, degraded, unavailable, or disabled.
- [x] 6.6 Add tests proving catalog snapshots, policy explanations, audit query results, result retrieval, and artifact opening are bounded and sanitized.

## 7. Result, Artifact, And Availability Completion

- [x] 7.1 Implement result normalization for small inline results, multimodal results, artifact references, streaming progress, background task handles, approval requests, and structured failures.
- [x] 7.2 Persist oversized and binary provider outputs as artifact references with bounded model-visible summaries.
- [x] 7.3 Add availability-expression evaluation with bounded TTL caching and explicit invalidation on provider or config changes.
- [x] 7.4 Add tests proving raw secrets, prompts, headers, environment values, provider payloads, and unbounded outputs are excluded from logs, EventLog, SSE, and audit records.

## 8. Context, Manifest, WASM, SDK, And Shell Integration

- [x] 8.1 Wire compact tool capability indexes into Context with visible family counts, hidden reason counts, toolset summaries, risky-family usage discipline, and capability dependency summaries.
- [x] 8.2 Wire application manifest toolsets, family allow/deny rules, tool allow/deny rules, approval profiles, and result budget profiles into planning and invocation admission.
- [x] 8.3 Route WASM host tool access through `macaca:service/call service.tool/tool.catalog.plan` and `macaca:service/call service.tool/tool.invoke`.
- [x] 8.4 Ensure SDK, SystemFacade, Web, CLI, and frontend diagnostics consume the same service-owned DTOs and cannot define provider semantics.
- [x] 8.5 Add boundary tests proving Context, manifests, WASM, SDK, Web, CLI, and frontend do not bypass `service.tool` or owning service boundaries.

## 9. Bootstrap And Thin Shell Boundaries

- [x] 9.1 Add a runtime-host industrial planner composition helper that registers real provider contributors.
- [x] 9.2 Change Web startup to use the runtime-host helper instead of registering an empty planner.
- [x] 9.3 Add boundary tests proving Web, CLI, and SDK remain thin clients for tool planning, invocation, diagnostics, and audit replay.

## 10. Provider-Backed Integration Proof

- [x] 10.1 Replace the synthetic `industrial_tool_system.rs` proof with provider-backed setup.
- [x] 10.2 Remove manual availability signal injection and fake document owner registration.
- [x] 10.3 Prove a multi-family workflow covering planning, real provider invocation or explicit unavailable diagnostics, result normalization, artifact recording, context index contribution, manifest filtering, provider health, audit replay, and sanitized proof reporting.
- [x] 10.4 Add assertions that no visible callable descriptor uses a synthetic owner and no family is forced into MCP route semantics unless it is actually MCP-backed.

## 11. Governance Validation

- [x] 11.1 Run `openspec validate complete-provider-backed-industrial-tool-system --strict`.
- [x] 11.2 Run focused Rust tests for proto, runtime-host tool service, family providers, invocation, environment, gateway, context, manifest, WASM, SDK, shell diagnostics, frontend diagnostics, and integration coverage.
- [x] 11.3 Run serviceization boundary tests for architecture governance, microkernel boundaries, and allowlist constraints.
- [x] 11.4 Run GitNexus change detection before commit and record HIGH/CRITICAL warnings as notes for this refactor.
