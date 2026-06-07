# S3 SDK/SystemFacade 收敛 Implementation Plan

## Scope

Implement S3 from `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`: make `macaca-sdk` the stable SystemFacade boundary for upper-layer system calls.

S3 establishes SDK command/client contracts and migrates the safest upper consumers to that boundary. It does not serviceize every provider. Later phases still own concrete provider migrations.

## Required Governance Inputs

- `macaca/docs/agent-os-microkernel-boundaries.md`
- `macaca/docs/route-c-serviceization-allowlist.md`
- `macaca/docs/route-c-architecture-governance.md`
- `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`
- `docs/superpowers/plans/2026-05-08-s0-serviceization-boundary-audit-plan.md`
- `docs/superpowers/plans/2026-05-08-s1-service-runtime-v1-plan.md`
- `docs/superpowers/plans/2026-05-08-s2-kernel-provider-dependency-removal-plan.md`

## Architecture Decision

Use typed SDK clients composed by `SystemFacade`.

Design patterns:

- Facade: `SystemFacade` remains the one upper-layer entry point for system operations.
- Command: Web, CLI, gateway, and application inputs become typed commands before execution.
- Adapter: current stores/kernel/runtime state adapt to client traits until real services exist.
- Bridge: client contracts can later dispatch through `ServiceRuntime` or `ServiceBus`.
- Strategy: local, runtime-backed, remote, or mock client implementations are replaceable.
- Observer: every operation logs start, completion, rejection, and key IDs.
- Specification: command constructors validate required scope, pagination, trace/policy fields, and unsupported operations.
- Memento: query/tail/replay commands use cursor/snapshot fields where needed.

Rejected alternatives:

- Keep expanding one generic `SystemFacade<T, S, ...>` type indefinitely: rejected due to type growth.
- Route all SDK calls through `ServiceRuntime` immediately: rejected because S4-S8 providers are not fully migrated.
- Let Web/CLI keep calling lower crates directly: rejected because it contradicts S3 and Route C.

## Proposed OpenSpec Change

Expected change id:

- `update-sdk-system-facade-convergence`

Expected artifacts:

- `openspec/changes/update-sdk-system-facade-convergence/proposal.md`
- `openspec/changes/update-sdk-system-facade-convergence/design.md`
- `openspec/changes/update-sdk-system-facade-convergence/tasks.md`
- `openspec/changes/update-sdk-system-facade-convergence/specs/sdk-system-facade/spec.md`

The proposal should state:

- SDK/SystemFacade is the stable upper-layer boundary for system operations.
- Web/CLI/Gateway/Application code should convert inputs to typed commands and call SDK clients.
- SDK clients must not construct concrete providers.
- S3 is additive and does not replace S4-S12 service migrations.

## Implementation Slices

### Slice S3.1: Impact and Current Dependency Audit

Files to inspect before editing:

- `macaca/crates/macaca-sdk/Cargo.toml`
- `macaca/crates/macaca-sdk/src/lib.rs`
- `macaca/crates/macaca-sdk/src/system_facade.rs`
- `macaca/crates/macaca-sdk/src/facade.rs`
- `macaca/crates/macaca-sdk/src/registry_api.rs`
- `macaca/crates/macaca-web/src/shell.rs`
- `macaca/crates/macaca-web/src/routes.rs`
- `macaca/crates/macaca-web/src/sse.rs`
- `macaca/crates/macaca-cli/src/commands.rs`
- `macaca/crates/macaca-gateway/src/*`

Required actions:

1. Run GitNexus impact before modifying any existing symbol.
2. Audit direct `macaca-web` / `macaca-cli` / `macaca-gateway` calls into provider or system-semantic crates.
3. Confirm no new dependency allowlist rows are needed.

### Slice S3.2: SDK Client Module Skeletons

Files:

- New: `macaca/crates/macaca-sdk/src/service_client.rs`
- New: `macaca/crates/macaca-sdk/src/task_client.rs`
- New: `macaca/crates/macaca-sdk/src/trace_client.rs`
- New: `macaca/crates/macaca-sdk/src/package_client.rs`
- Optional new: `macaca/crates/macaca-sdk/src/status_client.rs`
- Modify: `macaca/crates/macaca-sdk/src/lib.rs`

Behavior:

- Move or re-export existing command types from `system_facade.rs` into focused modules where appropriate.
- Define trait-based client boundaries:
  - `SystemServiceClient`
  - `SystemTaskClient`
  - `SystemTraceClient`
  - `SystemPackageClient`
  - `SystemStatusClient`
- Each operation should accept a command object, not ad hoc parameters.
- Command constructors must validate non-empty IDs, bounded limits, and required scope.

Rules:

- No provider construction.
- No app/provider/workflow/model/driver/gateway hardcoding.
- Detailed English comments explaining each client and command role.
- Structured logs for start/completion/rejection.
- Keep files below 500 lines.

### Slice S3.3: SystemFacade Composition

Files:

- Modify: `macaca/crates/macaca-sdk/src/system_facade.rs`

Behavior:

- Refactor `SystemFacade` to compose focused client traits or a small client bundle.
- Preserve existing `query_task_board` and `status_snapshot` behavior.
- Add facade methods for:
  - service inspection/query
  - service call placeholder or unsupported response where no service exists yet
  - trace tail/replay
  - package inspection
  - approval decision recording or policy-ready placeholder
- Unsupported operations must return structured errors, not panic or hang.

Rules:

- Facade methods must be thin orchestration boundaries.
- They should validate command shape, log intent and outcome, then delegate to clients.
- If a command requires trace by Route C rules, model trace fields now even if the backing adapter is local.

### Slice S3.4: Local Compatibility Adapters

Files:

- Modify or add SDK adapter files as needed.

Initial adapters:

- Task board adapter over existing `TodoStore`.
- Status adapter over prepared snapshot or kernel count snapshot.
- Trace adapter over current event/replay sources where feasible.
- Package inspection adapter over current package/app metadata where feasible.
- Service inspection adapter over `KernelFacade`/service registry or `ServiceRuntimeSnapshot` where feasible.

Rules:

- Adapters are compatibility bridges, not provider hubs.
- Adapters must be clearly named as local/current-state adapters.
- Adapters should be replaceable by runtime-backed clients later.

### Slice S3.5: Web Shell First Consumer Migration

Files:

- Modify: `macaca/crates/macaca-web/src/shell.rs`
- Targeted route files only if behavior-preserving:
  - `macaca/crates/macaca-web/src/routes.rs`
  - `macaca/crates/macaca-web/src/sse.rs`

Behavior:

- Keep existing response shapes.
- Use SDK commands for task board reads, trace tail/replay surfaces, and service/package inspection where safe.
- Do not migrate PlanLoop/WorkerLoop/review execution in S3; that belongs to S4.
- Do not migrate LLM/Memory/Context provider calls in S3; that belongs to S5.

Rules:

- Web remains HTTP/SSE adapter.
- Web must not define new system semantics.
- Web logs should identify route command, app/session scope, and facade result.

### Slice S3.6: CLI First Consumer Migration

Files:

- Modify: `macaca/crates/macaca-cli/src/commands.rs`
- Potentially add CLI command handler integration if needed.

Behavior:

- CLI status/list/inspect commands should call SDK/SystemFacade where possible.
- Keep stdout text and exit behavior compatible.
- Do not remove current deprecated helpers until all callers migrate.
- Do not make CLI a provider construction hub beyond existing compatibility paths; prefer SDK client adapters.

Rules:

- CLI stays command shell.
- CLI should map terminal input to SDK commands and print results.

### Slice S3.7: Gateway/Application Boundary Notes

Files:

- Potentially update docs only in S3 unless there is a safe no-op command adapter.

Behavior:

- Define how future gateway events become `SystemFacade` commands.
- Define how applications request service/package/trace operations through SDK command builders.
- Do not migrate gateway providers in S3; that belongs to S8.
- Do not migrate application lifecycle service in S3; that belongs to S7.

### Slice S3.8: Dependency Pruning and Allowlist Review

Files:

- Modify: `macaca/crates/macaca-sdk/Cargo.toml` only if dependencies can be removed safely.
- Potentially update: `macaca/docs/route-c-serviceization-allowlist.md` if a direct dependency is genuinely eliminated.

Behavior:

- Prefer moving direct SDK provider dependencies behind tests or compatibility adapters.
- Do not add new forbidden edges.
- If a dependency remains, document it as active migration debt.

### Slice S3.9: Documentation

Files:

- Modify: `macaca/docs/route-c-architecture-governance.md`
- Potentially add SDK facade docs if needed.

Documentation must state:

- SDK/SystemFacade is the upper-layer system API boundary.
- SDK clients are command-driven adapters, not provider factories.
- Web/CLI/Gateway/Application code should convert inputs to SDK commands.
- Full provider service migrations remain assigned to S4-S12.

## Verification

Run after implementation:

```bash
openspec validate update-sdk-system-facade-convergence --strict
cargo fmt --all --check
cargo test -p macaca-sdk
cargo test -p macaca-web
cargo check -p macaca-cli
cargo test -p macaca-integration-tests route_c_dependency_boundaries
cargo test -p macaca-integration-tests --test route_c_baseline
cargo check --workspace
npx gitnexus detect-changes -r agent
```

Expected result:

- SDK facade/client tests pass.
- Web/CLI behavior-compatible tests pass or compile.
- Route C dependency gate remains green.
- No new provider construction dependency is introduced.
- Remaining dependency debt is documented and bounded.

## Completion Criteria

- Superpowers brainstorm and plan exist.
- OpenSpec proposal/design/tasks/spec exists and validates before implementation.
- SDK has focused client/command boundaries for S3 scope.
- `SystemFacade` composes clients and remains the canonical upper-layer API surface.
- Safe Web/CLI consumers use SDK commands instead of direct semantic calls where feasible.
- Deprecated direct helper paths remain searchable.
- All new implementation code has detailed English comments and structured logs at key execution nodes.
- No Route C regression matrix scenario is broken.
