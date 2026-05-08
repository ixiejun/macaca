# Change: Converge SDK SystemFacade for Route C S3

## Why

Route C S3 requires upper layers to call system capabilities through a stable SDK/SystemFacade boundary instead of directly owning provider crates, kernel internals, task stores, trace replay, package inspection, or presentation-specific semantics.

`macaca-sdk` already contains a small `SystemFacade`, but it only covers task-board query and status snapshot. S3 expands that boundary into focused command/client modules while preserving current Web, CLI, application, gateway, `/api/chat/v2`, trace, task board, resume, driver, skill/MCP, and YAML application behavior.

## What Changes

- Add focused SDK client modules for service, task, trace, package, and status operations.
- Keep all shell/system operations command-shaped and provider-neutral.
- Refactor `SystemFacade` to compose focused clients rather than accumulating a large generic parameter list.
- Preserve existing task-board and status behavior through local compatibility adapters.
- Add structured logs and validation at command construction, facade execution, client delegation, rejection, and completion boundaries.
- Document that S3 defines the upper-layer system API boundary but does not migrate concrete providers; S4-S12 continue to own provider/service migrations.

## Non-Goals

- Do not migrate Task/Planner/Review service behavior; that belongs to S4.
- Do not migrate LLM, Memory, or Context providers; that belongs to S5.
- Do not migrate Driver, Skill, or MCP providers; that belongs to S6.
- Do not migrate Application Service lifecycle; that belongs to S7.
- Do not migrate Gateway Service/provider behavior; that belongs to S8.
- Do not complete Web/CLI thin shell cleanup; that belongs to S12.
- Do not add new provider construction hubs, application-specific branches, or hardcoded provider/app/workflow/driver/gateway names.

## Governance Inputs

- `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`
- `docs/superpowers/plans/2026-05-08-s3-sdk-system-facade-convergence-plan.md`
- `macaca/docs/agent-os-microkernel-boundaries.md`
- `macaca/docs/route-c-serviceization-allowlist.md`
- `macaca/docs/route-c-architecture-governance.md`

## Impact

- Affected specs: `sdk-system-facade`
- Affected code:
  - `macaca/crates/macaca-sdk/src/lib.rs`
  - `macaca/crates/macaca-sdk/src/system_facade.rs`
  - `macaca/crates/macaca-sdk/src/service_client.rs`
  - `macaca/crates/macaca-sdk/src/task_client.rs`
  - `macaca/crates/macaca-sdk/src/trace_client.rs`
  - `macaca/crates/macaca-sdk/src/package_client.rs`
  - `macaca/crates/macaca-sdk/src/status_client.rs`
  - `macaca/crates/macaca-web/src/shell.rs`
  - `macaca/crates/macaca-cli/src/commands.rs`
- Affected docs:
  - `macaca/docs/route-c-architecture-governance.md`
- Dependency gate:
  - S3 must not add new forbidden SDK/Web/CLI provider dependencies.
  - S3 may keep existing migration debt if it remains compatibility-only and documented.
