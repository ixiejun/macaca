# Macaca OS Serviceization Refactor Brainstorm And Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert the 2026-05-16 serviceization audit into a staged refactor that removes known boundary escape hatches without violating Macaca OS governance.

**Architecture:** The plan uses a freeze-first strategy: add executable guardrails before moving ownership. Refactors then remove debt by layer priority: kernel purity, Web thin shell, CLI decoupling, domain-pack externalization, provider-neutral routing, and OpenSpec baseline convergence.

**Tech Stack:** Rust workspace under `macaca/`, OpenSpec, GitNexus impact analysis, cargo metadata/tree, integration boundary gates, service runtime, SystemFacade/focused SDK clients.

---

## Governance Inputs

This plan is constrained by:

- `macaca/docs/2026-05-16-macaca-os-serviceization-implementation-audit.md`
- `macaca/docs/macaca-os-architecture-governance.md`
- `macaca/docs/macaca-os-microkernel-boundaries.md`
- `macaca/docs/macaca-os-serviceization-allowlist.md`
- `macaca/docs/design_patterns.md`
- `openspec/AGENTS.md`

The stable rule is strict: compatibility paths are migration debt, not accepted final architecture.

## Brainstorm

### Option A: Big-Bang Architecture Cleanup

Remove all allowlist rows, kernel compatibility providers, Web direct runtimes, CLI-Web dependency, and domain-pack implementation in one large refactor.

Benefit: fastest theoretical path to the stable architecture.

Risk: unacceptable regression surface across `/api/chat/v2`, SSE, session recovery, app lifecycle, WASM delegation, task execution, and local shell startup. It also makes each failure hard to attribute.

Decision: rejected.

### Option B: Freeze Escape Hatches, Then Retire Debt By Ownership Track

First add static gates that prevent new direct paths. Then remove debt in independent tracks: kernel provider compatibility, Web provider/runtime access, CLI coupling, domain-pack location, provider/model routing, and OpenSpec baseline.

Benefit: makes current debt executable and prevents growth before high-risk movement. Each removed allowlist row is backed by `cargo metadata` and the Route C dependency gate.

Risk: some deprecated paths remain during early phases.

Mitigation: each remaining path must be named in a migration module or allowlist row with owner, caller, replacement, and expiry phase.

Decision: recommended.

### Option C: OpenSpec Baseline First, Refactor Later

Archive completed changes and build baseline specs before touching implementation.

Benefit: improves traceability and reduces ambiguity.

Risk: does not stop new boundary violations while the backlog is cleaned up. It can also codify current migration debt as if it were stable behavior.

Decision: defer until guardrails exist, then run as a dedicated baseline convergence phase.

## Recommended Design

Use Option B.

Design patterns:

- **Specification:** executable dependency and static-reference gates define what is forbidden.
- **Facade:** Web and CLI call `SystemFacade` or focused SDK clients rather than providers.
- **Command:** cross-boundary operations use typed service commands/results.
- **Adapter/Bridge:** remaining legacy provider shapes live only behind explicit migration adapters.
- **Strategy:** provider/model routing moves from hardcoded branches to descriptors or configured resolver chains.
- **Decorator:** service calls keep trace, policy, audit, resource, and entitlement behavior at the boundary.
- **State/Memento/Observer:** service lifecycle, snapshots, checkpoints, audit, event log, and replay remain explicit.
- **Abstract Factory:** provider/module bootstrapping stays in approved composition roots such as runtime-host or plugin packages, not kernel or shells.

Non-goals:

- Do not rewrite the full Web shell in one pass.
- Do not move replaceable behavior into the kernel to reduce dependency edges.
- Do not special-case finance, crypto, app names, workflow names, providers, models, drivers, gateways, chains, or payment names in generic OS layers.
- Do not delete compatibility paths until callers are migrated and gates prove the direct edge is gone.

## Target Change Set

Use a new OpenSpec change:

- `freeze-serviceization-escape-hatches`

Expected artifacts:

- `openspec/changes/freeze-serviceization-escape-hatches/proposal.md`
- `openspec/changes/freeze-serviceization-escape-hatches/design.md`
- `openspec/changes/freeze-serviceization-escape-hatches/tasks.md`
- `openspec/changes/freeze-serviceization-escape-hatches/specs/serviceization-escape-hatches/spec.md`

This change should define executable requirements for:

- no new production references to deprecated Web `AppState` provider/runtime fields outside migration modules,
- no new production direct calls to `AppRuntime::start_app` or `start_app_from_file`,
- no new production direct driver/MCP runtime reads in Web outside migration modules,
- no hardcoded agent role names in production OS layers outside fixtures and manifests,
- allowlist rows must carry owner, current caller path, replacement, expiry phase, and validation command.

## File Responsibility Map

Initial OpenSpec and guardrails:

- Modify: `openspec/changes/freeze-serviceization-escape-hatches/**`
- Modify: `macaca/crates/tests/macaca-integration-tests/tests/route_c_dependency_boundaries/allowlist.rs`
- Modify or create: `macaca/crates/tests/macaca-integration-tests/tests/serviceization_escape_hatches.rs`
- Reference: `macaca/docs/macaca-os-architecture-governance.md`
- Reference: `macaca/docs/macaca-os-microkernel-boundaries.md`
- Reference: `macaca/docs/macaca-os-serviceization-allowlist.md`

Kernel purity track:

- Modify: `macaca/crates/kernel/macaca-kernel/src/provider_compat.rs`
- Modify: `macaca/crates/kernel/macaca-kernel/src/kernel.rs`
- Modify: `macaca/crates/kernel/macaca-kernel/src/kernel_builder.rs`
- Modify: `macaca/crates/kernel/macaca-kernel/Cargo.toml`
- Test: `macaca/crates/tests/macaca-integration-tests/tests/route_c_dependency_boundaries.rs`

Web thin-shell track:

- Modify: `macaca/crates/shells/macaca-web/src/state.rs`
- Modify: `macaca/crates/shells/macaca-web/src/framework_toolkit.rs`
- Split later: `macaca/crates/shells/macaca-web/src/framework_runner.rs`
- Split later: `macaca/crates/shells/macaca-web/src/loop_manager.rs`
- Split later: `macaca/crates/shells/macaca-web/src/chat_orchestrator.rs`

CLI decoupling track:

- Modify: `macaca/crates/shells/macaca-cli/src/commands.rs`
- Modify: `macaca/crates/shells/macaca-cli/src/command_handlers.rs`
- Add if needed: a small public bootstrap facade owned outside Web internals.

Domain-pack externalization track:

- Modify: `macaca/crates/runtime/macaca-runtime-host/src/domain_pack_service_provider.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/finance_live_data.rs`
- Add later: plugin/package-owned provider crate or package fixture.

OpenSpec baseline track:

- Archive completed OpenSpec changes in batches.
- Add baseline specs under `openspec/specs/` for service runtime, dependency gates, SDK/SystemFacade, application service, execution control, and thin shells.

## Task 1: Freeze Serviceization Escape Hatches

**Files:**

- Create: `openspec/changes/freeze-serviceization-escape-hatches/proposal.md`
- Create: `openspec/changes/freeze-serviceization-escape-hatches/design.md`
- Create: `openspec/changes/freeze-serviceization-escape-hatches/tasks.md`
- Create: `openspec/changes/freeze-serviceization-escape-hatches/specs/serviceization-escape-hatches/spec.md`
- Modify: `macaca/crates/tests/macaca-integration-tests/tests/route_c_dependency_boundaries/allowlist.rs`
- Create or modify: `macaca/crates/tests/macaca-integration-tests/tests/serviceization_escape_hatches.rs`

- [ ] Step 1: Run GitNexus impact for the first symbols touched.

  Run impact analysis before editing `entries`, Web state fields, kernel constructors, or toolkit functions. If risk is HIGH or CRITICAL, report the blast radius before editing.

- [ ] Step 2: Create the OpenSpec change.

  The proposal must state that this change does not remove behavior. It freezes new violations and makes existing migration debt executable.

- [ ] Step 3: Add static reference gates.

  The test should scan production Rust sources and fail when forbidden tokens appear outside approved migration modules, fixtures, or tests. Initial forbidden families:

  - `AppRuntime::start_app`
  - `start_app_from_file`
  - `state.driver_runtime`
  - `state.mcp_runtime`
  - `state.runtime`
  - `state.registry`
  - hardcoded production role names: `"coordinator"`, `"planner"`, `"worker"`, `"backend"`, `"frontend"`, `"architect"`

- [ ] Step 4: Enrich allowlist rows.

  Add metadata for each existing row: owner track, current caller evidence, replacement service/facade, expiry phase, and validation command. Do not delete a row until `cargo metadata` proves the edge is gone.

- [ ] Step 5: Validate.

  Run:

  ```bash
  openspec validate freeze-serviceization-escape-hatches --strict
  cargo test -p macaca-integration-tests route_c_dependency_boundaries_reject_unallowlisted_forbidden_edges -- --nocapture
  cargo test -p macaca-integration-tests serviceization_escape_hatches -- --nocapture
  ```

## Task 2: Remove Production Kernel Provider Compatibility

**Files:**

- Modify: `macaca/crates/kernel/macaca-kernel/src/provider_compat.rs`
- Modify: `macaca/crates/kernel/macaca-kernel/src/kernel.rs`
- Modify: `macaca/crates/kernel/macaca-kernel/src/kernel_builder.rs`
- Modify: `macaca/crates/kernel/macaca-kernel/Cargo.toml`
- Test: `macaca/crates/tests/macaca-integration-tests/tests/route_c_dependency_boundaries.rs`

- [ ] Step 1: Run GitNexus impact for `Kernel`, `KernelBuilder`, `KernelProviderCompat`, and `execute_agent`.

- [ ] Step 2: Introduce a service-client execution port.

  The kernel should receive provider-neutral service-client handles or a command port for execution. It must not own concrete LLM, tool, task, driver, gateway, skill, memory, or persist providers.

- [ ] Step 3: Move legacy `Agent::run(llm, tools, services)` wiring out of production kernel construction.

  Keep any remaining adapter in runtime-host/application-framework service provider code or in explicit migration/test modules.

- [ ] Step 4: Prune kernel dependencies one edge at a time.

  Target first:

  - `macaca-kernel -> macaca-task`
  - `macaca-kernel -> macaca-tools`
  - `macaca-kernel -> macaca-persist`

- [ ] Step 5: Delete allowlist rows only after proof.

  Run:

  ```bash
  cargo metadata --no-deps --format-version 1
  cargo tree -e normal -p macaca-kernel --depth 1
  cargo test -p macaca-integration-tests route_c_dependency_boundaries_reject_unallowlisted_forbidden_edges -- --nocapture
  ```

## Task 3: Migrate Web Toolkit And Runtime Reads To Focused Clients

**Files:**

- Modify: `macaca/crates/shells/macaca-web/src/framework_toolkit.rs`
- Modify: `macaca/crates/shells/macaca-web/src/state.rs`
- Modify service client/provider files only if an existing command is missing.

- [ ] Step 1: Run GitNexus impact for `build_framework_toolkit` or the current toolkit entry symbol.

- [ ] Step 2: Remove deprecated driver fallback.

  `framework_toolkit` already calls `SystemDriverClient` first. Replace the deprecated `state.driver_runtime.collect_tools().await` fallback with structured unavailable diagnostics and an audit/trace event.

- [ ] Step 3: Replace direct MCP definition reads.

  Replace `state.mcp_runtime.definitions().await` with a typed MCP snapshot/catalog command. If the client lacks the command, add it to the MCP service contract rather than reading runtime internals from Web.

- [ ] Step 4: Add Web production guard coverage.

  The static test from Task 1 must fail if new Web production code references deprecated direct fields outside migration modules.

- [ ] Step 5: Validate chat/session behavior.

  Run targeted Web/service tests plus:

  ```bash
  cargo test -p macaca-integration-tests route_c_dependency_boundaries_reject_unallowlisted_forbidden_edges -- --nocapture
  ```

## Task 4: Move Session Loop Semantics Out Of Web

**Files:**

- Modify/split: `macaca/crates/shells/macaca-web/src/loop_manager.rs`
- Modify/split: `macaca/crates/shells/macaca-web/src/chat_orchestrator.rs`
- Modify/split: `macaca/crates/shells/macaca-web/src/framework_runner.rs`
- Use: execution-control/task service APIs.

- [ ] Step 1: Inventory semantic ownership in the three oversized Web files.

  Classify code as route adapter, SSE adapter, session-channel adapter, service-client adapter, or migration-only.

- [ ] Step 2: Extract only adapter-shaped modules first.

  Keep response shape stable. Do not rewrite planner/task semantics in Web.

- [ ] Step 3: Move pause/resume/checkpoint/status semantics to `service.execution_control` or task service commands.

- [ ] Step 4: Keep Web responsible only for HTTP DTO mapping, SSE subscription, GenUI mounting, approval rendering, and diagnostics.

- [ ] Step 5: Validate `/api/chat/v2`, session recovery, task board isolation, trace replay, and SSE event ordering.

## Task 5: Decouple CLI From Web And Provider Construction

**Files:**

- Modify: `macaca/crates/shells/macaca-cli/src/commands.rs`
- Modify: `macaca/crates/shells/macaca-cli/src/command_handlers.rs`
- Add if required: small public bootstrap facade outside Web internals.

- [ ] Step 1: Run GitNexus impact for CLI command handlers that construct kernel, gateway, tools, or Web server state.

- [ ] Step 2: Replace status/run provider construction with SDK/runtime service inspector clients.

- [ ] Step 3: Remove `macaca-cli -> macaca-web` as a direct internal dependency.

  If `macaca web` still needs to start a server, expose only a tiny public bootstrap contract or binary entrypoint facade. CLI must not import Web internals.

- [ ] Step 4: Validate:

  ```bash
  cargo tree -e normal -p macaca-cli --depth 1
  cargo test -p macaca-integration-tests route_c_dependency_boundaries_reject_unallowlisted_forbidden_edges -- --nocapture
  ```

## Task 6: Externalize Finance/Crypto Domain Packs

**Files:**

- Modify: `macaca/crates/runtime/macaca-runtime-host/src/domain_pack_service_provider.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/finance_live_data.rs`
- Add later: plugin/package domain-pack provider fixture outside base runtime-host.

- [ ] Step 1: Run GitNexus impact for domain-pack provider symbols.

- [ ] Step 2: Split generic registration mechanics from finance/crypto behavior.

  Runtime-host may keep provider factory and service registration mechanics. It must not own finance/crypto business rules as base behavior.

- [ ] Step 3: Move fixed service IDs and exchange/RSS adapters into plugin/package provider metadata.

- [ ] Step 4: Keep deterministic fixtures in tests only.

- [ ] Step 5: Validate optional absence.

  Missing domain-pack providers must return structured unavailable/disabled states, not crash, hang, silent fallback, or fake success.

## Task 7: Make LLM Provider/Model Routing Descriptor-Driven

**Files:**

- Modify: `macaca/crates/services/macaca-llm/src/router.rs`
- Modify: `macaca/crates/services/macaca-llm/src/resolver.rs`
- Add tests under `macaca/crates/services/macaca-llm/`.

- [ ] Step 1: Run GitNexus impact for router and resolver symbols.

- [ ] Step 2: Move built-in provider/model prefix rules into descriptors or resolver-chain configuration.

- [ ] Step 3: Keep default resolver behavior compatible, but make it data-driven.

- [ ] Step 4: Add audit tests proving kernel/Web/CLI do not branch on provider or model names.

## Task 8: Align OpenSpec Baseline

**Files:**

- Modify: `openspec/specs/**`
- Archive completed changes under `openspec/changes/archive/**`

- [ ] Step 1: Archive baseline governance changes first.

  Start with service runtime, dependency gate, SDK/SystemFacade, Web/CLI thin shell, application service, and execution-control service.

- [ ] Step 2: Add or update baseline specs.

  Baseline specs must describe current stable contracts, not historical task lists.

- [ ] Step 3: Validate after each batch:

  ```bash
  openspec validate --strict
  ```

## Global Verification Gates

Run these after each ownership track:

```bash
cargo metadata --no-deps --format-version 1
cargo test -p macaca-integration-tests route_c_dependency_boundaries_reject_unallowlisted_forbidden_edges -- --nocapture
openspec validate --strict
```

Run these before declaring the whole refactor complete:

```bash
cargo test --workspace
openspec validate --strict
```

Manual validation must cover:

- Existing YAML, WASM, and GenUI applications still run.
- `/api/chat/v2` session creation and recovery do not regress.
- Task boards remain session-scoped.
- Trace and audit evidence remain replayable after refresh.
- Optional Driver, Skill, MCP, LLM, Memory, Context, Application, Store, Payment, Web3, and EVM failures are structured unavailable/denied states.
- Logs, snapshots, and diagnostics are sanitized.

## Completion Criteria

- The dependency gate has zero allowlist rows for kernel provider dependencies.
- Web no longer directly reads Driver/MCP/Application runtime internals outside migration-only tests.
- CLI no longer depends on `macaca-web`, `macaca-gateway`, or `macaca-tools` as presentation-owned provider construction paths.
- Domain-pack finance/crypto implementation is no longer base runtime-host behavior.
- Provider/model routing is descriptor or strategy driven inside the LLM service.
- OpenSpec baseline specs represent current stable architecture.
- No new hardcoded role/provider/model/app/workflow/domain names exist in generic OS production code.

## Execution Recommendation

Use subagent-driven execution per task, but keep each task's write set disjoint. Task 1 should be implemented first and reviewed before any ownership movement. Tasks 2, 3, and 5 can proceed independently after Task 1 lands. Tasks 6 and 7 should wait until the freeze gates are active. Task 8 should run after the implementation shape is stable enough to avoid archiving migration debt as baseline truth.
