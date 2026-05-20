# Design: SDK SystemFacade Convergence

## Context

Route C separates microkernel invariants, replaceable services, application behavior, and presentation shells. S3 is the point where `macaca-sdk` becomes the stable upper-layer system API boundary. Web, CLI, gateway, applications, and future plugins should translate input into typed SDK commands instead of directly owning lower-layer system semantics.

Existing code already has useful foundations:

- `macaca-sdk/src/system_facade.rs` contains task-board and status snapshot commands.
- `macaca-web/src/shell.rs` already delegates task-board reads through `SystemFacade`.
- `macaca-web/src/route_command.rs` already models HTTP handlers as command adapters.
- `macaca-cli` already routes some commands through compatibility helpers and uses `SystemFacade` for status snapshot formatting.

S3 should converge these foundations without stealing provider migrations from S4-S12.

## Goals

- Make SDK/SystemFacade the canonical upper-layer boundary for system operations.
- Split SDK clients by capability family so the facade remains composable and readable.
- Model operations as typed commands with validation, scope, and trace/policy-ready metadata.
- Preserve existing task-board/status behavior through local compatibility adapters.
- Make future service-runtime/service-bus dispatch possible without changing Web/CLI contracts.
- Emit structured logs at command validation, facade execution, client delegation, success, rejection, and failure.

## Non-Goals

- No full Web/CLI rewrite.
- No new provider implementation.
- No mandatory `ServiceRuntime` dispatch for all operations yet.
- No direct construction of concrete LLM, memory, task planner, driver, skill, MCP, gateway, package, or payment providers from SDK clients.
- No generic untyped RPC dumping ground.

## Decisions

### Decision: Use focused SDK client modules

S3 will add focused client modules:

- `service_client.rs`
- `task_client.rs`
- `trace_client.rs`
- `package_client.rs`
- `status_client.rs`

Each module owns its command/result types, client trait, local compatibility adapters, and no-network tests.

Why:

- It keeps each file below 500 lines.
- It avoids generic parameter explosion in `SystemFacade`.
- It gives later S4-S12 phases a stable place to plug runtime-backed clients.

### Decision: Keep `SystemFacade` as the facade, not a provider factory

`SystemFacade` composes client traits and exposes upper-layer methods. It validates and logs, then delegates to focused clients.

Why:

- Upper layers should depend on one stable facade.
- SDK must not become another macro-kernel or provider construction hub.
- This preserves clean layering while supporting local compatibility adapters.

### Decision: Use typed command objects

Every facade operation should accept a typed command object with bounded scope, cursor/limit fields where needed, and trace/policy-ready metadata where appropriate.

Why:

- Commands are auditable and testable.
- They can later cross `ServiceBus` or `ServiceRuntime` without changing shell contracts.
- They prevent ad hoc `(id, json, option)` parameter bundles from spreading.

### Decision: Unsupported operations return structured errors

Some S3 operations will have command/client contracts before concrete services exist. Those paths must return structured unavailable/unsupported errors instead of panicking or hanging.

Why:

- Route C optional/service capabilities must fail explicitly.
- Web/CLI can map structured errors to stable responses.
- Future runtime-backed clients can replace the local adapter without changing contracts.

## Alternatives Considered

### Expand `SystemFacade<T, S, ...>` with more generic parameters

Rejected because it scales poorly once service, task, trace, package, status, and approval clients are added.

### Force all SDK calls through `ServiceRuntime` now

Rejected because S4-S8 service providers are not fully migrated yet. S3 should be additive and compatible.

### Let Web/CLI keep direct lower-crate semantics

Rejected because it contradicts S3 and keeps presentation shells as system coordinators.

## Risks and Mitigations

- Risk: SDK becomes a provider construction hub.
  - Mitigation: clients are traits/adapters; they must not construct concrete providers.
- Risk: S3 overlaps with later phases.
  - Mitigation: S3 provides contracts and safe first consumers only; provider/service migrations remain in S4-S12.
- Risk: Facade methods become generic untyped RPC.
  - Mitigation: only typed command/result APIs are allowed.
- Risk: Web/CLI response shapes drift.
  - Mitigation: preserve current task-board/status output and add compatibility tests.
- Risk: trace/policy are modeled but not enforced for every path.
  - Mitigation: commands carry trace/policy-ready fields and emit structured logs now; enforcement can be added via later decorators/clients.

## Verification

- `openspec validate update-sdk-system-facade-convergence --strict`
- `cargo fmt --all --check`
- `cargo test -p macaca-sdk`
- `cargo test -p macaca-web`
- `cargo check -p macaca-cli`
- `cargo test -p macaca-integration-tests route_c_dependency_boundaries`
- `cargo test -p macaca-integration-tests --test route_c_baseline`
- `cargo check --workspace`
- `npx gitnexus detect-changes -r agent`
