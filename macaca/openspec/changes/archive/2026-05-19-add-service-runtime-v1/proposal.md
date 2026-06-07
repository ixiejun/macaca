# Change: Add ServiceRuntime v1

## Why

Route C requires Macaca to move from macro-kernel-style direct provider wiring toward a microkernel Agent OS where replaceable capabilities run as system services. S0 added the dependency boundary gate; S1 must add the runtime layer that can register, start, call, stop, clean up, trace, policy-check, and snapshot provider-neutral services before later phases migrate concrete providers.

## What Changes

- Add a host-owned `ServiceRuntime` facade in `macaca-runtime-host`.
- Add provider-neutral `ServiceProviderFactory` abstractions for built-in and future plugin-backed services.
- Add runtime decorator chain for trace-required and policy-required admission control, with extension points for resource, entitlement, and metering.
- Bridge runtime calls through the existing `macaca-ipc` service bus and `macaca-kernel` `SystemService` adapter path.
- Add deterministic runtime snapshots and structured runtime lifecycle/call events.
- Add no-network runtime tests using mock services.
- Update governance documentation to clarify that ServiceRuntime is host-owned orchestration, not kernel provider ownership.

## Non-Goals

- Do not migrate Task, LLM, Memory, Driver, Skill, MCP, Gateway, Payment, Web3, or EVM providers.
- Do not remove existing S0 allowlist rows.
- Do not change Web UI, CLI, `/api/chat/v2`, YAML application loading, trace viewer, task board, resume, driver, or skill/MCP behavior.
- Do not introduce app/provider/workflow/model/driver/gateway/chain/business hardcoding.
- Do not implement remote service transport, real entitlement enforcement, real metering, or concrete provider factories in S1.

## Governance Inputs

- `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`
- `docs/superpowers/plans/2026-05-08-s1-service-runtime-v1-plan.md`
- `macaca/docs/agent-os-microkernel-boundaries.md`
- `macaca/docs/route-c-serviceization-allowlist.md`
- `macaca/docs/route-c-architecture-governance.md`

## Impact

- Affected specs: `service-runtime`
- Affected code:
  - `macaca/crates/macaca-runtime-host/src/service_runtime.rs`
  - `macaca/crates/macaca-runtime-host/src/service_provider.rs`
  - `macaca/crates/macaca-runtime-host/src/service_decorator.rs`
  - `macaca/crates/macaca-runtime-host/src/lib.rs`
  - `macaca/crates/macaca-runtime-host/Cargo.toml`
  - `macaca/crates/macaca-runtime-host/tests/service_runtime.rs`
- Affected docs:
  - `macaca/docs/route-c-architecture-governance.md`
- Dependency gate:
  - S1 may add `macaca-runtime-host -> macaca-ipc`, which is expected to be valid runtime-host to IPC/service-bus coupling.
  - S1 must not add kernel-to-provider, presentation-to-provider, or provider-to-presentation edges.
