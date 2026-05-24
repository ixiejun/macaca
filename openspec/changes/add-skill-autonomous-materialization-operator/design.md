## Context

Run 38 showed two facts. First, a failed live `/api/chat/v2` execution correctly
does not produce a proposal. Second, proposal backlog evidence from successful
background executions remains `Draft`/`Queued` with no autonomous processing,
materialization, registry/load-path, usage, or optimization telemetry.

The single-proposal materialization command is necessary but not sufficient.
The missing capability is a service-owned operator that can drive the backlog
through processing and materialization under explicit policy.

## Goals / Non-Goals

Goals:

- Provide a Skill service command for an autonomous materialization cycle.
- Keep orchestration semantics inside the Skill service provider.
- Reuse existing processing and materialization Strategies.
- Return body-free, traceable, auditable aggregate evidence.

Non-Goals:

- Do not add application-specific materialization rules.
- Do not make Web, CLI, or the kernel own proposal eligibility.
- Do not claim activation, reuse, or optimization from materialization alone.
- Do not write executable scripts.

## Decisions

- Decision: implement the operator as a Director in `macaca-runtime-host`.
  - Reason: the runtime-host provider already owns local Skill service state and
    local Strategy composition.
- Decision: expose the operator as a typed `service.skill` command.
  - Reason: callers need a stable service boundary, not direct state access.
- Decision: require explicit apply-mode policy refs and package readiness.
  - Reason: autonomous mutation is privileged and must fail closed.
- Decision: use a provider-neutral package target resolver Strategy.
  - Reason: package roots must not be inferred from application names or business
    logic.

## Risks / Trade-Offs

- Risk: low-quality duplicate proposals could be materialized.
  - Mitigation: process proposals first, batch-limit, duplicate-suppress, and
    materialize only `ReadyForMaterialization` records.
- Risk: shell code could become the de facto owner.
  - Mitigation: shells may invoke or display operator results only through SDK
    commands and DTOs.
- Risk: materialization could be mistaken for optimization.
  - Mitigation: result names and operations output must keep P3 materialization
    separate from P4 activation/reuse and P5 optimization metrics.

## Migration Plan

1. Add DTOs, command constants, and descriptor capability.
2. Add runtime-host operator module with focused TDD coverage.
3. Add SDK and operations snapshot read paths.
4. Run live monitor again with a successful `/api/chat/v2` task and record P1-P5
   evidence.

## Open Questions

- Which package target resolver should be the default for application-scoped
  proposals: workspace-owned Skill packages, user-client Skills, or a future
  Store/package provider? The implementation must make this a Strategy so the
  default can change without rewriting eligibility logic.
