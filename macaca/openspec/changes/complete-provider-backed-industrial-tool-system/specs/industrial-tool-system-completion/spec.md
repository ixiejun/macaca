## ADDED Requirements

### Requirement: Provider-Backed Families Shall Replace Catalog-Only Descriptors

Macaca SHALL expose industrial tool families only when each visible callable descriptor is backed by a real provider contributor, owning service, runtime environment, managed gateway, plugin, MCP server, driver, skill, or structured unavailable route.

#### Scenario: Visible family descriptor has real ownership

- **GIVEN** an industrial family descriptor is visible and callable
- **WHEN** the Tool Capability Plane returns the descriptor
- **THEN** the descriptor SHALL identify a real owning service, MCP server, plugin, runtime environment, managed gateway, driver, or skill route
- **AND** the descriptor SHALL NOT use a synthetic `service.tool.family.{family}` owner.

#### Scenario: Optional provider is absent

- **GIVEN** an optional industrial family has no configured provider
- **WHEN** the Tool Capability Plane builds the family plan
- **THEN** the family SHALL be reported as hidden diagnostics or an unavailable provider summary
- **AND** Macaca SHALL NOT expose a fake visible callable tool for that family.

### Requirement: Tool Invocation Shall Dispatch By Typed Executor Route

Macaca SHALL dispatch `tool.invoke` through typed executor route metadata rather than collapsing industrial families into MCP origin semantics.

#### Scenario: Runtime environment route is invoked

- **GIVEN** a file, shell, or code-execution tool descriptor uses a runtime-environment route
- **WHEN** `tool.invoke` is called for that descriptor
- **THEN** `service.tool` SHALL dispatch through the registered runtime environment provider
- **AND** the invocation SHALL include sandbox, cleanup, metering, trace, and audit context.

#### Scenario: Managed gateway route is invoked

- **GIVEN** a browser, web, document, media, communication, or enterprise API descriptor uses a managed-gateway route
- **WHEN** `tool.invoke` is called for that descriptor
- **THEN** `service.tool` SHALL dispatch through the registered managed gateway provider
- **AND** provider health, timeout, metering, redaction, and audit behavior SHALL be applied.

#### Scenario: Service-owned route is invoked

- **GIVEN** a memory, task, scheduler, entitlement, skill, or MCP-backed descriptor uses its owning route
- **WHEN** `tool.invoke` is called for that descriptor
- **THEN** `service.tool` SHALL delegate through the owning service adapter
- **AND** `service.tool` SHALL NOT read or mutate that service's internal storage directly.

### Requirement: Invocation Admission Shall Enforce Industrial Gates Before Side Effects

Macaca SHALL run a typed invocation admission chain before dispatching any side-effecting tool invocation.

#### Scenario: Policy denies side effect

- **GIVEN** a shell or file invocation violates family policy or application scope
- **WHEN** `tool.invoke` is evaluated
- **THEN** the invocation SHALL return a typed denied result before owner dispatch
- **AND** an audit event SHALL record the denial reason code.

#### Scenario: Entitlement is missing

- **GIVEN** a paid or entitlement-gated family requires an entitlement
- **AND** the caller lacks that entitlement
- **WHEN** `tool.invoke` is evaluated
- **THEN** the invocation SHALL return a missing-entitlement result before owner dispatch
- **AND** the provider SHALL NOT execute side effects.

#### Scenario: Approval is required

- **GIVEN** a route requires human or policy approval for its side-effect class
- **WHEN** `tool.invoke` is evaluated without the required approval state
- **THEN** the invocation SHALL return an approval-required result before owner dispatch
- **AND** the approval request SHALL include trace, route, policy, and resource context.

### Requirement: Runtime Environments And Managed Gateways Shall Execute Tool Routes

Macaca SHALL connect runtime environment and managed gateway providers to the production tool invocation path.

#### Scenario: Runtime environment executes sandboxed command

- **GIVEN** a shell or code-execution route selects a registered runtime environment
- **WHEN** the invocation is admitted
- **THEN** the runtime environment provider SHALL execute the work with sandbox, cleanup, metering, and audit hooks
- **AND** the result SHALL include structured output references rather than unbounded raw output.

#### Scenario: Gateway-backed provider executes external work

- **GIVEN** a gateway-backed route selects a registered managed gateway
- **WHEN** the invocation is admitted
- **THEN** the managed gateway provider SHALL execute the work through its provider adapter
- **AND** the result SHALL include provider status, metering, audit, and sanitized error data.

### Requirement: Operator Diagnostics Shall Reflect Real Provider State

Macaca SHALL provide toolset resolution, provider health, and provider status diagnostics derived from registered contributors and runtime availability.

#### Scenario: Toolset is resolved

- **WHEN** an operator calls `tool.toolset.resolve`
- **THEN** the response SHALL include selected families, selected providers, route kinds, filtered providers, unavailable families, policy reason codes, and trace/audit references
- **AND** the response SHALL NOT be an empty synthetic plan when providers are registered.

#### Scenario: Provider health is queried

- **WHEN** an operator calls `tool.provider.health`
- **THEN** the response SHALL include registered provider count, healthy count, degraded count, unavailable count, last check timestamp, and reason-code summaries
- **AND** the response SHALL NOT always report healthy with `provider_count` equal to zero.

### Requirement: Results And Artifacts Shall Be Bounded And Sanitized

Macaca SHALL normalize provider-backed tool results before returning them to models, shells, SDK callers, audit stores, or event streams.

#### Scenario: Oversized output becomes artifact reference

- **GIVEN** a provider-backed invocation returns output larger than the configured inline result budget
- **WHEN** result normalization runs
- **THEN** the output SHALL be persisted as an artifact reference with a bounded summary
- **AND** the model-visible result SHALL NOT include unbounded raw output.

#### Scenario: Sensitive provider data is redacted

- **GIVEN** a provider-backed invocation observes secrets, credentials, headers, environment values, prompts, or raw provider payloads
- **WHEN** logs, EventLog, SSE, audit, or shell diagnostics are emitted
- **THEN** those surfaces SHALL exclude the sensitive raw values
- **AND** they SHALL include only stable refs, hashes, bounded summaries, and reason codes.

### Requirement: Availability Expressions Shall Be Evaluated As Specifications

Macaca SHALL evaluate provider availability through composable specification signals with bounded cache lifetimes and explicit invalidation.

#### Scenario: Provider config changes

- **GIVEN** a provider availability decision was cached
- **WHEN** provider configuration, credentials, binary availability, service health, platform support, resource capacity, entitlement, plugin state, application declaration, or session context changes
- **THEN** the availability cache SHALL be invalidated or refreshed within its bounded TTL
- **AND** subsequent tool plans SHALL reflect the updated visible or hidden state.

### Requirement: Context And Manifest Integration Shall Use Generic Tool Policy

Macaca SHALL integrate industrial tools with Context and application manifests through compact indexes and generic policy declarations.

#### Scenario: Context receives compact capability index

- **WHEN** Context composition includes tool capability information
- **THEN** it SHALL include compact visible family counts, visible tool names, hidden reason summaries, toolset summaries, risky-family usage discipline, and capability dependencies
- **AND** it SHALL NOT include raw provider payloads, unbounded schemas, raw MCP resources, or full tool documentation by default.

#### Scenario: Manifest declares tool policy

- **GIVEN** an application manifest declares toolsets, family allow/deny rules, tool allow/deny rules, approval profile, and result budget profile
- **WHEN** tool planning and invocation admission run
- **THEN** Macaca SHALL enforce those generic declarations
- **AND** OS-layer code SHALL NOT branch on the application name or business domain.

### Requirement: WASM, SDK, And Shell Callers Shall Use The Same Service Surface

Macaca SHALL expose industrial tools through the same `service.tool` planning, invocation, result, artifact, diagnostics, and audit contracts for WASM guests, SDK callers, Web, CLI, and frontend shells.

#### Scenario: WASM guest invokes a tool

- **GIVEN** a WASM guest needs tool access
- **WHEN** it requests a catalog plan or invocation
- **THEN** the host bridge SHALL call `macaca:service/call service.tool/tool.catalog.plan` or `macaca:service/call service.tool/tool.invoke`
- **AND** the request SHALL preserve application identity, session identity, trace context, capability declarations, payload bounds, and policy hooks.

#### Scenario: SDK and shell diagnostics are rendered

- **WHEN** SDK, Web, CLI, or frontend surfaces display tool diagnostics
- **THEN** they SHALL consume service-owned DTOs for plan, provider status, provider health, policy explanation, audit query, result retrieval, and artifact access
- **AND** they SHALL NOT define provider routing, policy, or execution semantics.

### Requirement: Production Bootstrap Shall Register Industrial Contributors

Macaca runtime startup SHALL register the industrial tool planner with real provider contributors through a runtime-host composition root.

#### Scenario: Web runtime starts

- **WHEN** the Web shell starts
- **THEN** it SHALL call the runtime-host industrial planner composition helper
- **AND** it SHALL NOT register an empty planner or define provider semantics inside the shell.

#### Scenario: Shell boundary is audited

- **WHEN** serviceization boundary tests inspect Web, CLI, and SDK shells
- **THEN** shells SHALL be verified as thin clients for planning, invocation, diagnostics, and audit replay
- **AND** provider ownership SHALL remain inside services, MCP, plugins, runtime environments, managed gateways, drivers, or skills.

### Requirement: Industrial Proof Shall Use Real Providers And No Fake Owners

Macaca SHALL prove the industrial tool system through application-neutral integration tests that use registered contributors and real invocation paths or explicit unavailable diagnostics.

#### Scenario: Multi-family industrial workflow is tested

- **GIVEN** a generic agent task requires multiple tool families
- **WHEN** the integration proof runs
- **THEN** planning, invocation, artifact recording, provider health, and audit replay SHALL execute through registered contributors
- **AND** the test SHALL NOT manually inject availability signals or register fake document owners.

#### Scenario: Proof report is sanitized

- **WHEN** the integration proof emits evidence
- **THEN** the report SHALL include stable session refs, provider counts, route counts, audit refs, artifact refs, and reason-code summaries
- **AND** the report SHALL NOT include secrets, credentials, raw provider payloads, raw model output, or unbounded command output.
