## 1. Preparation

- [x] 1.1 Read `docs/superpowers/plans/2026-05-08-s5-llm-memory-context-serviceization-plan.md`.
- [x] 1.2 Read `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`.
- [x] 1.3 Read `macaca/docs/agent-os-microkernel-boundaries.md`, `macaca/docs/route-c-serviceization-allowlist.md`, and `macaca/docs/route-c-architecture-governance.md`.
- [x] 1.4 Inspect current LLM, Memory, Context, runtime-host, SDK, framework, Web, CLI, and kernel compat call paths.
- [x] 1.5 Run GitNexus impact before editing existing functions, structs, traits, or methods, and warn before HIGH/CRITICAL blast radius edits.

## 2. OpenSpec

- [x] 2.1 Create `add-llm-memory-context-services-v1` proposal, design, tasks, and delta specs.
- [x] 2.2 Validate with `openspec validate add-llm-memory-context-services-v1 --strict`.
- [x] 2.3 Confirm scope stays on LLM/Memory/Context serviceization and does not absorb Driver/Skill/MCP/Gateway/Application lifecycle phases.

## 3. LLM Service Contract

- [x] 3.1 Add provider-neutral LLM typed commands for chat, model selection, and service snapshot.
- [x] 3.2 Add provider-neutral LLM result, inventory, token/cost metadata, event, and snapshot DTOs.
- [x] 3.3 Update LLM service adapter and exports without introducing kernel/runtime-host/Web/CLI dependencies.
- [x] 3.4 Add English comments explaining command/result/event purpose and operating principle.
- [x] 3.5 Add structured logs for model selection, dispatch, completion, failure, and snapshot emission.

## 4. Memory Service Contract

- [x] 4.1 Add provider-neutral Memory commands for remember, recall/search, prefetch, forget/delete, status, and snapshot.
- [x] 4.2 Reuse and expose `MemoryScope`, `AgentPrivate`, `SessionShared`, capability metadata, and topology labels.
- [x] 4.3 Ensure recall requires explicit application, session, and agent scope with no app-wide/global fallback.
- [x] 4.4 Add snapshot DTOs that expose health, capabilities, topology labels, governance counts, and audit ids without dumping content.
- [x] 4.5 Add English comments and structured logs for key memory call nodes.

## 5. Context Service Contract

- [x] 5.1 Add provider-neutral Context commands for assemble, active recall, provider inventory, engine inventory, and snapshot.
- [x] 5.2 Add assembled context, context report, active recall diagnostics, inventory, and snapshot DTOs.
- [x] 5.3 Bridge active recall to Memory Service client rather than a concrete memory backend.
- [x] 5.4 Keep LLM calls outside Context Service except future explicit service-call summarization strategies.
- [x] 5.5 Add English comments and structured logs for context composition, active recall, budget, report, and snapshot nodes.

## 6. Runtime-Host Service Providers

- [x] 6.1 Add LLM, Memory, and Context service provider wrappers in `macaca-runtime-host`.
- [x] 6.2 Translate `ServiceCommand` payloads into typed domain commands and structured results.
- [x] 6.3 Dispatch through injected domain facades/strategies without encoding provider, app, workflow, or model-specific branches.
- [x] 6.4 Enforce trace-required and policy decorators before dispatch.
- [x] 6.5 Return structured unavailable when a service is not configured.

## 7. SDK Focused Clients

- [x] 7.1 Add `SystemLlmClient`, `SystemMemoryClient`, and `SystemContextClient` traits.
- [x] 7.2 Add service-call-backed clients over `SystemServiceClient` / `ServiceCallCommand`; host-specific runtime adapters stay outside SDK to avoid dependency cycles.
- [x] 7.3 Add unavailable/null-object clients for shells without configured runtime.
- [x] 7.4 Add thin `SystemFacade` methods for chat, memory recall/prefetch, context assemble, and service snapshots where useful.
- [x] 7.5 Validate SDK remains a client/facade layer and not a provider factory.

## 8. Framework and Agent Construction Migration

- [x] 8.1 Add a service-backed `ChatModel` adapter over `SystemLlmClient`.
- [x] 8.2 Deprecate old provider-backed framework adapters while keeping them searchable.
- [x] 8.3 Make ReAct/context assembly injectable through `SystemContextClient` or a `ContextAssembler` strategy.
- [x] 8.4 Preserve behavior when no service client is installed.
- [x] 8.5 Log model/context dispatch nodes with trace ids.

## 9. Web Migration

- [x] 9.1 Build LLM/Memory/Context services during startup through runtime-host factories.
- [x] 9.2 Store service clients or runtime-backed facades in `AppState` as the primary path.
- [x] 9.3 Keep direct provider/backend fields only as deprecated compatibility fields until all call sites migrate.
- [x] 9.4 Migrate `ContextReportingModel` to Context Service for assembly and LLM Service for model calls.
- [x] 9.5 Migrate Memory active recall to Memory Service instead of direct `WebMemoryRuntime` access.
- [x] 9.6 Preserve `/api/chat/v2`, framework runner, trace viewer, context report, and SSE/EventLog behavior.

## 10. CLI Migration

- [x] 10.1 Replace production direct `StubLlmProvider` construction with SDK unavailable or runtime-backed LLM client.
- [x] 10.2 Keep stubs only in tests or explicit compatibility fixtures.
- [x] 10.3 Surface LLM/Memory/Context service availability through SDK/SystemFacade for status/inspect commands.
- [x] 10.4 Ensure missing services return structured unavailable instead of panic.

## 11. Kernel Compatibility Shrink

- [x] 11.1 Keep `KernelProviderCompat` and old provider-facing construction APIs deprecated and searchable.
- [x] 11.2 Route new construction through service-client compatibility bundle or SystemFacade where available.
- [x] 11.3 Stop new kernel code from requiring `LegacyLlmProvider`.
- [x] 11.4 Remove `macaca-kernel -> macaca-llm` and `macaca-kernel -> macaca-memory` only after references and dependency gate allow it.

## 12. Governance and Allowlist

- [x] 12.1 Update Route C governance with LLM Service, Memory Service, and Context Service ownership rules.
- [x] 12.2 Mark Web/CLI/framework as adapters for LLM/Memory/Context.
- [x] 12.3 Remove S5 allowlist rows only when direct dependency edges are actually gone.
- [x] 12.4 Update dependency boundary tests if allowlist rows are removed or explicitly changed.

## 13. Verification

- [x] 13.1 Run `openspec validate add-llm-memory-context-services-v1 --strict`.
- [x] 13.2 Run `cargo fmt --all --check`.
- [x] 13.3 Run `cargo test -p macaca-llm`.
- [x] 13.4 Run `cargo test -p macaca-memory`.
- [x] 13.5 Run `cargo test -p macaca-context`.
- [x] 13.6 Run `cargo test -p macaca-sdk llm_client`.
- [x] 13.7 Run `cargo test -p macaca-sdk memory_client`.
- [x] 13.8 Run `cargo test -p macaca-sdk context_client`.
- [x] 13.9 Run `cargo test -p macaca-runtime-host service_runtime`.
- [x] 13.10 Run `cargo test -p macaca-framework react_agent`.
- [x] 13.11 Run `cargo test -p macaca-web framework_runner`.
- [x] 13.12 Run `cargo test -p macaca-web context_reporting_model`.
- [x] 13.13 Run `cargo test -p macaca-cli`.
- [x] 13.14 Run `cargo test -p macaca-integration-tests route_c_dependency_boundaries`.
- [x] 13.15 Run `cargo test -p macaca-integration-tests --test route_c_baseline`.
- [x] 13.16 Run `cargo check --workspace`.
- [x] 13.17 Run `npx gitnexus detect-changes -r agent`.
