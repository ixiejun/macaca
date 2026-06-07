# Change: Add system service contract

## Why

Route C Phase 02 must turn replaceable OS capabilities into explicit system services. Macaca currently has many mature service-like crates, but upper layers can still call concrete implementations directly, which makes capability replacement, policy enforcement, trace/audit coverage, plugin onboarding, and optional module behavior harder to guarantee.

## What Changes

- Add provider-neutral service descriptors in `macaca-proto` for service type, capabilities, lifecycle state, health, permissions, supported scopes, trace schema, cleanup policy, and call payloads.
- Add `macaca-kernel` `SystemService` contracts for lifecycle, health, descriptor export, trace-required calls, structured errors, and logged/audited call execution.
- Add `ServiceCommand`, `ServiceCallContext`, `ServiceCallResult`, and `ServiceError` as command-style service invocation primitives.
- Add a first adapter skeleton slice for LLM, Task, and Trace services without migrating their existing runtime call paths.
- Add a second adapter skeleton slice for Driver, Skill, Gateway, and Memory services without embedding provider, driver, gateway, application, or business names in kernel contracts.
- Add service call trace middleware so missing `TraceContext` is rejected and successful mock service calls emit trace/audit events.
- Require detailed English comments for all new code explaining purpose, lifecycle, trace/audit behavior, and compatibility limitations.

## Impact

- Affected specs: `system-service-contract`
- Affected crates: `macaca-proto`, `macaca-kernel`, `macaca-llm`, `macaca-memory`, `macaca-task`, `macaca-driver`, `macaca-skill`, `macaca-gateway`
- Affected tests: `macaca-proto` service serde tests, `macaca-kernel/tests/system_service_contract.rs`, targeted adapter skeleton tests
- Regression matrix references: `RC-GOAL-001`, `RC-TRACE-001`, `RC-SKILL-001`

## Governance Alignment

- Follows `macaca/docs/agent-os-microkernel-boundaries.md`: replaceable LLM, Memory, Task, Driver, Skill, MCP, Gateway, Store, Payment, Web3, EVM, UI, and Persistence capabilities are system services, plugins, or optional modules rather than kernel business logic.
- Follows `macaca/docs/route-c-regression-matrix.md`: Phase 02 must preserve goal execution, trace real-time behavior, and skill/MCP trace semantics.
- Follows `macaca/docs/route-c-phase-template.md`: OpenSpec first, additive-first implementation, GitNexus impact before symbol edits, targeted tests, integration smoke, detect_changes before commit.
- Follows `macaca/docs/route-c-architecture-governance.md`: service calls require policy/permission modeling, trace/audit observability, no provider/application hardcode, and structured unavailable behavior for optional modules.

## Non-Goals

- Do not implement a full service bus transport in this phase.
- Do not migrate all existing LLM, Task, Trace, Driver, Skill, Gateway, or Memory runtime calls to the new contract.
- Do not implement Store, Payment, Web3, EVM, GenUI, package entitlement, or plugin installation behavior.
- Do not move `TodoBoard`, planner/review workflow, LLM provider routing, driver execution, skill runtime, MCP runtime, gateway adapters, memory implementations, or persistence implementations into `macaca-kernel`.
- Do not hardcode application names, provider names, driver names, gateway names, model names, workflow names, chain names, or business-specific routing in the service contract.
