# S5 LLM / Memory / Context 服务化实施计划

## Scope

Implement S5 from `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`: move LLM provider access, Memory backend access, and Context composition/active-recall access behind service boundaries compatible with `ServiceRuntime` and `SystemFacade`.

S5 covers:

- LLM chat/model selection/service snapshot.
- Memory remember/recall/prefetch/forget/status/snapshot.
- Context assemble/active recall/knowledge digest/provider inventory/snapshot.
- SDK client boundaries for upper consumers.
- Gradual migration of Web/CLI/framework/agent construction away from direct provider/backend ownership.

S5 does not cover:

- Driver / Skill / MCP serviceization. That belongs to S6.
- Application lifecycle serviceization. That belongs to S7.
- Gateway serviceization. That belongs to S8.
- Store/payment/Web3/EVM phases.
- Removing legacy wrappers before all callers migrate.

## Required Governance Inputs

- `macaca/docs/agent-os-microkernel-boundaries.md`
- `macaca/docs/route-c-serviceization-allowlist.md`
- `macaca/docs/route-c-architecture-governance.md`
- `macaca/docs/route-c-regression-matrix.md`
- `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`
- `docs/superpowers/plans/2026-05-08-s1-service-runtime-v1-plan.md`
- `docs/superpowers/plans/2026-05-08-s3-sdk-system-facade-convergence-plan.md`
- `docs/superpowers/plans/2026-05-08-s4-task-planner-review-serviceization-plan.md`

## Architecture Decision

Use three provider-neutral services with independent contracts and host-owned runtime adapters:

- `LlmService`: chat, model route, model inventory, token/cost metadata, health snapshot.
- `MemoryService`: remember, recall/search, prefetch, forget/delete, status, governance snapshot.
- `ContextService`: assemble model context, active recall orchestration, knowledge digest composition, provider/engine inventory, context report snapshot.

Design patterns:

- Facade: each service exposes one focused boundary for upper callers.
- Adapter / Bridge: existing `LlmProvider`, `LlmRouter`, `MemoryFacade`, `MemoryFabricFacade`, `ContextFacade`, and `ContextEngine` are adapted into service providers.
- Strategy: provider routing, memory routing, context engine selection, active recall policy, digest selection, and fallback behavior are replaceable.
- Command: all calls use typed commands before they become `ServiceCommand` payloads.
- Decorator: trace-required, policy, token/cost, memory governance, context budget, and privacy checks are composable.
- Observer: all key lifecycle and call nodes emit structured logs/events.
- Memento: snapshots expose deterministic health/inventory/decision summaries without dumping sensitive content.
- Specification: command constructors and runtime admission validate scope, trace, permissions, budget, privacy tier, and optional availability.

Rejected alternatives:

- Move all LLM/Memory/Context logic directly into `macaca-runtime-host`: rejected because domain semantics belong to domain crates; runtime-host only owns lifecycle and dispatch.
- Keep Web as the composite provider hub: rejected because presentation shells must be thin adapters.
- Only add SDK clients without runtime providers: rejected as incomplete S5 serviceization.
- Serviceize only LLM in this phase: rejected because Memory active recall and Context composer are coupled with model calls and must receive compatible service seams.

## Proposed OpenSpec Change

Expected change id:

- `add-llm-memory-context-services-v1`

Expected artifacts:

- `openspec/changes/add-llm-memory-context-services-v1/proposal.md`
- `openspec/changes/add-llm-memory-context-services-v1/design.md`
- `openspec/changes/add-llm-memory-context-services-v1/tasks.md`
- `openspec/changes/add-llm-memory-context-services-v1/specs/llm-service/spec.md`
- `openspec/changes/add-llm-memory-context-services-v1/specs/memory-service/spec.md`
- `openspec/changes/add-llm-memory-context-services-v1/specs/context-service/spec.md`

The proposal should state:

- S5 is additive-first and preserves existing `/api/chat/v2`, framework runner, active recall, memory tools, and context report behavior.
- Web/CLI/framework/agent builder must stop constructing or owning concrete provider/backend state as the long-term path.
- Legacy provider/backend wrappers remain searchable and deprecated until all call sites migrate.
- Service calls require trace and policy admission through `ServiceRuntime` or equivalent SDK client contract.
- Snapshot/event payloads must avoid dumping sensitive prompt or memory content by default.

## Implementation Slices

### Slice S5.1: Impact and Boundary Audit

Files to inspect before editing:

- `macaca/crates/macaca-llm/src/provider.rs`
- `macaca/crates/macaca-llm/src/router.rs`
- `macaca/crates/macaca-llm/src/service_adapter.rs`
- `macaca/crates/macaca-memory/src/core/facade.rs`
- `macaca/crates/macaca-memory/src/core/adapter.rs`
- `macaca/crates/macaca-memory/src/runtime/*`
- `macaca/crates/macaca-memory/src/service_adapter.rs`
- `macaca/crates/macaca-context/src/composer/facade.rs`
- `macaca/crates/macaca-context/src/engine.rs`
- `macaca/crates/macaca-context/src/memory_active_recall_provider.rs`
- `macaca/crates/macaca-framework/src/adapter.rs`
- `macaca/crates/macaca-framework/src/react_agent.rs`
- `macaca/crates/macaca-web/src/lib.rs`
- `macaca/crates/macaca-web/src/state.rs`
- `macaca/crates/macaca-web/src/context_reporting_model.rs`
- `macaca/crates/macaca-web/src/framework_runner.rs`
- `macaca/crates/macaca-cli/src/commands.rs`
- `macaca/crates/macaca-kernel/src/provider_compat.rs`
- `macaca/crates/macaca-kernel/src/kernel_builder.rs`
- `macaca/crates/macaca-runtime-host/src/service_runtime.rs`
- `macaca/crates/macaca-sdk/src/system_facade.rs`
- `macaca/crates/macaca-sdk/src/service_client.rs`

Required actions:

1. Run GitNexus impact before modifying existing symbols.
2. Identify all direct `Arc<dyn LlmProvider>`, `LlmRouter`, `MemoryFacade`, `WebMemoryRuntime`, `ContextFacade`, and `ContextEngineRegistry` ownership paths.
3. Classify each path as kernel, service, framework, SDK, Web, CLI, or test.
4. Confirm which allowlist rows can only be deleted after S5 completes.
5. Warn before editing HIGH or CRITICAL impact symbols.

### Slice S5.2: LLM Service Contract

Files:

- Add or update: `macaca/crates/macaca-llm/src/service_contract.rs`
- Update: `macaca/crates/macaca-llm/src/service_adapter.rs`
- Update: `macaca/crates/macaca-llm/src/lib.rs`

Behavior:

- Define typed commands:
  - `LlmChatCommand`
  - `LlmModelSelectionCommand`
  - `LlmServiceSnapshotCommand`
- Define typed results:
  - `LlmChatResult`
  - `LlmModelSelectionResult`
  - `LlmServiceSnapshot`
- Define events:
  - `llm.chat.requested`
  - `llm.chat.completed`
  - `llm.chat.failed`
  - `llm.model.selected`
  - `llm.snapshot.emitted`
- Keep commands provider-neutral: app/session/agent scope, trace, messages, model hint, options, budget/policy hints.
- Do not put concrete API keys, provider URLs, or model-provider if/else into service command types.

Rules:

- Detailed English comments explaining each command/result/event and why it is provider-neutral.
- Structured logs at model selection, dispatch, completion, failure, and snapshot.
- File size under 500 lines.

### Slice S5.3: Memory Service Contract

Files:

- Add or update: `macaca/crates/macaca-memory/src/service_contract.rs`
- Update: `macaca/crates/macaca-memory/src/service_adapter.rs`
- Update: `macaca/crates/macaca-memory/src/lib.rs`

Behavior:

- Define typed commands:
  - `MemoryRememberCommand`
  - `MemoryRecallCommand`
  - `MemoryPrefetchCommand`
  - `MemoryForgetCommand`
  - `MemoryStatusCommand`
  - `MemoryServiceSnapshotCommand`
- Reuse `MemoryScope`, `MemoryVisibility`, `MemoryFacade`, and provider capability metadata.
- Preserve application -> database and agent -> collection topology as an abstraction, not a provider hardcode.
- Expose AgentPrivate and SessionShared semantics explicitly.
- Snapshot should include provider id, capability set, topology labels, health, governance counts, and last audit ids; it must not dump memory content by default.

Rules:

- No default app-wide/global recall.
- All recall commands require explicit app/session/agent scope.
- Detailed English comments and structured logs.
- File size under 500 lines.

### Slice S5.4: Context Service Contract

Files:

- Add: `macaca/crates/macaca-context/src/service_adapter.rs`
- Add: `macaca/crates/macaca-context/src/service_contract.rs`
- Update: `macaca/crates/macaca-context/src/lib.rs`

Behavior:

- Define typed commands:
  - `ContextAssembleCommand`
  - `ContextActiveRecallCommand`
  - `ContextProviderInventoryCommand`
  - `ContextEngineInventoryCommand`
  - `ContextServiceSnapshotCommand`
- Define results:
  - assembled messages / compiled context metadata
  - context report
  - active recall diagnostics
  - provider/engine inventory
- Reuse `ContextFacade`, `ContextEngineSelection`, `ContextFacadeAssemblyPolicy`, `ContextProviderRegistry`, and governance pipeline.
- Keep Memory active recall behind a memory service client bridge; Context Service should not bind to a concrete memory backend.

Rules:

- Context Service owns composition, budget, provider chain, active recall orchestration, and report assembly.
- Memory Service owns recall storage and memory governance.
- LLM Service owns model calls. Context Service must not call LLM except for future summarization strategies explicitly modeled as service calls.

### Slice S5.5: Runtime-Host Provider Wrappers

Files:

- Add: `macaca/crates/macaca-runtime-host/src/llm_service_provider.rs`
- Add: `macaca/crates/macaca-runtime-host/src/memory_service_provider.rs`
- Add: `macaca/crates/macaca-runtime-host/src/context_service_provider.rs`
- Update: `macaca/crates/macaca-runtime-host/src/lib.rs`
- Update: `macaca/crates/macaca-runtime-host/Cargo.toml` only if dependency graph remains legal.

Behavior:

- Wrap existing domain facades into `macaca_kernel::SystemService`.
- Translate `ServiceCommand` payloads into typed domain commands.
- Dispatch through injected domain strategies/facades.
- Emit service runtime events via existing ServiceRuntime path.
- Return structured unavailable when a service is not configured.

Rules:

- Runtime-host owns lifecycle wiring, not domain decisions.
- No Web/CLI dependency.
- No application-specific or provider-specific hardcode.
- All calls must require trace and pass policy decorators before dispatch.

### Slice S5.6: SDK Focused Clients

Files:

- Add: `macaca/crates/macaca-sdk/src/llm_client.rs`
- Add: `macaca/crates/macaca-sdk/src/memory_client.rs`
- Add: `macaca/crates/macaca-sdk/src/context_client.rs`
- Update: `macaca/crates/macaca-sdk/src/system_facade.rs`
- Update: `macaca/crates/macaca-sdk/src/lib.rs`

Behavior:

- Define traits:
  - `SystemLlmClient`
  - `SystemMemoryClient`
  - `SystemContextClient`
- Provide unavailable/null-object clients for shells not wired to ServiceRuntime.
- Provide service-call-backed clients over `SystemServiceClient` / `ServiceCallCommand`.
- Add thin `SystemFacade` methods for chat, memory recall/prefetch, context assemble, and snapshots where useful.

Rules:

- SDK remains a client/facade layer, not a provider factory.
- Unsupported operations return structured unavailable.
- Commands validate non-empty scope, bounded limits, and trace requirements.

### Slice S5.7: Framework and Agent Construction Migration

Files:

- Update: `macaca/crates/macaca-framework/src/adapter.rs`
- Update: `macaca/crates/macaca-framework/src/react_agent.rs`
- Update: `macaca/crates/macaca-framework/src/model_impls.rs`
- Update: `macaca/crates/macaca-agent` only if direct provider construction exists.

Behavior:

- Add `ServiceChatModelAdapter` that implements `ChatModel` over `SystemLlmClient`.
- Deprecate `LlmProviderAdapter` and `RoutedLlmAdapter` for new code, but keep them searchable.
- Make `ReActAgent` context assembly injectable through `SystemContextClient` or a `ContextAssembler` Strategy.
- Preserve current behavior when no service client is installed.

Rules:

- Framework must not become a provider hub.
- Context assembly must be replaceable and testable without real LLM/network.
- All new code must log model/context dispatch nodes with trace ids.

### Slice S5.8: Web Migration

Files:

- Update: `macaca/crates/macaca-web/src/lib.rs`
- Update: `macaca/crates/macaca-web/src/state.rs`
- Update: `macaca/crates/macaca-web/src/context_reporting_model.rs`
- Update: `macaca/crates/macaca-web/src/framework_runner.rs`
- Update: `macaca/crates/macaca-web/src/routes.rs` only for read/status surfaces if needed.

Behavior:

- Build LLM/Memory/Context services during startup through runtime-host factories.
- Store service clients or runtime-backed facades in `AppState` instead of exposing concrete provider/backend as the primary path.
- Keep direct fields as deprecated compatibility fields until all call sites migrate.
- `ContextReportingModel` should request context assembly through Context Service and model calls through LLM Service.
- Memory active recall should query Memory Service, not `WebMemoryRuntime` directly.

Rules:

- Web remains HTTP/SSE adapter and trace viewer.
- Web may host compatibility adapters temporarily, but must not define new LLM/Memory/Context semantics.
- No route should construct provider/backend-specific logic after migration.

### Slice S5.9: CLI Migration

Files:

- Update: `macaca/crates/macaca-cli/src/commands.rs`

Behavior:

- Replace direct `StubLlmProvider` construction in production command paths with SDK unavailable or runtime-backed LLM client.
- Keep test-only stubs local to tests if needed.
- CLI status/inspect commands should surface LLM/Memory/Context service availability through SDK/SystemFacade.

Rules:

- CLI remains command shell.
- Missing services return structured unavailable, not panic.

### Slice S5.10: Kernel Compatibility Shrink

Files:

- Update: `macaca/crates/macaca-kernel/src/provider_compat.rs`
- Update: `macaca/crates/macaca-kernel/src/kernel_builder.rs`
- Update: `macaca/crates/macaca-kernel/src/kernel.rs`
- Update: `macaca/crates/macaca-kernel/Cargo.toml` only after callers migrate.

Behavior:

- Keep `KernelProviderCompat` deprecated and searchable.
- Add or use service-client compatibility bundle for new construction paths.
- Stop new kernel code from requiring `LegacyLlmProvider`.
- Remove `macaca-kernel -> macaca-llm` and `macaca-kernel -> macaca-memory` dependencies only after all references are gone.

Rules:

- Do not remove compatibility APIs until all upper callers migrate.
- Do not update allowlist before dependency gate proves the direct edges are gone.

### Slice S5.11: Allowlist and Governance Updates

Files:

- Update: `macaca/docs/route-c-architecture-governance.md`
- Update: `macaca/docs/route-c-serviceization-allowlist.md`
- Update: `macaca/crates/macaca-integration-tests/tests/route_c_dependency_boundaries/allowlist.rs`
- Optional update: `macaca/docs/route-c-regression-matrix.md`

Behavior:

- Document LLM Service, Memory Service, Context Service ownership rules.
- Mark Web/CLI/framework as adapters only.
- Remove S5 allowlist rows only when direct dependencies are actually gone:
  - `macaca-kernel -> macaca-llm`
  - `macaca-kernel -> macaca-memory`
  - `macaca-cli -> macaca-llm`
  - `macaca-web -> macaca-llm`
  - `macaca-web -> macaca-memory`
- Do not remove `macaca-web -> macaca-context` unless Web fully delegates context assembly to SDK/service and no longer needs DTO types.

## Dependency Boundary Expectations

Expected safe new dependencies:

- `macaca-runtime-host -> macaca-llm`
- `macaca-runtime-host -> macaca-memory`
- `macaca-runtime-host -> macaca-context`
- `macaca-sdk -> macaca-llm` only for command/result DTOs if kept provider-neutral.
- `macaca-sdk -> macaca-memory` only for command/result DTOs if kept provider-neutral.
- `macaca-sdk -> macaca-context` only for command/result DTOs if kept provider-neutral.

Expected forbidden or risky dependencies:

- `macaca-llm -> macaca-kernel`
- `macaca-memory -> macaca-kernel`
- `macaca-context -> macaca-kernel`
- `macaca-llm -> macaca-runtime-host`
- `macaca-memory -> macaca-runtime-host`
- `macaca-context -> macaca-runtime-host`
- any domain service crate -> `macaca-web`
- any provider-specific crate dependency added to `macaca-kernel`

If any new dependency edge trips S0 gate, stop and either redesign the placement or update OpenSpec plus allowlist explicitly.

## Verification

Run after implementation:

```bash
openspec validate add-llm-memory-context-services-v1 --strict
cargo fmt --all --check
cargo test -p macaca-llm
cargo test -p macaca-memory
cargo test -p macaca-context
cargo test -p macaca-sdk llm_client
cargo test -p macaca-sdk memory_client
cargo test -p macaca-sdk context_client
cargo test -p macaca-runtime-host service_runtime
cargo test -p macaca-framework react_agent
cargo test -p macaca-web framework_runner
cargo test -p macaca-web context_reporting_model
cargo test -p macaca-cli
cargo test -p macaca-integration-tests route_c_dependency_boundaries
cargo test -p macaca-integration-tests --test route_c_baseline
cargo check --workspace
npx gitnexus detect-changes -r agent
```

## Rollout Order

1. Write OpenSpec proposal/design/tasks/spec for all three services.
2. Implement domain contracts and descriptors first.
3. Implement runtime-host service providers using existing domain facades.
4. Add SDK focused clients and unavailable null-object clients.
5. Migrate framework model/context seams.
6. Migrate Web startup/state and `ContextReportingModel` path.
7. Migrate CLI direct LLM dependency.
8. Shrink kernel compatibility usage.
9. Run dependency gate and remove allowlist rows only when direct edges disappear.
10. Archive OpenSpec only after specs, code, tests, and dependency gate agree.

## Implementation Guardrails

- All code comments must be detailed English comments explaining function and operating principle.
- Every service call path must log key execution nodes: command accepted, policy checked, dispatched, completed, failed, snapshot emitted.
- Keep Rust files under 500 lines by splitting contract, events, provider, client, and tests.
- Do not add new external dependencies unless a service boundary cannot be implemented without them.
- Do not hardcode app name, workflow, provider, driver, gateway, model, chain, or business-specific names.
- Deprecated wrappers must remain searchable and should delegate to the new canonical service path where feasible.
