# Provider-Backed Industrial Tool System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the current descriptor-only industrial tool catalog with a real provider-backed, policy-gated, auditable tool execution plane.

**Architecture:** The implementation uses a provider-contributor strategy plus typed executor routes. `service.tool` remains the orchestration and policy boundary, while concrete work is delegated to owning services, MCP, plugins, runtime environments, or managed gateways through explicit route descriptors.

**Tech Stack:** Rust crates under `macaca/crates/runtime`, `macaca/crates/shells`, `macaca/crates/proto`, OpenSpec, existing service-runtime contracts, and integration tests.

---

## Brainstorm Summary

Three implementation approaches were considered:

- **Patch the existing family catalog in place.** This is the smallest edit, but it would preserve the false model where visible families look callable even when they are only synthetic descriptors.
- **Add product-specific routes per family.** This would make a few demos pass quickly, but it would violate Macaca OS governance because the microkernel would encode provider and application behavior.
- **Introduce provider-backed contributors and route-kind dispatch.** This is the selected approach because it keeps service ownership intact, makes each family independently extensible, and gives policy, resource, entitlement, approval, gateway, and environment gates a single auditable invocation path.

## Implementation Scope

This plan intentionally supersedes the previous catalog-only completion claim for `complete-industrial-tool-family-providers`. The new OpenSpec change is `complete-provider-backed-industrial-tool-system`.

## File Structure

- `macaca/crates/proto/src/capability_tool.rs`: add route-kind and provider-status DTOs without exposing provider-specific business logic.
- `macaca/crates/runtime/macaca-runtime-host/src/tool_family_providers.rs`: replace synthetic family ownership with contributor-backed descriptors and unavailable diagnostics.
- `macaca/crates/runtime/macaca-runtime-host/src/tool_service_invocation.rs`: route invocations by typed executor route and run the admission decorator chain before side effects.
- `macaca/crates/runtime/macaca-runtime-host/src/tool_service_environment.rs`: connect runtime environment providers to invocation for file, shell, and code-execution style routes.
- `macaca/crates/runtime/macaca-runtime-host/src/tool_service_gateway.rs`: connect managed gateway providers to invocation and diagnostics.
- `macaca/crates/runtime/macaca-runtime-host/src/tool_service_provider.rs`: return real toolset resolution, provider health, and status snapshots.
- `macaca/crates/shells/macaca-web/src/lib.rs`: use the runtime-host industrial planner composition helper rather than registering an empty planner.
- `macaca/crates/tests/macaca-integration-tests/tests/industrial_tool_system.rs`: replace synthetic proof with provider-backed multi-family integration proof.

## Task Plan

### Task 1: Correct The Contract Truth

- [ ] Add an implementation note that `complete-industrial-tool-family-providers` delivered catalog descriptors only and is superseded by `complete-provider-backed-industrial-tool-system`.
- [ ] Add failing spec and unit tests proving visible industrial family descriptors cannot use synthetic `service.tool.family.{family}` owners.
- [ ] Add failing tests proving industrial family descriptors cannot all be represented as `CapabilityToolOriginKind::Mcp`.

### Task 2: Add Typed Executor Routes

- [ ] Extend capability tool DTOs with an application-neutral executor route kind covering driver, skill, MCP, owning service command, runtime environment, managed gateway, plugin, and unavailable.
- [ ] Add serialization tests for every route kind and failure reason.
- [ ] Update descriptor construction so route metadata is data-driven and never branches on concrete application names, workflow names, or provider product names.

### Task 3: Replace Catalog Owners With Provider Contributors

- [ ] Introduce a `ToolFamilyProviderContributor` strategy interface for descriptor, health, status, and route contribution.
- [ ] Register contributors for file, shell, web/browser, memory, task, scheduler, document, code execution, skill, MCP, gateway-backed families, and payment/entitlement.
- [ ] Represent absent optional providers as hidden diagnostics or unavailable provider rows, not as visible fake callable tools.

### Task 4: Enforce Industrial Invocation Admission

- [ ] Add an invocation admission chain that checks trace scope, application entitlement, provider allowlist, resource lease, budget, approval, side-effect class, timeout, and audit metadata before dispatch.
- [ ] Replace metadata-string denial and approval heuristics with typed policy decisions and reason codes.
- [ ] Ensure denied, approval-required, unavailable, timeout, and provider-failed outcomes all produce structured audit events.

### Task 5: Connect Runtime Environments And Managed Gateways

- [ ] Route file, shell, and code-execution invocations through registered runtime environment providers with sandbox, cleanup, metering, and audit hooks.
- [ ] Route gateway-backed browser, web, document, media, communication, and enterprise API invocations through managed gateway providers when configured.
- [ ] Preserve unavailable behavior for provider families that are not configured in the local environment.

### Task 6: Return Real Operator Diagnostics

- [ ] Implement `tool.toolset.resolve` so it returns the selected provider plan, unavailable families, policy filters, and route decisions.
- [ ] Implement `tool.provider.health` with real registered provider counts, degraded/unavailable counts, last-check timestamps, and reason-code summaries.
- [ ] Add `tool.provider.status` or equivalent snapshots that can be consumed by Web, CLI, SDK, and audit replay without shell-owned semantics.

### Task 7: Wire Production Bootstrap

- [ ] Add a runtime-host composition helper that builds the industrial tool planner from actual service contributors.
- [ ] Change Web startup to call that helper instead of registering an empty planner.
- [ ] Add boundary tests proving Web, CLI, and SDK remain thin shells and do not own provider semantics.

### Task 8: Replace Synthetic Integration Proof

- [ ] Rewrite `industrial_tool_system.rs` so it does not manually inject availability signals.
- [ ] Remove the fake document owner and use real registered contributors or structured unavailable diagnostics.
- [ ] Add a multi-family proof covering real planning, invocation, artifact recording, provider health, audit replay, and sanitized reporting.

### Task 9: Validate Governance And Contracts

- [ ] Run `openspec validate complete-provider-backed-industrial-tool-system --strict`.
- [ ] Run focused Rust tests for proto, runtime-host tool service, family providers, invocation, gateway, environment, and integration coverage.
- [ ] Run serviceization boundary tests covering architecture governance, microkernel boundaries, and allowlist constraints.
- [ ] Run GitNexus change detection before commit and record HIGH/CRITICAL warnings as notes, not blockers, per the user instruction for this refactor.

