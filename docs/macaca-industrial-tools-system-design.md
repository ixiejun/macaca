# Macaca Industrial Tools System Design

Date: 2026-05-26

## 1. Goal

Macaca needs an industrial-grade Tools system, not a larger list of callable
functions. A Macaca-based application should be able to perform real complex
work such as software engineering, browser operation, document processing,
data retrieval, media analysis, system automation, remote execution,
multi-agent delegation, scheduled follow-up, memory access, and enterprise API
operations through generic OS capabilities.

The target design is a full **Tool Capability Plane** owned by system services:

```text
Application intent / agent task
  -> Context and capability planning
  -> Tool plan with visible and hidden diagnostics
  -> Policy, approval, entitlement, resource, and budget gates
  -> Service-owned invocation
  -> Runtime environment or managed gateway
  -> Sanitized result, durable artifact, trace, telemetry, and audit
```

This design follows:

- `macaca/docs/macaca-os-architecture-governance.md`
- `macaca/docs/macaca-os-microkernel-boundaries.md`
- `macaca/docs/macaca-os-serviceization-allowlist.md`
- `macaca/docs/design_patterns.md`

It is application-neutral. It does not hardcode product workflows, application
names, provider names, model names, driver names, chain names, or business
logic into OS-layer control flow.

## 2. Reference Findings

### 2.1 Hermes Agent

Hermes treats tools as registered runtime capabilities:

- `tools/registry.py` is a central self-registration registry for schema,
  handler, toolset, availability checks, async dispatch, and error wrapping.
- `toolsets.py` composes tools into scenario-oriented tool families such as
  web, browser, file, terminal, memory, skills, delegation, cron, vision, image
  generation, and code execution.
- `check_fn` hides tools when credentials, binaries, services, or runtime
  dependencies are unavailable.
- Tool execution supports approval, guardrails, concurrent dispatch, result
  budget enforcement, oversized result persistence, and gateway-backed managed
  providers.
- Runtime environments are pluggable: local, Docker, SSH, Singularity, Modal,
  Daytona, Vercel sandbox, and managed/cloud backends.

The useful lesson is not Python registration itself. The lesson is that tools
must be grouped, dynamically available, policy-filtered, environment-aware,
budgeted, and observable.

### 2.2 OpenClaw

OpenClaw models tool exposure with stronger control-plane primitives:

- `ToolDescriptor` carries owner, executor, input schema, output schema,
  availability expression, annotations, and sort key.
- `buildToolPlan` separates visible tools from hidden tools with diagnostics.
- Plugin tools are filtered by allowlists, denylists, manifest availability,
  auth/config/env signals, descriptor cache, and factory timing traces.
- Trusted tool policies can block a call, adjust parameters, or require
  approval before execution.
- The agent runner normalizes malformed tool names, repairs malformed tool
  arguments, and sanitizes tool-call / tool-result pairing.

The useful lesson is that a model-visible tool list must be a planned,
diagnostic-bearing snapshot. Hidden tools and denied tools should be explainable
without leaking secrets or provider internals.

### 2.3 Macaca Current Base

Macaca already has several correct primitives:

- `CapabilityToolDescriptor`, `CapabilityToolInvocation`, and
  `CapabilityToolInvocationResult` in `macaca-proto`.
- Service-backed Driver, Skill, and MCP catalog/invoke paths.
- `service_tool_adapter.rs`, which adapts service descriptors into framework
  tools without letting Web own service semantics.
- `macaca-tools` command/schema/middleware primitives and standard trace
  middleware.
- Context capability catalogs for skills, MCP, and runtime tool names.
- MCP service-backed invocation design and partial implementation direction.

The gap is not lack of primitives. The gap is that the primitives are still
distributed across toolkit assembly, Driver, Skill, MCP, Context, and legacy
runtime tools without a single industrial Tool Capability Plane.

## 3. Design Principles

1. Tools are OS capabilities, not application features.
2. Every tool belongs to an owning service or provider adapter.
3. Tool visibility is a plan, not a raw registry dump.
4. Invocation requires trace, scope, policy, and audit.
5. Tool output is bounded; large results become durable artifacts.
6. Absence is structured: unavailable, unsupported, denied, or failed.
7. Tool providers are replaceable: built-in, plugin, remote, managed gateway,
   mock, and unavailable providers use the same contract.
8. Context sees compact capability indexes by default, not unbounded tool docs
   or raw provider payloads.
9. Shells render and adapt. They do not own tool semantics.
10. Applications declare required capabilities and policy; they do not force
    OS-layer hardcoded branches.

## 4. Proposed Architecture

### 4.1 Layered Ownership

| Layer | Responsibility |
| --- | --- |
| Kernel | identity, service registry, service-call invariants, trace/audit primitives, policy facade hooks |
| Service Runtime | typed command dispatch, lifecycle, health, snapshots, decorators, local/remote/plugin service transport |
| Tool Capability Service | tool plan, toolset resolution, policy admission, invocation routing, budget, artifact handling, telemetry |
| Provider Services | Driver, Skill, MCP, Gateway, Memory, Task, Store, Browser, Shell, File, Media, Web, Code Execution |
| Application Framework | manifests, app capability declarations, agent policy, ABI, WASM/YAML adapters |
| SDK/SystemFacade | focused clients and generic service-call facade |
| Shells | Web/CLI/gateway rendering, approvals, diagnostics, event subscription |

The Tool Capability Service does not replace Driver, Skill, MCP, Memory, or
Gateway. It coordinates them through descriptors and commands.

### 4.2 Core Service: `service.tool`

`service.tool` is the industrial Tools control and invocation plane.

Required commands:

- `tool.catalog.plan`
- `tool.catalog.snapshot`
- `tool.toolset.resolve`
- `tool.invoke`
- `tool.invoke.cancel`
- `tool.invocation.status`
- `tool.result.get`
- `tool.artifact.open`
- `tool.provider.status`
- `tool.provider.health`
- `tool.policy.explain`
- `tool.audit.query`

Required properties:

- All commands require `TraceContext`.
- All invocation commands require application id, session id, agent name, and
  optional task/goal ids.
- All returned diagnostics must be sanitized and bounded.
- No command may expose raw secrets, raw prompts, raw provider payloads,
  private keys, credentials, headers, env values, or unbounded output.

### 4.3 Tool Descriptor

Macaca should extend the current `CapabilityToolDescriptor` into an industrial
descriptor family. The descriptor remains metadata only; it never transfers
runtime ownership away from the provider.

Required fields:

- stable tool id
- model-visible tool name
- display title
- description
- input schema
- output schema or output shape hints
- owning service id
- provider id
- executor route
- origin kind
- tool family
- toolset membership
- availability expression
- required permission hints
- resource scope hints
- side-effect classification
- approval profile
- result budget profile
- artifact policy
- conflict namespace
- trust level
- lifecycle scope
- telemetry labels
- sanitized metadata

Descriptor ownership:

- Built-in runtime tools are owned by `service.tool` providers.
- Driver tools remain owned by `service.driver`.
- Skill tools remain owned by `service.skill`.
- MCP tools remain owned by `service.mcp`.
- Memory tools remain owned by `service.memory`.
- Scheduled-task and delegation tools remain owned by their autonomy services.
- Managed gateway tools remain owned by `service.gateway` or a gateway provider
  registered under `service.tool`.

### 4.4 Tool Plan

Tool planning converts application policy, agent role, service availability,
provider status, and context into one deterministic snapshot.

```text
ToolPlan {
  visible: [ToolPlanEntry],
  hidden: [HiddenToolPlanEntry],
  conflicts: [ToolConflictDiagnostic],
  policy_refs: [PolicyDecisionRef],
  audit_refs: [AuditRef],
  estimated_schema_tokens,
  captured_at
}
```

Visible entries contain:

- descriptor
- executor route
- effective policy
- invocation budget
- approval requirement
- resource lease hints

Hidden entries contain:

- descriptor summary
- stable reason code
- human-readable diagnostic
- non-secret remediation hint

Example hidden reason codes:

- `policy_denied`
- `missing_auth`
- `missing_config`
- `missing_binary`
- `provider_unavailable`
- `resource_capacity_exceeded`
- `unsupported_platform`
- `toolset_disabled`
- `schema_invalid`
- `name_conflict`
- `entitlement_missing`

The model only sees visible tools. Operators and audits can inspect hidden
diagnostics.

### 4.5 Tool Families

Macaca should support a generic family taxonomy. Families are abstract
capabilities, not hardcoded application behavior.

| Family | Example capabilities |
| --- | --- |
| `file` | read, write, patch, search, sync, diff, artifact open |
| `shell` | command execution, process registry, PTY, background jobs |
| `browser` | navigate, snapshot, click, type, scroll, console, screenshot, CDP, vision |
| `web` | search, extract, crawl, fetch, structured page parse |
| `memory` | active recall, search, get, persist, forget, provenance |
| `knowledge` | code index, document index, repo search, dependency graph |
| `task` | todo, plan, delegate, claim, heartbeat, block, complete |
| `scheduler` | create, list, update, pause, resume, trigger scheduled agent work |
| `skill` | list, view, invoke, manage governed skills |
| `mcp` | dynamic external tool surfaces through MCP |
| `media` | image/audio/video analysis and generation |
| `document` | PDF, Word, spreadsheet, presentation conversion and editing |
| `communication` | message send/list/reply through gateway providers |
| `enterprise_api` | provider-neutral SaaS/API operations through plugins or MCP |
| `code_execution` | sandboxed scripts, notebooks, typed batch tool programs |
| `computer_use` | desktop/app automation through a provider-neutral driver |
| `payment_entitlement` | paid capability checks, metering, and A2A settlement hooks |

Tool families define stable policy categories. Providers implement concrete
tools under those categories.

### 4.6 Toolsets

Toolsets are named bundles of tool families and specific tools.

Examples:

- `default_agent`
- `software_engineering`
- `research`
- `browser_automation`
- `data_processing`
- `document_work`
- `media_generation`
- `safe_no_shell`
- `worker_minimal`
- `autonomy_scheduler`
- `mcp_only`
- `application_declared`

Toolsets should be resolved by data, not code branches:

```text
toolset
  -> includes families
  -> includes tool names
  -> excludes families
  -> excludes tool names
  -> requires context signals
  -> applies policy profile
```

Application manifests may reference toolsets and capability families. The OS
must not branch on application names to choose tools.

### 4.7 Availability Expressions

Availability should follow a Specification pattern. Signals are composable:

- config path exists or is non-empty
- secret reference is configured
- auth provider exists
- environment value exists
- binary exists
- service health is ready
- platform supports capability
- resource capacity is available
- entitlement passes
- plugin is enabled
- application declared capability
- agent policy allows capability
- session context value matches

Availability evaluation must be cached with bounded TTLs and explicit
invalidation on config/provider changes.

### 4.8 Policy and Approval

Every tool invocation passes a policy chain before side effects.

Policy strategies:

- allow/deny by family, toolset, provider, and tool id
- resource scope restrictions
- workspace-only filesystem policy
- network egress policy
- destructive command detection
- write operation approval
- credential/secret access policy
- external communication policy
- paid/entitlement policy
- human approval policy
- model-autonomous approval policy for low-risk actions
- session-level remembered approvals

Policy decisions can:

- allow
- deny
- require approval
- rewrite safe parameters
- require a different environment
- downgrade to read-only mode
- require artifact persistence
- require narrower result budget

All decisions produce stable audit refs.

### 4.9 Invocation Path

The canonical invocation path:

```text
agent/tool_call
  -> framework Toolkit adapter
  -> SystemToolClient::invoke
  -> ServiceRuntime
  -> service.tool/tool.invoke
  -> descriptor route lookup
  -> policy and resource decorators
  -> owning service client or provider adapter
  -> provider runtime
  -> result normalizer
  -> artifact/result storage
  -> trace/audit/event bus
  -> bounded model-visible result
```

`service.tool` can invoke an owning service, but it must not bypass that
service's lifecycle or policy. For example:

- MCP invocation routes to `service.mcp/mcp.tool.invoke`.
- Skill invocation routes to `service.skill/skill.tool.invoke`.
- Driver invocation routes to `service.driver/driver.tool.invoke`.
- Memory invocation routes to `service.memory`.

This preserves service ownership while giving applications one generic Tools
system.

### 4.10 Runtime Environments

Industrial tools need environment abstraction. Macaca should model environments
as providers:

- local workspace
- local sandbox
- Docker container
- SSH host
- remote worker
- WASM host import
- managed cloud sandbox
- browser sandbox
- per-call ephemeral environment
- session-scoped persistent environment

Each environment provider exposes:

- descriptor
- health
- resource policy
- filesystem mounts
- network policy
- secret injection policy
- cleanup lifecycle
- process registry
- artifact roots
- audit scope

Tools request an environment by capability and policy. They do not construct
environment backends directly.

### 4.11 Managed Tool Gateway

Macaca should support a managed tool gateway as an optional provider, not as a
separate control plane.

Gateway-backed tools can provide:

- web search and extraction
- image generation
- browser automation
- document conversion
- media analysis
- remote sandbox execution
- enterprise connectors

Gateway rules:

- A gateway provider registers descriptors like every other provider.
- Direct keys and managed gateway can coexist.
- Routing is strategy-based and policy-driven.
- Gateway usage is metered and audited.
- Gateway absence returns structured unavailable diagnostics.
- Gateway-specific provider names must stay in config/descriptor data, not OS
  control-flow branches.

### 4.12 Result Handling

Tool results must be normalized before returning to the model.

Result classes:

- small text/json result
- multimodal result
- artifact reference
- streaming progress
- background task handle
- approval request
- structured failure

Rules:

- Small results may be returned inline.
- Large results are persisted and returned as artifact refs with summaries.
- Binary results are artifacts.
- Logs are paginated.
- Long-running tools return handles and progress events.
- Sensitive fields are redacted before logs, EventLog, SSE, and audit.
- Raw provider payloads are not copied to durable traces unless a specific
  governed debug policy allows it.

### 4.13 Observability and Audit

The system must emit sanitized events for:

- catalog plan started/completed
- provider discovered/unavailable
- tool hidden with reason
- policy decision
- approval requested/resolved
- resource lease acquired/released
- invocation started/progress/completed/failed/cancelled
- result persisted
- artifact opened
- provider health changed

Audit records include:

- trace id
- application id
- session id
- agent name
- task/goal ids when available
- service id
- provider id
- tool id and visible name
- policy decision ref
- resource scope
- input hash
- output hash
- result class
- artifact refs
- latency
- status and stable reason code

Audit records exclude raw secrets, raw prompts, private keys, credentials, raw
headers, raw env values, raw provider payloads, and unbounded outputs.

### 4.14 Context Integration

The Context service should include compact capability indexes:

- visible tool families
- visible tool names
- key hidden diagnostics summary counts
- usage discipline for risky families
- toolset summary
- capability dependencies for skills

It should not inject full tool docs, raw MCP resources, provider payloads, or
unbounded schemas by default.

The model should learn:

- what it can do
- when it must ask for approval
- when to prefer one tool family over another
- how to retrieve larger artifacts
- which tools are unavailable without exposing secrets

### 4.15 Application Manifest Integration

Applications declare desired capabilities through generic policy:

```yaml
capabilities:
  tools:
    toolsets:
      - software_engineering
      - research
    families:
      allow:
        - file
        - shell
        - browser
        - memory
      deny:
        - communication
    tools:
      allow: []
      deny: []
    approval_profile: standard
    result_budget_profile: default
```

Applications may define stricter agent-level overrides. They may not require
Macaca OS code to special-case their domain.

### 4.16 WASM and SDK Integration

WASM guests and SDK callers should use the same surface:

```text
macaca:service/call service.tool/tool.catalog.plan
macaca:service/call service.tool/tool.invoke
```

The host bridge preserves:

- application identity
- session identity
- trace context
- capability declarations
- payload bounds
- policy hooks

WASM must not receive a special tool path that bypasses the service runtime.

## 5. Design Patterns

| Pattern | Usage |
| --- | --- |
| Facade | `SystemToolClient`, focused provider clients, `SystemFacade` |
| Command | typed catalog, plan, invoke, status, result, artifact, audit commands |
| Adapter / Bridge | Driver, Skill, MCP, Memory, Gateway, plugin, environment providers |
| Strategy | routing, availability, policy, approval, result budget, conflict handling |
| Decorator | trace, redaction, metering, entitlement, resource, approval, timeout |
| State | provider lifecycle, environment lifecycle, invocation lifecycle |
| Observer | EventLog, SSE, telemetry, audit, usage analytics |
| Memento | tool plan snapshots, invocation records, artifacts, provider snapshots |
| Specification | availability, policy, entitlement, dependency, package admission rules |
| Abstract Factory | provider/environment/gateway bootstrapping in runtime-host |
| Null Object | unavailable providers and disabled tools return structured diagnostics |

## 6. Rejected Designs

### 6.1 Expand Web Toolkit Assembly Only

Rejected because Web would remain the semantic owner of tool composition,
provider routing, and policy filtering. This violates shell boundaries.

### 6.2 One Giant Tool Runtime That Owns Everything

Rejected because it would steal lifecycle ownership from Driver, Skill, MCP,
Memory, Task, and Gateway services. `service.tool` coordinates; it does not
replace owning services.

### 6.3 Provider-Specific Built-In Branches

Rejected because OS code must not branch on concrete provider names, app names,
or business domains. Provider selection belongs in descriptors, policy, and
strategy data.

### 6.4 Prompt-Only Tool Instructions

Rejected because industrial tasks require actual invocation, sandboxing,
policy, result storage, cleanup, and audit.

### 6.5 Raw Tool Dumps in Context

Rejected because large schemas and raw resources bloat prompts and can leak
unsafe provider content. Context gets compact capability indexes by default.

## 7. Migration Strategy

### Phase 1: Specification and Contracts

- Create OpenSpec change for the industrial Tools system.
- Add or extend DTOs for tool plans, hidden diagnostics, policy refs,
  invocation refs, result classes, and artifact refs.
- Define `SystemToolClient`.
- Define `service.tool` descriptor and command names.

### Phase 2: Plan-Only Service Path

- Implement `tool.catalog.plan` without changing invocation behavior.
- Feed existing Driver, Skill, MCP, runtime tools, memory tools, scheduled-task
  tools, and workspace tools into one plan.
- Report visible and hidden diagnostics.
- Keep current toolkit invocation adapters.

### Phase 3: Service-Owned Invocation Facade

- Implement `tool.invoke` as a routing facade to owning services.
- Add policy, approval, resource, timeout, and result-budget decorators.
- Preserve direct provider service calls for compatibility but make toolkit
  invocation use `SystemToolClient`.

### Phase 4: Runtime Environment Providers

- Normalize local workspace, sandbox, Docker/remote, browser, and managed
  gateway environments behind environment descriptors.
- Add health, cleanup, and artifact roots.

### Phase 5: Tool Families and Toolsets

- Add data-driven tool family and toolset resolution.
- Move manifest `allowed_tools` toward capability/family/toolset policy while
  preserving exact tool allowlists for compatibility.

### Phase 6: Rich Provider Expansion

- Add first-class industrial providers for browser, web, documents, media,
  code execution, enterprise APIs, communication, and computer-use through
  plugin/MCP/gateway adapters.
- Do not embed application-specific behavior.

### Phase 7: Context, UI, and Audit Completion

- Add compact capability context.
- Add Web/CLI diagnostics for visible/hidden tools, provider status, approval,
  artifacts, and audit refs.
- Add replayable audit queries.

## 8. Acceptance Criteria

The system is complete only when:

1. Applications can declare generic tool capability needs without OS code
   changes.
2. Agents receive a deterministic tool plan with visible tools and hidden
   diagnostics.
3. Tool invocation routes through `service.tool` and then to the owning service
   or provider.
4. Driver, Skill, MCP, Memory, Task, Gateway, and runtime tools remain
   service-owned.
5. Policy, approval, entitlement, resource, timeout, and result-budget gates
   execute before side effects.
6. Large results become artifacts with stable refs.
7. Tool events are observable live and replayable from audit.
8. Missing providers, denied tools, unsupported platforms, bad schemas, and
   runtime failures return structured states.
9. Shells do not own tool semantics.
10. No OS-layer code hardcodes application-specific or provider-specific
    business logic.

## 9. Verification Plan

Required verification after implementation:

- OpenSpec strict validation.
- Unit tests for tool plan visibility and hidden diagnostics.
- Unit tests for availability expression evaluation.
- Unit tests for policy and approval decisions.
- Unit tests for result budget and artifact persistence.
- Service tests for `service.tool` catalog, invoke, status, and audit.
- Integration tests proving Driver, Skill, MCP, Memory, and Task tools route
  through the common plan.
- Boundary tests proving Web/CLI remain shell adapters.
- Dependency gate checks for serviceization boundaries.
- Live smoke with a real application-neutral task using multiple tool families.
- Audit replay test proving stable refs and aggregate counts without raw model
  output or raw provider payloads.

## 10. Open Questions for OpenSpec

1. Should `service.tool` live in a new `macaca-tools-service` crate or extend
   the existing `macaca-tools` service crate with a service adapter?
2. Should `CapabilityToolDescriptor` be extended directly, or should a new
   `ToolCapabilityDescriptor` wrap it for compatibility?
3. Should result artifacts be owned by Store service first, or should
   `service.tool` initially write through an existing artifact abstraction?
4. How aggressively should exact-name `allowed_tools` be migrated to family and
   toolset policy?
5. Which managed gateway provider should be used for the first live proof, if
   any, without making it a built-in OS branch?

## 11. Immediate Next Step

Create an OpenSpec change named:

```text
upgrade-industrial-tool-capability-plane
```

The OpenSpec should include:

- `proposal.md`
- `design.md`
- `tasks.md`
- specs for tool capability planning, service-owned invocation, tool families,
  toolsets, policy diagnostics, result artifacts, audit, and shell boundaries.

Implementation should not start until that OpenSpec is reviewed and validated.
