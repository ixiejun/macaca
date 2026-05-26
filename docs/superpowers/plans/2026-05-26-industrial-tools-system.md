# Industrial Tools System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the complete industrial-grade Macaca Tools system described in `docs/macaca-industrial-tools-system-design.md`, including contracts, planning, invocation, runtime environments, managed gateways, rich tool families, context integration, observability, audit, shell diagnostics, and live application-neutral validation.

**Architecture:** Implement a service-owned Tool Capability Plane. `service.tool` coordinates descriptor planning, toolset resolution, policy admission, invocation routing, result/artifact handling, telemetry, and audit while preserving ownership of Driver, Skill, MCP, Memory, Task, Gateway, Store, and other provider services. The plan is split into six OpenSpec proposals so each slice is reviewable, testable, and shippable while the full sequence still covers the complete industrial system.

**Tech Stack:** Rust workspace under `macaca/`, OpenSpec, `macaca-proto`, `macaca-runtime-host`, `macaca-tools`, `macaca-context`, `macaca-sdk`, `macaca-web`, `frontend/`, service runtime, existing Driver/Skill/MCP/Memory/Task/Gateway services, EventLog/SSE/audit surfaces.

---

## Governance Constraints

This implementation must preserve these documents as hard boundaries:

- `macaca/docs/macaca-os-architecture-governance.md`
- `macaca/docs/macaca-os-microkernel-boundaries.md`
- `macaca/docs/macaca-os-serviceization-allowlist.md`
- `macaca/docs/design_patterns.md`
- `docs/macaca-industrial-tools-system-design.md`

Required design pattern usage:

- **Facade:** `SystemToolClient`, `SystemFacade`, focused service clients.
- **Command:** every cross-boundary operation uses typed command/result DTOs.
- **Adapter / Bridge:** Driver, Skill, MCP, Memory, Task, Gateway, plugin, runtime environment, and shell adapters.
- **Strategy:** routing, availability, policy, approval, result budget, conflict handling, provider selection.
- **Decorator:** trace, policy, resource, entitlement, metering, timeout, redaction.
- **State:** provider lifecycle, environment lifecycle, invocation lifecycle.
- **Observer:** EventLog, SSE, telemetry, audit, usage analytics.
- **Memento:** tool plan snapshots, invocation records, artifacts, provider snapshots.
- **Specification:** availability, policy, entitlement, dependency, package admission rules.
- **Abstract Factory:** runtime-host provider/environment/gateway bootstrapping.
- **Null Object:** unavailable providers and disabled tools return structured diagnostics.

Non-negotiable coding rules for later implementation:

- All non-obvious Rust code must include English comments explaining purpose and operating principle.
- Key execution points must emit structured logs or audit events with sanitized fields.
- No OS-layer code may hardcode application names, business workflow names, provider names, model names, driver names, gateway names, chain names, or domain-specific branches.
- GitNexus `CRITICAL` and `HIGH` warnings are recorded as notes for this refactor, not blockers, per user instruction. Still run required detection before commits.
- Each implementation slice must preserve YAML/WASM/GenUI application compatibility.

## OpenSpec Proposal Breakdown

This work must be implemented through six OpenSpec proposals, in order. Do not collapse these into one proposal; the system is too large and each slice needs separate validation.

### Proposal 1: `add-tool-capability-contracts`

Purpose: define the provider-neutral industrial Tools contract and the `service.tool` command surface.

Scope:

- Extend or wrap `CapabilityToolDescriptor`.
- Add tool plan, hidden diagnostics, tool family, toolset, availability expression, policy ref, result class, artifact ref, provider status, and audit DTOs.
- Add `service.tool` descriptor and typed commands/results.
- Add SDK `SystemToolClient` and unavailable Null Object.
- No production invocation migration yet.

Design coverage:

- Design sections 4.2, 4.3, 4.4, 4.5, 4.6, 4.7, 4.12, 4.13, 4.16.

Acceptance:

- DTOs compile.
- Strict OpenSpec validation passes.
- Service descriptor exposes health/snapshot/commands.
- Unavailable client returns structured unavailable states.
- No provider-specific branches.

### Proposal 2: `add-tool-capability-planning-service`

Purpose: implement the catalog planning engine, tool family/toolset resolution, availability diagnostics, conflict handling, and compact context integration.

Scope:

- Add runtime-host `ToolCapabilityServiceProvider`.
- Add descriptor contributors for existing Driver, Skill, MCP, Memory, Task, Scheduler, workspace, and runtime tools.
- Add data-driven tool family and toolset resolution.
- Add visible/hidden plan construction.
- Add availability expression evaluator and diagnostics.
- Add context provider for compact tool capability index.

Design coverage:

- Design sections 4.4, 4.5, 4.6, 4.7, 4.14, 4.15.

Acceptance:

- `tool.catalog.plan` returns visible and hidden entries.
- Hidden reasons are stable and sanitized.
- Context reports aggregate tool capability counts.
- Manifest policy can reference families/toolsets without app-specific branches.

### Proposal 3: `route-tool-invocation-through-tool-service`

Purpose: make framework and service callers invoke tools through `service.tool/tool.invoke`, then route to the owning service without stealing ownership.

Scope:

- Implement `tool.invoke`, `tool.invoke.cancel`, `tool.invocation.status`, `tool.result.get`.
- Route MCP tools to `service.mcp/mcp.tool.invoke`.
- Route Skill tools to `service.skill/skill.tool.invoke`.
- Route Driver tools to `service.driver/driver.tool.invoke`.
- Route Memory/Task/Scheduler tools to their focused services.
- Add policy, approval, timeout, resource, result-budget, and redaction decorators.
- Add framework toolkit adapter from `ToolPlan` visible entries to model-visible tools.
- Preserve compatibility adapters with deprecation notes.

Design coverage:

- Design sections 4.8, 4.9, 4.12, 4.13, 4.16.

Acceptance:

- Production framework agents call tools through `SystemToolClient`.
- Owning services still own lifecycle and concrete invocation.
- Policy runs before side effects.
- Invocation audit includes trace id, scope, input hash, output hash, status, and stable reason code.
- Large results become artifact refs.

### Proposal 4: `add-tool-runtime-environments-and-gateway`

Purpose: provide industrial runtime environments and managed gateway routing as provider-backed capabilities.

Scope:

- Add environment descriptors for local workspace, local sandbox, Docker, SSH/remote, WASM host import, browser sandbox, session-scoped and per-call environments.
- Add environment health, cleanup, resource policy, artifact roots, process handles, network policy, and secret injection policy.
- Add managed gateway provider interface.
- Support gateway-backed web, browser, media, document, remote sandbox, and enterprise connector descriptors.
- Keep all gateway/provider names in config/descriptor data.

Design coverage:

- Design sections 4.10, 4.11, 4.12, 4.13.

Acceptance:

- Environment absence is structured unavailable.
- Gateway absence is structured unavailable.
- No provider-specific routing branches in OS code.
- Cleanup and health events are observable and audited.

### Proposal 5: `add-industrial-tool-observability-and-shell-diagnostics`

Purpose: complete live diagnostics, audit query, shell rendering, operator visibility, and API surfaces.

Scope:

- Add EventLog/SSE events for plan, hidden diagnostics, policy decisions, approvals, leases, invocation lifecycle, artifacts, provider health.
- Add `tool.audit.query`, `tool.provider.status`, `tool.provider.health`, `tool.policy.explain`, `tool.catalog.snapshot`.
- Add Web/CLI thin shell endpoints and frontend diagnostics.
- Add approval UI integration without moving policy into Web.
- Add audit replay tests.

Design coverage:

- Design sections 4.13, 4.14, 4.15, 8, 9.

Acceptance:

- Operators can inspect visible tools, hidden tools, provider health, invocation traces, artifacts, and audit refs.
- Frontend/Web/CLI remain shell adapters.
- Logs and UI payloads are bounded and sanitized.

### Proposal 6: `complete-industrial-tool-family-providers`

Purpose: finish the industrial capability surface so Macaca applications can actually do complex real-world work, not just see a framework.

Scope:

- Add or adapt provider-backed tool families:
  - `file`
  - `shell`
  - `browser`
  - `web`
  - `memory`
  - `knowledge`
  - `task`
  - `scheduler`
  - `skill`
  - `mcp`
  - `media`
  - `document`
  - `communication`
  - `enterprise_api`
  - `code_execution`
  - `computer_use`
  - `payment_entitlement`
- Prefer existing services, MCP, plugins, gateway providers, and runtime adapters before adding new built-ins.
- Add end-to-end application-neutral validation tasks that use multiple families in one realistic workflow.

Design coverage:

- Design sections 1, 4.5, 4.10, 4.11, 4.12, 7, 8, 9.

Acceptance:

- A Macaca-based application can run a multi-step industrial task using browser/web/file/shell/memory/task/scheduler/document/media/code-execution families.
- Tool capability expansion remains data-driven and provider-neutral.
- The live proof uses no application-specific OS code.

## Coverage Matrix

| Design Requirement | Proposal Coverage |
| --- | --- |
| Complete service-owned Tool Capability Plane | 1, 2, 3 |
| Descriptor, plan, hidden diagnostics | 1, 2 |
| Tool families and toolsets | 1, 2, 6 |
| Availability expressions | 1, 2 |
| Policy and approval | 1, 3, 5 |
| Service-owned invocation | 1, 3 |
| Runtime environments | 4, 6 |
| Managed tool gateway | 4, 6 |
| Result artifacts and budgets | 1, 3, 5 |
| Observability and audit | 1, 3, 5 |
| Context integration | 2, 5 |
| Manifest integration | 2, 3, 6 |
| WASM and SDK integration | 1, 3, 4 |
| Rich industrial tool surface | 4, 6 |
| Shell diagnostics | 5 |
| End-to-end industrial validation | 6 |

## Proposed File Structure

### OpenSpec

- Create: `openspec/changes/add-tool-capability-contracts/proposal.md`
- Create: `openspec/changes/add-tool-capability-contracts/design.md`
- Create: `openspec/changes/add-tool-capability-contracts/tasks.md`
- Create: `openspec/changes/add-tool-capability-contracts/specs/tool-capability-contracts/spec.md`
- Create: `openspec/changes/add-tool-capability-planning-service/proposal.md`
- Create: `openspec/changes/add-tool-capability-planning-service/design.md`
- Create: `openspec/changes/add-tool-capability-planning-service/tasks.md`
- Create: `openspec/changes/add-tool-capability-planning-service/specs/tool-capability-planning/spec.md`
- Create: `openspec/changes/route-tool-invocation-through-tool-service/proposal.md`
- Create: `openspec/changes/route-tool-invocation-through-tool-service/design.md`
- Create: `openspec/changes/route-tool-invocation-through-tool-service/tasks.md`
- Create: `openspec/changes/route-tool-invocation-through-tool-service/specs/tool-service-invocation/spec.md`
- Create: `openspec/changes/add-tool-runtime-environments-and-gateway/proposal.md`
- Create: `openspec/changes/add-tool-runtime-environments-and-gateway/design.md`
- Create: `openspec/changes/add-tool-runtime-environments-and-gateway/tasks.md`
- Create: `openspec/changes/add-tool-runtime-environments-and-gateway/specs/tool-runtime-environments/spec.md`
- Create: `openspec/changes/add-industrial-tool-observability-and-shell-diagnostics/proposal.md`
- Create: `openspec/changes/add-industrial-tool-observability-and-shell-diagnostics/design.md`
- Create: `openspec/changes/add-industrial-tool-observability-and-shell-diagnostics/tasks.md`
- Create: `openspec/changes/add-industrial-tool-observability-and-shell-diagnostics/specs/tool-observability/spec.md`
- Create: `openspec/changes/complete-industrial-tool-family-providers/proposal.md`
- Create: `openspec/changes/complete-industrial-tool-family-providers/design.md`
- Create: `openspec/changes/complete-industrial-tool-family-providers/tasks.md`
- Create: `openspec/changes/complete-industrial-tool-family-providers/specs/industrial-tool-families/spec.md`

### Rust Contracts and Services

- Modify: `macaca/crates/foundation/macaca-proto/src/capability_tool.rs`
  - Add or wrap industrial descriptor, plan, availability, policy, result, artifact, and audit DTOs.
- Create: `macaca/crates/foundation/macaca-proto/src/tool_service.rs`
  - Define `service.tool` command/result DTOs and constants.
- Modify: `macaca/crates/foundation/macaca-proto/src/lib.rs`
  - Export `tool_service`.
- Modify: `macaca/crates/facade/macaca-sdk/src/lib.rs`
  - Export `SystemToolClient`.
- Create: `macaca/crates/facade/macaca-sdk/src/tool_client.rs`
  - Focused facade for `service.tool`.
- Create: `macaca/crates/facade/macaca-sdk/src/tool_client_service_backed.rs`
  - Service-backed and unavailable client implementations.
- Create: `macaca/crates/runtime/macaca-runtime-host/src/tool_service_provider.rs`
  - Runtime-host provider for `service.tool`.
- Create: `macaca/crates/runtime/macaca-runtime-host/src/tool_service_provider_state.rs`
  - Provider state, snapshots, plan cache, provider status cache.
- Create: `macaca/crates/runtime/macaca-runtime-host/src/tool_service_planning.rs`
  - Tool plan builder.
- Create: `macaca/crates/runtime/macaca-runtime-host/src/tool_service_availability.rs`
  - Specification-style availability evaluator.
- Create: `macaca/crates/runtime/macaca-runtime-host/src/tool_service_policy.rs`
  - Policy and approval strategy interfaces.
- Create: `macaca/crates/runtime/macaca-runtime-host/src/tool_service_invocation.rs`
  - Invocation router and decorators.
- Create: `macaca/crates/runtime/macaca-runtime-host/src/tool_service_result.rs`
  - Result normalization, artifact refs, pagination summaries.
- Create: `macaca/crates/runtime/macaca-runtime-host/src/tool_service_audit.rs`
  - Sanitized audit mementos and audit query support.
- Create: `macaca/crates/runtime/macaca-runtime-host/src/tool_service_environment.rs`
  - Runtime environment descriptors, status, cleanup contracts.
- Create: `macaca/crates/runtime/macaca-runtime-host/src/tool_service_gateway.rs`
  - Managed gateway provider interface.
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/lib.rs`
  - Export new provider modules.

### Context, Web, CLI, Frontend

- Create: `macaca/crates/services/macaca-context/src/capability/tool_provider.rs`
  - Compact tool capability context provider.
- Modify: `macaca/crates/services/macaca-context/src/capability/mod.rs`
  - Export tool provider.
- Modify: `macaca/crates/services/macaca-context/src/service_contract.rs`
  - Include compact tool capability catalog in context inputs.
- Modify: `macaca/crates/shells/macaca-web/src/state.rs`
  - Wire `SystemToolClient`.
- Modify: `macaca/crates/shells/macaca-web/src/framework_toolkit.rs`
  - Build model-visible tools from `ToolPlan` instead of distributed direct catalog assembly.
- Create: `macaca/crates/shells/macaca-web/src/tool_service_adapter.rs`
  - Framework adapter from `ToolPlanEntry` to `macaca_framework::tool::ToolHandler`.
- Create: `macaca/crates/shells/macaca-web/src/tool_routes.rs`
  - Thin shell API routes for plan, provider status, audit, artifacts, and policy explain.
- Modify: `macaca/crates/shells/macaca-web/src/main.rs`
  - Register tool routes only as shell adapters.
- Create: `frontend/src/components/tools/ToolCapabilityPanel.tsx`
  - Render visible/hidden tools and provider health.
- Create: `frontend/src/components/tools/ToolInvocationTracePanel.tsx`
  - Render invocation lifecycle and audit refs.
- Create: `frontend/src/lib/api/tools.ts`
  - Frontend API client for Web tool routes.

### Tests and Docs

- Create: `macaca/crates/runtime/macaca-runtime-host/src/tool_service_provider_tests.rs`
- Create: `macaca/crates/runtime/macaca-runtime-host/src/tool_service_planning_tests.rs`
- Create: `macaca/crates/runtime/macaca-runtime-host/src/tool_service_invocation_tests.rs`
- Create: `macaca/crates/runtime/macaca-runtime-host/src/tool_service_environment_tests.rs`
- Create: `macaca/crates/runtime/macaca-runtime-host/src/tool_service_audit_tests.rs`
- Create: `macaca/crates/tests/macaca-integration-tests/tests/industrial_tool_system.rs`
- Create: `macaca/crates/tests/macaca-integration-tests/tests/tool_service_boundaries.rs`
- Modify: `docs/macaca-industrial-tools-system-design.md`
  - Append implementation status and OpenSpec proposal links after proposals are created.

## Task 1: Create OpenSpec Proposal `add-tool-capability-contracts`

**Files:**

- Create: `openspec/changes/add-tool-capability-contracts/proposal.md`
- Create: `openspec/changes/add-tool-capability-contracts/design.md`
- Create: `openspec/changes/add-tool-capability-contracts/tasks.md`
- Create: `openspec/changes/add-tool-capability-contracts/specs/tool-capability-contracts/spec.md`

- [ ] **Step 1: Scaffold proposal files**

Use `apply_patch` to create the four files listed above.

`proposal.md` content:

```markdown
# Change: Add industrial tool capability contracts

## Why
Macaca has service-backed Driver, Skill, MCP, Memory, Task, and runtime tool primitives, but it lacks one provider-neutral industrial Tools contract for planning, visibility diagnostics, invocation routing, result artifacts, and audit.

## What Changes
- Add provider-neutral Tool Capability DTOs for descriptors, tool plans, hidden diagnostics, tool families, toolsets, availability expressions, policy refs, result classes, artifact refs, provider status, and audit refs.
- Add `service.tool` command/result contracts for planning, snapshots, toolset resolution, invocation, cancellation, status, result retrieval, artifact access, provider health, policy explanation, and audit query.
- Add SDK `SystemToolClient` and unavailable Null Object behavior.
- Preserve ownership of Driver, Skill, MCP, Memory, Task, Gateway, Store, and other provider services.

## Impact
- Affected specs: tool-capability-contracts, sdk-system-facade, service-runtime
- Affected code: `macaca-proto`, `macaca-sdk`, `macaca-runtime-host`
```

`design.md` content:

```markdown
## Context
`docs/macaca-industrial-tools-system-design.md` defines a complete service-owned Tool Capability Plane. This change creates the contracts only. It does not migrate runtime invocation or expose new rich providers.

## Goals
- Define stable DTOs for descriptors, plans, diagnostics, availability, policy, result classes, artifacts, provider status, and audit references.
- Define `service.tool` command names and command/result DTOs.
- Add SDK focused facade and unavailable client behavior.

## Non-Goals
- Do not implement production invocation routing in this change.
- Do not move Driver, Skill, MCP, Memory, Task, or Gateway ownership into `service.tool`.
- Do not add application-specific tool families.

## Decisions
Use Command for all service operations, Facade for SDK access, Memento for plan/audit snapshots, Specification for availability expressions, and Null Object for unavailable clients.

`service.tool` coordinates tool contracts, but owning services remain authoritative for their concrete lifecycle and invocation.
```

`tasks.md` content:

```markdown
## 1. Contract DTOs
- [ ] 1.1 Extend or wrap `CapabilityToolDescriptor` for industrial metadata.
- [ ] 1.2 Add `ToolPlan`, `ToolPlanEntry`, `HiddenToolPlanEntry`, and diagnostics DTOs.
- [ ] 1.3 Add tool family, toolset, availability expression, policy ref, result class, artifact ref, provider status, and audit DTOs.

## 2. Service Contract
- [ ] 2.1 Add `macaca-proto/src/tool_service.rs` constants and typed commands/results.
- [ ] 2.2 Export `tool_service` from `macaca-proto`.
- [ ] 2.3 Add `service.tool` descriptor coverage.

## 3. SDK Facade
- [ ] 3.1 Add `SystemToolClient`.
- [ ] 3.2 Add service-backed client.
- [ ] 3.3 Add unavailable Null Object client.

## 4. Validation
- [ ] 4.1 Add unit tests for DTO serialization and unavailable client behavior.
- [ ] 4.2 Run `cargo test -p macaca-proto -- --nocapture`.
- [ ] 4.3 Run `cargo test -p macaca-sdk -- --nocapture`.
- [ ] 4.4 Run `openspec validate add-tool-capability-contracts --strict`.
```

`spec.md` content:

```markdown
## ADDED Requirements

### Requirement: Tool Capability Contracts Shall Be Provider-Neutral
Macaca SHALL define provider-neutral tool capability contracts that describe tool metadata, planning, diagnostics, policy, result classes, artifacts, and audit references without transferring runtime ownership away from owning services.

#### Scenario: Descriptor identifies owner and route without provider leakage
- **GIVEN** a Driver, Skill, MCP, Memory, Task, Gateway, or runtime tool is described
- **WHEN** a tool descriptor is serialized
- **THEN** it SHALL include stable owner, service, provider, family, schema, policy, lifecycle, result, and audit metadata
- **AND** it SHALL NOT include raw secrets, credentials, env values, raw provider payloads, prompts, or unbounded output.

### Requirement: Service Tool Commands Shall Be Typed And Trace-Required
Macaca SHALL expose `service.tool` commands as typed command/result DTOs and every command SHALL require trace context.

#### Scenario: Missing trace is rejected
- **WHEN** a caller submits a `service.tool` command without trace context
- **THEN** the command SHALL be rejected before side effects
- **AND** the result SHALL use a structured failure reason.

### Requirement: Tool Client Shall Provide Unavailable Behavior
The SDK SHALL provide `SystemToolClient` with service-backed and unavailable implementations.

#### Scenario: Tool service is absent
- **GIVEN** `service.tool` is not registered
- **WHEN** a shell or SDK caller requests a tool plan
- **THEN** the unavailable client SHALL return an explicit unavailable result
- **AND** it SHALL NOT crash, hang, silently fall back, or fake success.
```

- [ ] **Step 2: Validate proposal**

Run:

```bash
openspec validate add-tool-capability-contracts --strict
```

Expected: `Result: OK`.

- [ ] **Step 3: Commit proposal**

Run:

```bash
git add openspec/changes/add-tool-capability-contracts
git commit -m "spec: add tool capability contracts proposal"
```

## Task 2: Create OpenSpec Proposal `add-tool-capability-planning-service`

**Files:**

- Create: `openspec/changes/add-tool-capability-planning-service/proposal.md`
- Create: `openspec/changes/add-tool-capability-planning-service/design.md`
- Create: `openspec/changes/add-tool-capability-planning-service/tasks.md`
- Create: `openspec/changes/add-tool-capability-planning-service/specs/tool-capability-planning/spec.md`

- [ ] **Step 1: Scaffold proposal files**

Create the files with these contents.

`proposal.md`:

```markdown
# Change: Add tool capability planning service

## Why
Applications and agents need deterministic tool plans with visible tools, hidden diagnostics, family/toolset policy, availability reasons, conflicts, and compact context integration. Current toolkit assembly spreads this logic across shell and provider paths.

## What Changes
- Add `tool.catalog.plan`, `tool.catalog.snapshot`, and `tool.toolset.resolve` behavior.
- Add descriptor contributors for existing Driver, Skill, MCP, Memory, Task, Scheduler, workspace, and runtime tools.
- Add availability expression evaluation and stable hidden diagnostics.
- Add compact Context provider for tool capability indexes.
- Add manifest support for generic tool families and toolsets.

## Impact
- Affected specs: tool-capability-planning, context-composer
- Affected code: `macaca-runtime-host`, `macaca-context`, `macaca-app`, `macaca-web`
```

`design.md`:

```markdown
## Context
This proposal builds the planning plane from the contracts created by `add-tool-capability-contracts`. It is plan-only: invocation still uses existing service adapters until `route-tool-invocation-through-tool-service`.

## Goals
- Build deterministic `ToolPlan` snapshots.
- Separate visible and hidden tools.
- Resolve data-driven families and toolsets.
- Evaluate availability and policy diagnostics without leaking secrets.
- Feed compact tool capability indexes into Context.

## Non-Goals
- Do not route production invocation through `service.tool` yet.
- Do not implement runtime environment providers.
- Do not add managed gateway execution.

## Decisions
Use Specification for availability expressions, Strategy for conflict/toolset resolution, Memento for plan snapshots, and Adapter contributors for existing owning services.
```

`tasks.md`:

```markdown
## 1. Planning Provider
- [ ] 1.1 Add `tool_service_provider.rs`.
- [ ] 1.2 Add `tool_service_planning.rs`.
- [ ] 1.3 Add contributor interfaces for existing service descriptors.
- [ ] 1.4 Add provider status cache and plan snapshot cache.

## 2. Availability And Toolsets
- [ ] 2.1 Add `tool_service_availability.rs`.
- [ ] 2.2 Add tool family and toolset resolution strategies.
- [ ] 2.3 Add hidden diagnostics for policy, config, auth, binary, platform, service health, entitlement, and conflict failures.

## 3. Context And Manifest
- [ ] 3.1 Add compact tool capability provider in `macaca-context`.
- [ ] 3.2 Add generic manifest fields for tool families and toolsets.
- [ ] 3.3 Preserve exact `allowed_tools` compatibility.

## 4. Validation
- [ ] 4.1 Add unit tests for visible/hidden planning.
- [ ] 4.2 Add context report tests for tool capability counts.
- [ ] 4.3 Run `cargo test -p macaca-runtime-host tool_service_planning -- --nocapture`.
- [ ] 4.4 Run `cargo test -p macaca-context -- --nocapture`.
- [ ] 4.5 Run `openspec validate add-tool-capability-planning-service --strict`.
```

`spec.md`:

```markdown
## ADDED Requirements

### Requirement: Tool Planning Shall Produce Visible And Hidden Entries
Macaca SHALL convert service-owned tool descriptors, application policy, agent policy, availability, and provider health into a deterministic tool plan.

#### Scenario: Unavailable tool is hidden with reason
- **GIVEN** a tool descriptor requires a missing auth provider
- **WHEN** `tool.catalog.plan` runs
- **THEN** the tool SHALL be excluded from model-visible tools
- **AND** the hidden entry SHALL include a stable reason code such as `missing_auth`
- **AND** the diagnostic SHALL NOT expose secrets or raw provider configuration.

### Requirement: Toolsets Shall Be Data-Driven
Macaca SHALL resolve toolsets from declarative family and tool membership rules rather than application-specific code branches.

#### Scenario: Application declares research toolset
- **GIVEN** an application manifest declares the `research` toolset
- **WHEN** the plan is built for an agent
- **THEN** matching web, browser, memory, and document-capable tools SHALL be considered through data-driven rules
- **AND** no OS-layer branch SHALL depend on the application name.

### Requirement: Context Shall Include Compact Tool Capability Index
The Context service SHALL expose compact tool capability information without injecting raw tool docs or unbounded schemas by default.

#### Scenario: Context report records capability counts
- **WHEN** context composition completes
- **THEN** the report SHALL include selected, hidden, skipped, and conflicted tool counts
- **AND** model-visible context SHALL remain bounded and sanitized.
```

- [ ] **Step 2: Validate proposal**

Run:

```bash
openspec validate add-tool-capability-planning-service --strict
```

Expected: `Result: OK`.

- [ ] **Step 3: Commit proposal**

Run:

```bash
git add openspec/changes/add-tool-capability-planning-service
git commit -m "spec: add tool capability planning proposal"
```

## Task 3: Create OpenSpec Proposal `route-tool-invocation-through-tool-service`

**Files:**

- Create: `openspec/changes/route-tool-invocation-through-tool-service/proposal.md`
- Create: `openspec/changes/route-tool-invocation-through-tool-service/design.md`
- Create: `openspec/changes/route-tool-invocation-through-tool-service/tasks.md`
- Create: `openspec/changes/route-tool-invocation-through-tool-service/specs/tool-service-invocation/spec.md`

- [ ] **Step 1: Scaffold proposal files**

`proposal.md`:

```markdown
# Change: Route tool invocation through service.tool

## Why
Macaca needs one industrial invocation path for tools while preserving concrete lifecycle ownership in Driver, Skill, MCP, Memory, Task, Gateway, and runtime provider services.

## What Changes
- Implement `tool.invoke`, `tool.invoke.cancel`, `tool.invocation.status`, `tool.result.get`, and artifact-aware responses.
- Route invocations to owning services through descriptor routes.
- Add decorators for trace, policy, approval, resource admission, entitlement, timeout, result budget, redaction, telemetry, and audit.
- Migrate framework toolkit invocation to `SystemToolClient`.
- Keep compatibility adapters deprecated until all callers migrate.

## Impact
- Affected specs: tool-service-invocation, execution-control-service, service-runtime, sdk-system-facade
- Affected code: `macaca-runtime-host`, `macaca-sdk`, `macaca-web`, `macaca-framework`, provider services
```

`design.md`:

```markdown
## Context
This proposal turns the tool plan into an executable service-owned path. `service.tool` coordinates invocation but does not replace owning services. MCP calls still go through `service.mcp`, Skill calls still go through `service.skill`, Driver calls still go through `service.driver`, and so on.

## Goals
- Route all production framework tool calls through `SystemToolClient`.
- Enforce policy and resource admission before side effects.
- Normalize results into bounded inline content, artifacts, background handles, approval requests, or structured failures.
- Emit sanitized invocation audit.

## Non-Goals
- Do not add managed runtime environments beyond existing providers.
- Do not move concrete provider lifecycle into Web or SDK.
- Do not remove compatibility paths until migration coverage is complete.

## Decisions
Use Facade for `SystemToolClient`, Decorator for enforcement, Adapter for model-visible framework tools, Strategy for routing and result budget, Observer for eventing, and Memento for audit records.
```

`tasks.md`:

```markdown
## 1. Invocation Service
- [ ] 1.1 Add `tool_service_invocation.rs`.
- [ ] 1.2 Add descriptor route lookup.
- [ ] 1.3 Route MCP, Skill, Driver, Memory, Task, Scheduler, Gateway, and runtime tools to owning services.

## 2. Enforcement Decorators
- [ ] 2.1 Add policy decorator.
- [ ] 2.2 Add approval decorator.
- [ ] 2.3 Add resource and entitlement decorators.
- [ ] 2.4 Add timeout and cancellation support.
- [ ] 2.5 Add redaction and result budget decorators.

## 3. Framework Toolkit Migration
- [ ] 3.1 Add `tool_service_adapter.rs` in Web.
- [ ] 3.2 Convert `ToolPlanEntry` into framework tools.
- [ ] 3.3 Reapply manifest compatibility filtering.
- [ ] 3.4 Mark old direct assembly paths compatibility-only.

## 4. Results And Audit
- [ ] 4.1 Add `tool_service_result.rs`.
- [ ] 4.2 Add `tool_service_audit.rs`.
- [ ] 4.3 Persist large results as artifact refs.
- [ ] 4.4 Emit invocation lifecycle events.

## 5. Validation
- [ ] 5.1 Add invocation routing tests for MCP/Skill/Driver/Memory/Task tools.
- [ ] 5.2 Add policy-denied and approval-required tests.
- [ ] 5.3 Add large-result artifact tests.
- [ ] 5.4 Run `cargo test -p macaca-runtime-host tool_service_invocation -- --nocapture`.
- [ ] 5.5 Run `cargo test -p macaca-web framework_toolkit -- --nocapture`.
- [ ] 5.6 Run `openspec validate route-tool-invocation-through-tool-service --strict`.
```

`spec.md`:

```markdown
## ADDED Requirements

### Requirement: Production Tool Invocation Shall Route Through Service Tool
Macaca SHALL route production framework tool invocation through `service.tool/tool.invoke` and then to the owning service.

#### Scenario: MCP tool invokes through owning MCP service
- **GIVEN** a visible MCP tool is selected in a `ToolPlan`
- **WHEN** the model calls the tool
- **THEN** the framework adapter SHALL call `SystemToolClient::invoke`
- **AND** `service.tool` SHALL route to `service.mcp/mcp.tool.invoke`
- **AND** Web SHALL NOT own the MCP protocol client.

### Requirement: Policy Shall Run Before Side Effects
Macaca SHALL run policy, approval, resource, entitlement, timeout, and budget gates before privileged tool side effects.

#### Scenario: Write tool requires approval
- **GIVEN** a tool is classified as write-capable
- **WHEN** session policy requires approval for writes
- **THEN** `tool.invoke` SHALL return an approval request before executing the tool
- **AND** the audit log SHALL record the approval requirement without raw input leakage.

### Requirement: Tool Results Shall Be Bounded And Artifact-Aware
Macaca SHALL normalize tool results into bounded inline responses, artifact references, background handles, approval requests, or structured failures.

#### Scenario: Oversized result becomes artifact
- **GIVEN** a tool returns output larger than the configured inline result budget
- **WHEN** result normalization runs
- **THEN** the output SHALL be persisted as an artifact
- **AND** the model-visible result SHALL include a stable artifact ref and bounded summary.
```

- [ ] **Step 2: Validate proposal**

Run:

```bash
openspec validate route-tool-invocation-through-tool-service --strict
```

Expected: `Result: OK`.

- [ ] **Step 3: Commit proposal**

Run:

```bash
git add openspec/changes/route-tool-invocation-through-tool-service
git commit -m "spec: route tool invocation through service tool"
```

## Task 4: Create OpenSpec Proposal `add-tool-runtime-environments-and-gateway`

**Files:**

- Create: `openspec/changes/add-tool-runtime-environments-and-gateway/proposal.md`
- Create: `openspec/changes/add-tool-runtime-environments-and-gateway/design.md`
- Create: `openspec/changes/add-tool-runtime-environments-and-gateway/tasks.md`
- Create: `openspec/changes/add-tool-runtime-environments-and-gateway/specs/tool-runtime-environments/spec.md`

- [ ] **Step 1: Scaffold proposal files**

`proposal.md`:

```markdown
# Change: Add tool runtime environments and managed gateway

## Why
Industrial tools require controlled execution environments, process/runtime lifecycle, artifact roots, network and filesystem policy, and optional managed gateway providers. Without this layer, Macaca can plan tools but cannot safely run complex real-world work.

## What Changes
- Add runtime environment descriptors for local workspace, sandbox, Docker, SSH/remote, WASM host import, browser sandbox, per-call, and session-scoped environments.
- Add environment health, cleanup, resource policy, artifact roots, network policy, secret injection policy, and process handles.
- Add managed gateway provider interface for web, browser, media, document, remote sandbox, and enterprise connector tools.
- Keep provider names in descriptor/config data only.

## Impact
- Affected specs: tool-runtime-environments, service-runtime, serviceization-dependency-gate
- Affected code: `macaca-runtime-host`, `macaca-proto`, `macaca-sdk`, environment provider adapters
```

`design.md`:

```markdown
## Context
This proposal adds the environment and gateway execution substrate required by industrial tools. It depends on `service.tool` contracts and invocation routing.

## Goals
- Model runtime environments as provider-backed capabilities.
- Support health, cleanup, artifact roots, process handles, secret injection policy, network policy, and resource policy.
- Add optional managed gateway provider registration.

## Non-Goals
- Do not make a specific gateway mandatory.
- Do not hardcode provider names in OS routing.
- Do not move tool semantics into shell code.

## Decisions
Use Abstract Factory for provider bootstrapping, Strategy for provider routing, State for environment lifecycle, Decorator for resource/secret/network policy, and Null Object for unavailable environments.
```

`tasks.md`:

```markdown
## 1. Environment Contracts
- [ ] 1.1 Add environment descriptor DTOs.
- [ ] 1.2 Add environment health and cleanup DTOs.
- [ ] 1.3 Add artifact root and process handle DTOs.

## 2. Runtime Host Providers
- [ ] 2.1 Add `tool_service_environment.rs`.
- [ ] 2.2 Add local workspace environment adapter.
- [ ] 2.3 Add local sandbox environment adapter.
- [ ] 2.4 Add provider seams for Docker, SSH/remote, WASM host import, browser sandbox, per-call, and session-scoped environments.

## 3. Managed Gateway
- [ ] 3.1 Add `tool_service_gateway.rs`.
- [ ] 3.2 Add gateway descriptor registration.
- [ ] 3.3 Add gateway health and unavailable diagnostics.
- [ ] 3.4 Add metering/audit hooks.

## 4. Validation
- [ ] 4.1 Add environment health tests.
- [ ] 4.2 Add cleanup tests.
- [ ] 4.3 Add gateway unavailable tests.
- [ ] 4.4 Run `cargo test -p macaca-runtime-host tool_service_environment -- --nocapture`.
- [ ] 4.5 Run `openspec validate add-tool-runtime-environments-and-gateway --strict`.
```

`spec.md`:

```markdown
## ADDED Requirements

### Requirement: Tool Runtime Environments Shall Be Provider-Backed
Macaca SHALL model tool runtime environments as provider-backed capabilities with health, cleanup, resource policy, artifact roots, process handles, network policy, and secret injection policy.

#### Scenario: Environment is unavailable
- **GIVEN** a tool requires a sandbox environment
- **AND** no sandbox provider is available
- **WHEN** the tool plan or invocation is evaluated
- **THEN** Macaca SHALL return a structured unavailable diagnostic
- **AND** it SHALL NOT crash, hang, silently fall back, or fake success.

### Requirement: Managed Gateway Shall Be Optional And Audited
Macaca SHALL support managed gateway providers as optional tool providers.

#### Scenario: Gateway routes a web extraction tool
- **GIVEN** a gateway provider registers a web extraction descriptor
- **WHEN** policy selects the gateway route
- **THEN** invocation SHALL pass through service policy, metering, and audit
- **AND** provider-specific names SHALL remain descriptor/config data rather than OS control-flow branches.
```

- [ ] **Step 2: Validate proposal**

Run:

```bash
openspec validate add-tool-runtime-environments-and-gateway --strict
```

Expected: `Result: OK`.

- [ ] **Step 3: Commit proposal**

Run:

```bash
git add openspec/changes/add-tool-runtime-environments-and-gateway
git commit -m "spec: add tool runtime environments and gateway"
```

## Task 5: Create OpenSpec Proposal `add-industrial-tool-observability-and-shell-diagnostics`

**Files:**

- Create: `openspec/changes/add-industrial-tool-observability-and-shell-diagnostics/proposal.md`
- Create: `openspec/changes/add-industrial-tool-observability-and-shell-diagnostics/design.md`
- Create: `openspec/changes/add-industrial-tool-observability-and-shell-diagnostics/tasks.md`
- Create: `openspec/changes/add-industrial-tool-observability-and-shell-diagnostics/specs/tool-observability/spec.md`

- [ ] **Step 1: Scaffold proposal files**

`proposal.md`:

```markdown
# Change: Add industrial tool observability and shell diagnostics

## Why
Industrial tool execution must be traceable, auditable, explainable, and operable. Users and operators need visible/hidden tools, provider health, policy decisions, approvals, invocation lifecycle, artifacts, and replayable audit references without moving semantics into shells.

## What Changes
- Add sanitized EventLog/SSE events for planning, hidden diagnostics, policy, approval, leases, invocation lifecycle, artifacts, and provider health.
- Add `tool.audit.query`, `tool.provider.status`, `tool.provider.health`, `tool.policy.explain`, and `tool.catalog.snapshot`.
- Add Web/CLI thin shell routes and frontend panels.
- Add audit replay tests.

## Impact
- Affected specs: tool-observability, web-cli-thin-shell-v0, web-cli-thin-shell-completion
- Affected code: `macaca-runtime-host`, `macaca-web`, `frontend/`, CLI shell adapters
```

`design.md`:

```markdown
## Context
This proposal completes operator visibility for the industrial Tools system. It builds on planning, invocation, environments, and gateway contracts.

## Goals
- Expose bounded and sanitized diagnostic surfaces.
- Keep Web/CLI/frontend as shell adapters.
- Provide audit replay for plan and invocation decisions.
- Render approval and artifact states without owning policy.

## Non-Goals
- Do not let Web/CLI make policy decisions.
- Do not expose raw provider payloads, secrets, prompts, or unbounded output.
- Do not implement new provider families in this proposal.

## Decisions
Use Observer for live events, Memento for audit records, Facade for shell API clients, and Adapter for frontend rendering.
```

`tasks.md`:

```markdown
## 1. Event And Audit Surface
- [ ] 1.1 Add planning events.
- [ ] 1.2 Add hidden diagnostic events.
- [ ] 1.3 Add policy and approval events.
- [ ] 1.4 Add invocation lifecycle events.
- [ ] 1.5 Add artifact and provider health events.

## 2. API And Shell Routes
- [ ] 2.1 Add Web `tool_routes.rs`.
- [ ] 2.2 Add SDK calls for status, health, policy explain, snapshot, and audit query.
- [ ] 2.3 Add CLI command adapters if CLI exposes tool diagnostics.

## 3. Frontend
- [ ] 3.1 Add `ToolCapabilityPanel`.
- [ ] 3.2 Add `ToolInvocationTracePanel`.
- [ ] 3.3 Add `frontend/src/lib/api/tools.ts`.

## 4. Validation
- [ ] 4.1 Add audit replay tests.
- [ ] 4.2 Add Web route tests.
- [ ] 4.3 Add frontend type/lint checks.
- [ ] 4.4 Run `cargo test -p macaca-runtime-host tool_service_audit -- --nocapture`.
- [ ] 4.5 Run `cargo test -p macaca-web tool_routes -- --nocapture`.
- [ ] 4.6 Run `cd frontend && npm run lint`.
- [ ] 4.7 Run `openspec validate add-industrial-tool-observability-and-shell-diagnostics --strict`.
```

`spec.md`:

```markdown
## ADDED Requirements

### Requirement: Tool Events Shall Be Observable And Sanitized
Macaca SHALL emit bounded sanitized events for tool planning, diagnostics, policy, approvals, resource leases, invocation lifecycle, artifacts, and provider health.

#### Scenario: Invocation is visible live
- **WHEN** a tool invocation starts, progresses, completes, fails, or is cancelled
- **THEN** EventLog and live SSE SHALL expose a sanitized lifecycle event
- **AND** raw secrets, prompts, raw provider payloads, and unbounded output SHALL NOT be emitted.

### Requirement: Shells Shall Render Diagnostics Without Owning Semantics
Web, CLI, and frontend SHALL render tool plans, hidden diagnostics, provider health, policy explanations, approval state, artifacts, and audit refs through SDK/service clients only.

#### Scenario: Web shows hidden tool reason
- **GIVEN** a tool is hidden because its provider is unavailable
- **WHEN** the user opens tool diagnostics
- **THEN** Web SHALL display the stable reason and remediation hint
- **AND** Web SHALL NOT contain provider lifecycle or policy decision logic.
```

- [ ] **Step 2: Validate proposal**

Run:

```bash
openspec validate add-industrial-tool-observability-and-shell-diagnostics --strict
```

Expected: `Result: OK`.

- [ ] **Step 3: Commit proposal**

Run:

```bash
git add openspec/changes/add-industrial-tool-observability-and-shell-diagnostics
git commit -m "spec: add industrial tool observability"
```

## Task 6: Create OpenSpec Proposal `complete-industrial-tool-family-providers`

**Files:**

- Create: `openspec/changes/complete-industrial-tool-family-providers/proposal.md`
- Create: `openspec/changes/complete-industrial-tool-family-providers/design.md`
- Create: `openspec/changes/complete-industrial-tool-family-providers/tasks.md`
- Create: `openspec/changes/complete-industrial-tool-family-providers/specs/industrial-tool-families/spec.md`

- [ ] **Step 1: Scaffold proposal files**

`proposal.md`:

```markdown
# Change: Complete industrial tool family providers

## Why
The Tools system is only industrial-grade if Macaca applications can perform real multi-step work through rich, generic, service-owned tool families. Contracts and routing alone are insufficient.

## What Changes
- Add or adapt provider-backed families for file, shell, browser, web, memory, knowledge, task, scheduler, skill, MCP, media, document, communication, enterprise API, code execution, computer use, and payment/entitlement.
- Prefer existing services, MCP, plugins, gateway providers, and runtime adapters before adding new built-ins.
- Add end-to-end application-neutral validation that uses multiple families in one realistic workflow.

## Impact
- Affected specs: industrial-tool-families, tool-capability-planning, tool-service-invocation, tool-runtime-environments, tool-observability
- Affected code: provider service adapters, MCP/plugin/gateway adapters, integration tests, docs
```

`design.md`:

```markdown
## Context
Previous proposals create contracts, planning, invocation, environments, gateway, and diagnostics. This proposal fills out the actual application-neutral industrial tool surface.

## Goals
- Provide rich generic tool families.
- Use existing services and external extension points where possible.
- Prove a realistic multi-family industrial workflow.

## Non-Goals
- Do not hardcode application-specific business logic.
- Do not force one provider for every family.
- Do not bypass service runtime, policy, trace, or audit.

## Decisions
Use Adapter/Bridge for existing services, MCP, plugins, and gateway providers; Strategy for provider selection; Abstract Factory for provider bootstrapping; and Null Object for absent optional providers.
```

`tasks.md`:

```markdown
## 1. Provider Inventory
- [ ] 1.1 Inventory existing file, shell, browser, web, memory, knowledge, task, scheduler, skill, MCP, media, document, communication, enterprise API, code execution, computer use, and payment/entitlement providers.
- [ ] 1.2 Map each provider to an owning service and tool family.
- [ ] 1.3 Record missing providers as structured unavailable adapters or gateway/MCP/plugin extension points.

## 2. Family Completion
- [ ] 2.1 Add descriptors for file and shell families.
- [ ] 2.2 Add descriptors for browser and web families.
- [ ] 2.3 Add descriptors for memory and knowledge families.
- [ ] 2.4 Add descriptors for task and scheduler families.
- [ ] 2.5 Add descriptors for skill and MCP families.
- [ ] 2.6 Add descriptors for media and document families.
- [ ] 2.7 Add descriptors for communication and enterprise API families.
- [ ] 2.8 Add descriptors for code execution, computer use, and payment/entitlement families.

## 3. Live Industrial Proof
- [ ] 3.1 Create an application-neutral test manifest using multiple tool families.
- [ ] 3.2 Run a realistic task that requires research, file operations, shell/code execution, memory recall, document/artifact handling, and scheduled follow-up.
- [ ] 3.3 Capture stable refs and aggregate counts only.
- [ ] 3.4 Verify audit replay and artifact refs.

## 4. Validation
- [ ] 4.1 Add provider family unit tests.
- [ ] 4.2 Add integration tests in `industrial_tool_system.rs`.
- [ ] 4.3 Add boundary tests in `tool_service_boundaries.rs`.
- [ ] 4.4 Run `cargo test -p macaca-integration-tests industrial_tool_system -- --nocapture`.
- [ ] 4.5 Run `cargo test -p macaca-integration-tests tool_service_boundaries -- --nocapture`.
- [ ] 4.6 Run `openspec validate complete-industrial-tool-family-providers --strict`.
```

`spec.md`:

```markdown
## ADDED Requirements

### Requirement: Industrial Tool Families Shall Cover Real Complex Work
Macaca SHALL provide application-neutral provider-backed tool families for file, shell, browser, web, memory, knowledge, task, scheduler, skill, MCP, media, document, communication, enterprise API, code execution, computer use, and payment/entitlement.

#### Scenario: Multi-family task completes through generic tools
- **GIVEN** an application-neutral agent task requires research, browser/web access, file work, shell or code execution, memory recall, document/artifact handling, and scheduled follow-up
- **WHEN** the task runs through Macaca
- **THEN** the agent SHALL complete the task through planned visible tools
- **AND** every invoked tool SHALL pass service-owned policy, trace, result, and audit handling.

### Requirement: Missing Optional Families Shall Be Explicit
Optional providers SHALL return structured unavailable, disabled, unsupported, or denied states when absent.

#### Scenario: Document provider is absent
- **GIVEN** the document family is requested
- **AND** no document provider, plugin, MCP server, or gateway route is available
- **WHEN** the tool plan is built
- **THEN** document tools SHALL appear as hidden diagnostics or unavailable provider summaries
- **AND** Macaca SHALL NOT fake success.
```

- [ ] **Step 2: Validate proposal**

Run:

```bash
openspec validate complete-industrial-tool-family-providers --strict
```

Expected: `Result: OK`.

- [ ] **Step 3: Commit proposal**

Run:

```bash
git add openspec/changes/complete-industrial-tool-family-providers
git commit -m "spec: complete industrial tool family providers"
```

## Task 7: Implement Proposal 1 After Approval

**Files:**

- Modify: `macaca/crates/foundation/macaca-proto/src/capability_tool.rs`
- Create: `macaca/crates/foundation/macaca-proto/src/tool_service.rs`
- Modify: `macaca/crates/foundation/macaca-proto/src/lib.rs`
- Create: `macaca/crates/facade/macaca-sdk/src/tool_client.rs`
- Create: `macaca/crates/facade/macaca-sdk/src/tool_client_service_backed.rs`
- Modify: `macaca/crates/facade/macaca-sdk/src/lib.rs`

- [ ] **Step 1: Run GitNexus impact analysis notes**

Run impact checks for existing symbols before editing them. Record `CRITICAL` and `HIGH` output in the task notes but do not block solely on those warnings.

```bash
npx gitnexus analyze
```

Expected: repository index refresh completes or reports current status.

- [ ] **Step 2: Write contract tests first**

Create tests covering:

- descriptor serialization redacts sensitive values by construction
- `ToolPlan` serializes visible and hidden entries
- unavailable `SystemToolClient` returns structured unavailable

Run:

```bash
cargo test -p macaca-proto tool_service -- --nocapture
cargo test -p macaca-sdk tool_client -- --nocapture
```

Expected: initial failures for missing types.

- [ ] **Step 3: Implement DTOs with English comments**

Add DTOs in focused modules. Each struct with non-obvious security or lifecycle semantics must include an English doc comment explaining what it owns and what it must not contain.

- [ ] **Step 4: Implement SDK client**

Add focused service-backed and unavailable clients. The unavailable client must log structured unavailable diagnostics without pretending success.

- [ ] **Step 5: Verify**

Run:

```bash
cargo test -p macaca-proto -- --nocapture
cargo test -p macaca-sdk -- --nocapture
openspec validate add-tool-capability-contracts --strict
git diff --check
```

Expected: all pass.

- [ ] **Step 6: Commit**

Run:

```bash
git add macaca/crates/foundation/macaca-proto macaca/crates/facade/macaca-sdk openspec/changes/add-tool-capability-contracts
git commit -m "feat: add tool capability contracts"
```

## Task 8: Implement Proposal 2 After Proposal 1 Is Merged

**Files:**

- Create: `macaca/crates/runtime/macaca-runtime-host/src/tool_service_provider.rs`
- Create: `macaca/crates/runtime/macaca-runtime-host/src/tool_service_provider_state.rs`
- Create: `macaca/crates/runtime/macaca-runtime-host/src/tool_service_planning.rs`
- Create: `macaca/crates/runtime/macaca-runtime-host/src/tool_service_availability.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/lib.rs`
- Create: `macaca/crates/services/macaca-context/src/capability/tool_provider.rs`
- Modify: `macaca/crates/services/macaca-context/src/capability/mod.rs`
- Modify: `macaca/crates/services/macaca-context/src/service_contract.rs`

- [ ] **Step 1: Write planning tests first**

Tests must cover visible tools, hidden diagnostics, conflicts, missing auth, missing service, and exact `allowed_tools` compatibility.

Run:

```bash
cargo test -p macaca-runtime-host tool_service_planning -- --nocapture
```

Expected: fail before implementation.

- [ ] **Step 2: Implement plan builder**

Implement descriptor contributors as adapters. Driver, Skill, MCP, Memory, Task, Scheduler, workspace, and runtime tools must remain owned by their existing services.

- [ ] **Step 3: Implement availability evaluator**

Use Specification objects for config, secret, auth, env, binary, service health, platform, resource, entitlement, plugin, manifest, agent policy, and session-context signals.

- [ ] **Step 4: Add compact context provider**

Context must include bounded counts and capability indexes only. It must not inject raw tool docs or provider payloads.

- [ ] **Step 5: Verify**

Run:

```bash
cargo test -p macaca-runtime-host tool_service_planning -- --nocapture
cargo test -p macaca-context -- --nocapture
openspec validate add-tool-capability-planning-service --strict
git diff --check
```

Expected: all pass.

- [ ] **Step 6: Commit**

Run:

```bash
git add macaca/crates/runtime/macaca-runtime-host macaca/crates/services/macaca-context openspec/changes/add-tool-capability-planning-service
git commit -m "feat: add tool capability planning service"
```

## Task 9: Implement Proposal 3 After Proposal 2 Is Merged

**Files:**

- Create: `macaca/crates/runtime/macaca-runtime-host/src/tool_service_invocation.rs`
- Create: `macaca/crates/runtime/macaca-runtime-host/src/tool_service_policy.rs`
- Create: `macaca/crates/runtime/macaca-runtime-host/src/tool_service_result.rs`
- Create: `macaca/crates/runtime/macaca-runtime-host/src/tool_service_audit.rs`
- Create: `macaca/crates/shells/macaca-web/src/tool_service_adapter.rs`
- Modify: `macaca/crates/shells/macaca-web/src/framework_toolkit.rs`
- Modify: `macaca/crates/shells/macaca-web/src/state.rs`

- [ ] **Step 1: Write invocation tests first**

Tests must prove:

- MCP route calls `SystemMcpClient`.
- Skill route calls `SystemSkillClient`.
- Driver route calls `SystemDriverClient`.
- Policy denied returns structured denied result.
- Oversized output returns artifact ref.

Run:

```bash
cargo test -p macaca-runtime-host tool_service_invocation -- --nocapture
```

Expected: fail before implementation.

- [ ] **Step 2: Implement invocation router**

Route through owning services only. Do not parse provider-specific names or construct providers in the router.

- [ ] **Step 3: Implement enforcement decorators**

Add trace, policy, approval, resource, entitlement, timeout, result-budget, redaction, telemetry, and audit decorators.

- [ ] **Step 4: Migrate framework toolkit adapter**

`framework_toolkit.rs` should consume `ToolPlan` visible entries and register service-backed tool adapters. Compatibility paths must be marked as deprecated or limited to fallback tests.

- [ ] **Step 5: Verify**

Run:

```bash
cargo test -p macaca-runtime-host tool_service_invocation -- --nocapture
cargo test -p macaca-web framework_toolkit -- --nocapture
openspec validate route-tool-invocation-through-tool-service --strict
git diff --check
```

Expected: all pass.

- [ ] **Step 6: Commit**

Run:

```bash
git add macaca/crates/runtime/macaca-runtime-host macaca/crates/shells/macaca-web openspec/changes/route-tool-invocation-through-tool-service
git commit -m "feat: route tool invocation through service tool"
```

## Task 10: Implement Proposal 4 After Proposal 3 Is Merged

**Files:**

- Create: `macaca/crates/runtime/macaca-runtime-host/src/tool_service_environment.rs`
- Create: `macaca/crates/runtime/macaca-runtime-host/src/tool_service_gateway.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/tool_service_provider.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/lib.rs`

- [ ] **Step 1: Write environment and gateway tests first**

Tests must cover health, cleanup, unavailable environments, gateway absence, metering hook emission, and sanitized diagnostics.

Run:

```bash
cargo test -p macaca-runtime-host tool_service_environment -- --nocapture
```

Expected: fail before implementation.

- [ ] **Step 2: Implement environment descriptors**

Model local workspace, local sandbox, Docker, SSH/remote, WASM host import, browser sandbox, per-call, and session-scoped environments as provider-backed descriptors.

- [ ] **Step 3: Implement gateway provider interface**

Gateway providers register descriptors and health. Provider names must remain descriptor/config data.

- [ ] **Step 4: Verify**

Run:

```bash
cargo test -p macaca-runtime-host tool_service_environment -- --nocapture
openspec validate add-tool-runtime-environments-and-gateway --strict
git diff --check
```

Expected: all pass.

- [ ] **Step 5: Commit**

Run:

```bash
git add macaca/crates/runtime/macaca-runtime-host openspec/changes/add-tool-runtime-environments-and-gateway
git commit -m "feat: add tool runtime environments and gateway"
```

## Task 11: Implement Proposal 5 After Proposal 4 Is Merged

**Files:**

- Create: `macaca/crates/shells/macaca-web/src/tool_routes.rs`
- Modify: `macaca/crates/shells/macaca-web/src/main.rs`
- Create: `frontend/src/components/tools/ToolCapabilityPanel.tsx`
- Create: `frontend/src/components/tools/ToolInvocationTracePanel.tsx`
- Create: `frontend/src/lib/api/tools.ts`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/tool_service_audit.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/tool_service_provider.rs`

- [ ] **Step 1: Write route and audit tests first**

Run:

```bash
cargo test -p macaca-runtime-host tool_service_audit -- --nocapture
cargo test -p macaca-web tool_routes -- --nocapture
```

Expected: fail before implementation.

- [ ] **Step 2: Implement sanitized event and audit surfaces**

Add planning, hidden diagnostic, policy, approval, lease, invocation, artifact, and provider health events. All logs must be bounded and sanitized.

- [ ] **Step 3: Implement Web routes**

Routes must call `SystemToolClient` only. Web must not evaluate policy or own provider lifecycle.

- [ ] **Step 4: Implement frontend panels**

Panels render visible tools, hidden diagnostics, provider health, invocation lifecycle, artifact refs, and audit refs. They must not duplicate system policy.

- [ ] **Step 5: Verify**

Run:

```bash
cargo test -p macaca-runtime-host tool_service_audit -- --nocapture
cargo test -p macaca-web tool_routes -- --nocapture
cd frontend && npm run lint
openspec validate add-industrial-tool-observability-and-shell-diagnostics --strict
git diff --check
```

Expected: all pass.

- [ ] **Step 6: Commit**

Run:

```bash
git add macaca/crates/runtime/macaca-runtime-host macaca/crates/shells/macaca-web frontend openspec/changes/add-industrial-tool-observability-and-shell-diagnostics
git commit -m "feat: add industrial tool observability"
```

## Task 12: Implement Proposal 6 After Proposal 5 Is Merged

**Files:**

- Modify or add provider adapters under the owning services identified during inventory.
- Create: `macaca/crates/tests/macaca-integration-tests/tests/industrial_tool_system.rs`
- Create: `macaca/crates/tests/macaca-integration-tests/tests/tool_service_boundaries.rs`
- Modify: `docs/macaca-industrial-tools-system-design.md`

- [ ] **Step 1: Inventory providers**

Create a provider inventory in the proposal task notes. For every family, record owning service and whether the first implementation is existing service, MCP, plugin, gateway, built-in adapter, or unavailable provider:

```text
file -> service.tool/workspace adapter or existing runtime tool
shell -> service.tool/environment-backed runtime adapter
browser -> MCP/plugin/gateway/provider adapter
web -> MCP/plugin/gateway/provider adapter
memory -> service.memory
knowledge -> context/memory/knowledge digest services
task -> service.task and execution-control services
scheduler -> service.scheduler / scheduled-agent-task
skill -> service.skill
mcp -> service.mcp
media -> plugin/MCP/gateway/provider adapter
document -> plugin/MCP/gateway/provider adapter
communication -> service.gateway
enterprise_api -> MCP/plugin/gateway/provider adapter
code_execution -> environment-backed runtime adapter
computer_use -> driver/plugin/provider adapter
payment_entitlement -> entitlement/payment services
```

- [ ] **Step 2: Add missing family descriptors**

Implement descriptors and provider adapters only through owning services or provider extension seams. Missing optional providers return unavailable diagnostics.

- [ ] **Step 3: Add integration proof**

Add an application-neutral multi-family task test. The test must prove planning, invocation, artifact refs, audit, and no raw output leakage.

- [ ] **Step 4: Verify**

Run:

```bash
cargo test -p macaca-integration-tests industrial_tool_system -- --nocapture
cargo test -p macaca-integration-tests tool_service_boundaries -- --nocapture
openspec validate complete-industrial-tool-family-providers --strict
git diff --check
```

Expected: all pass.

- [ ] **Step 5: Commit**

Run:

```bash
git add macaca/crates openspec/changes/complete-industrial-tool-family-providers docs/macaca-industrial-tools-system-design.md
git commit -m "feat: complete industrial tool family providers"
```

## Task 13: Final Cross-Proposal Verification

**Files:**

- Read: all six `openspec/changes/*/tasks.md`
- Read: `docs/macaca-industrial-tools-system-design.md`
- Read: `macaca/docs/macaca-os-architecture-governance.md`
- Read: `macaca/docs/macaca-os-microkernel-boundaries.md`
- Read: `macaca/docs/macaca-os-serviceization-allowlist.md`

- [ ] **Step 1: Run OpenSpec validation for all six proposals**

Run:

```bash
openspec validate add-tool-capability-contracts --strict
openspec validate add-tool-capability-planning-service --strict
openspec validate route-tool-invocation-through-tool-service --strict
openspec validate add-tool-runtime-environments-and-gateway --strict
openspec validate add-industrial-tool-observability-and-shell-diagnostics --strict
openspec validate complete-industrial-tool-family-providers --strict
```

Expected: every command returns `Result: OK`.

- [ ] **Step 2: Run targeted Rust checks**

Run:

```bash
cargo test -p macaca-proto -- --nocapture
cargo test -p macaca-sdk -- --nocapture
cargo test -p macaca-runtime-host tool_service -- --nocapture
cargo test -p macaca-context -- --nocapture
cargo test -p macaca-web tool -- --nocapture
cargo test -p macaca-integration-tests industrial_tool_system -- --nocapture
cargo test -p macaca-integration-tests tool_service_boundaries -- --nocapture
```

Expected: all pass.

- [ ] **Step 3: Run shell/frontend checks**

Run:

```bash
cd frontend && npm run lint
```

Expected: lint passes.

- [ ] **Step 4: Run boundary and GitNexus detection**

Run:

```bash
cargo test -p macaca-integration-tests serviceization_escape_hatches -- --nocapture
npx gitnexus detect-changes
```

Expected: boundary tests pass. GitNexus `CRITICAL` and `HIGH` output is recorded as notes per user instruction and does not block by itself.

- [ ] **Step 5: Run live application-neutral proof**

Start the local backend if needed:

```bash
cd macaca
cargo run -p macaca-web --bin macaca-web-server -- --port 3001
```

Run an application-neutral task that requires research, browser/web, file, shell/code execution, memory recall, artifact handling, and scheduled follow-up through a Macaca application that declares generic tool families.

Expected evidence:

- stable session id
- `ToolPlan` visible and hidden aggregate counts
- invocation audit refs
- artifact refs
- provider health summary
- no raw model output in the report
- no raw provider payload in EventLog/SSE/audit

- [ ] **Step 6: Final commit**

Run:

```bash
git status --short
git add docs openspec macaca frontend
git commit -m "test: validate industrial tools system end to end"
```

Expected: commit includes only expected verification/docs/test updates.

## Execution Notes

- Do not implement proposal 2 before proposal 1 is reviewed and merged.
- Do not implement proposal 3 before proposal 2 proves deterministic planning.
- Do not implement proposal 6 until contracts, planning, invocation, environments, gateway, and observability exist.
- Do not treat a successful catalog plan as completion; industrial readiness requires real invocation, artifact handling, telemetry, audit, shell diagnostics, and multi-family live proof.
- Do not add application-specific shortcuts to make the live proof pass.
- Prefer existing services, MCP, plugins, and gateway adapters over new built-ins. Add built-ins only when the capability is truly OS-generic.
- Keep each Rust file under 500 lines. Split modules by ownership when a module grows.
- Add English comments for every non-obvious security, lifecycle, policy, trace, or routing decision.
- Add structured `tracing::info!`, `tracing::warn!`, or audit records at key execution nodes. Logs must use IDs, counts, hashes, refs, and reason codes rather than raw secrets or raw payloads.
