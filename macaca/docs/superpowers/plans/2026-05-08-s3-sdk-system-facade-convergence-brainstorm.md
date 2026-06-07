# S3 SDK/SystemFacade 收敛 Brainstorm

## Context

S3 follows `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`.

S0 has installed the dependency boundary gate. S1 has introduced host-owned `ServiceRuntime`. S2 has started moving `macaca-kernel` provider-facing construction behind a compatibility boundary. S3 now needs to make `macaca-sdk` the stable upper-layer system facade so application, web, CLI, gateway, and future plugins can call system capabilities without directly owning provider or internal crate semantics.

Current state:

- `macaca-sdk/src/system_facade.rs` already has a small shell-facing facade for task-board query and status snapshot.
- `macaca-sdk` still directly depends on `macaca-kernel`, `macaca-task`, `macaca-llm`, and `macaca-tools`.
- `macaca-web` still directly owns task/session/trace/app orchestration surfaces in many modules.
- `macaca-cli` still constructs a kernel and stub provider directly for status/list/run compatibility.
- `macaca-web/src/shell.rs` already demonstrates a thin adapter shape for task-board reads through `SystemFacade`.
- `macaca-web/src/route_command.rs` already defines a lightweight route command boundary.
- Route C later phases still own the real provider service migrations: S4 Task Service, S5 LLM/Memory/Context Service, S6 Driver/Skill/MCP Service, S7 Application Service, S8 Gateway Service, and S12 full Web/CLI thin shell.

Therefore S3 should not rewrite all Web/CLI/provider code. It should define the canonical SDK command/client/facade surface and migrate the safe, read-only or shell-facing consumers first.

## Constraints

- Must follow `macaca/docs/agent-os-microkernel-boundaries.md`.
- Must follow `macaca/docs/route-c-serviceization-allowlist.md`.
- Must follow `macaca/docs/route-c-architecture-governance.md`.
- Must not hardcode app/provider/workflow/model/driver/gateway/business names.
- Must keep existing YAML apps, `/api/chat/v2`, trace viewer, task board, resume, driver, skill/MCP behavior compatible.
- Must not add new provider construction hubs to SDK, Web, CLI, or Gateway.
- All future implementation code must include detailed English comments explaining purpose and runtime principles.
- All key execution nodes must use structured logs.
- Rust files must remain under 500 lines; split clients by capability family.
- For later code changes, run GitNexus impact before editing symbols and `gitnexus detect-changes` before commit.

## Design Pattern Options

### Option A: Expand the current generic `SystemFacade<T, S>` only

Keep `SystemFacade<T, S>` generic over data sources and keep adding more generic parameters for service, task, trace, package, status, and approvals.

Patterns:

- Facade
- Adapter
- Command

Pros:

- Smallest immediate code change.
- Preserves the current implementation style.
- Easy to test with in-memory data sources.

Cons:

- Generic parameter explosion will make `SystemFacade<T, S, ...>` hard to use.
- Web/CLI will need verbose type signatures.
- Harder to add optional clients without changing facade type shape.

Risk:

- Medium maintainability risk. This is acceptable for two data sources, but it will not scale to all Route C service families.

### Option B: Typed service-client modules composed by a `SystemFacade` facade

Create focused SDK modules:

- `service_client.rs`
- `task_client.rs`
- `trace_client.rs`
- `package_client.rs`
- optional `status_client.rs`
- optional `approval_client.rs`

Then make `SystemFacade` compose client traits or boxed client handles. Web/CLI convert input into typed command objects and call the facade.

Patterns:

- Facade: `SystemFacade` is the one upper-layer API surface.
- Command: each operation is a typed command with scope, trace, pagination, and policy fields.
- Adapter: current stores, kernel primitives, and future `ServiceRuntime` handles adapt into client traits.
- Bridge: facade commands can later dispatch through `macaca-ipc::ServiceBus` without changing Web/CLI.
- Strategy: client backends are replaceable local/runtime/remote implementations.
- Observer: every call emits trace/audit-friendly logs and can later emit trace events.
- Specification: commands validate required scope, pagination, trace, and policy preconditions.

Pros:

- Scales better as S4-S12 add real service clients.
- Keeps each Rust file smaller and easier to review.
- Allows additive migration: new clients can be backed by current local stores first, then by service runtime later.
- Gives Web/CLI/Gateway a stable API without forcing provider migration immediately.

Cons:

- More files and traits than Option A.
- Requires discipline to avoid SDK becoming another provider construction hub.
- Requires compatibility adapters until services are real.

Risk:

- Low to medium if clients stay thin and command-driven. The key guardrail is that SDK clients must not construct concrete providers; they adapt existing state or service/runtime handles.

### Option C: SDK delegates everything directly to `ServiceRuntime`

Make `SystemFacade` require a runtime handle and make all calls go through `ServiceRuntime` now.

Patterns:

- Facade
- Bridge
- Command
- Decorator

Pros:

- Best long-term architecture.
- Strongly aligns with S1 service runtime.

Cons:

- Too early: many service providers are not migrated yet.
- Would force S4-S8 work into S3.
- Higher regression risk for Web/CLI and current YAML app flows.

Risk:

- High schedule and behavior risk. This is the target direction but not the right immediate implementation strategy.

### Option D: Leave SDK as-is and migrate Web/CLI directly to existing service/provider crates

Do nothing substantial in SDK; move Web/CLI call sites to lower crates more cleanly.

Pros:

- Low SDK churn.
- Can fix individual Web/CLI issues quickly.

Cons:

- Violates S3's purpose.
- Reinforces Web/CLI as coordination hubs.
- Does not reduce provider or system-semantic leakage.

Risk:

- High architecture risk. This should be rejected.

## Recommended Approach

Choose Option B.

S3 should introduce a composable SDK facade made from focused command/client modules. It should keep current behavior by adapting existing local stores and kernel primitives, while defining the stable command surface that later phases can rewire to `ServiceRuntime` and `ServiceBus`.

S3 should be a boundary-convergence phase, not a full provider-migration phase:

- Define SDK command/client contracts for service inspection/call, task board/task command, trace replay/tail, package inspection, approval decision, and system status.
- Keep local adapters where services are not yet migrated.
- Migrate the safest current consumers first: task-board reads, status snapshots, service/package inspection placeholders, and trace read/tail facade shape.
- Leave full task planner/review service migration to S4.
- Leave LLM/Memory/Context migration to S5.
- Leave Driver/Skill/MCP migration to S6.
- Leave Application Service to S7.
- Leave Gateway Service to S8.
- Leave full Web/CLI thin shell cleanup to S12.

## Key Risks and Mitigations

- Risk: SDK becomes a new macro-kernel/provider construction hub.
  - Mitigation: SDK clients are traits and adapters; they must not construct LLM, driver, gateway, memory, task planner, or provider runtimes.

- Risk: Facade generic type explosion.
  - Mitigation: split client modules and compose a facade from client traits or boxed trait objects rather than one massive generic signature.

- Risk: S3 overlaps with S4-S12.
  - Mitigation: S3 defines command/client boundaries and migrates read-only shell paths only. Provider/service behavior stays in later phases.

- Risk: Trace and policy are modeled as future concerns only.
  - Mitigation: command objects must carry trace/policy-ready fields where applicable. Calls must log start/completion/rejection now and can emit real trace events later.

- Risk: Web/CLI behavior changes while moving through facade.
  - Mitigation: preserve existing JSON and CLI output shapes. Add no-network tests around facade adapters and shell mapping.

- Risk: Existing dependency gate still allowlists SDK/Web/CLI provider debt.
  - Mitigation: S3 should reduce or document debt, not add new exceptions. Any unavoidable exception requires OpenSpec and allowlist updates.

- Risk: SystemFacade clients become too abstract for current code.
  - Mitigation: keep clients capability-focused, command-shaped, and backed by current local adapters first.

## Decision

Proceed with a typed SDK client convergence strategy:

- Use `SystemFacade` as the upper-layer facade.
- Split clients by service family.
- Model every shell/system operation as a command.
- Back clients with current local adapters first.
- Add structured logs and validation in each client.
- Migrate only safe first consumers in S3.
- Keep old direct helpers deprecated and searchable until later phases migrate them fully.
