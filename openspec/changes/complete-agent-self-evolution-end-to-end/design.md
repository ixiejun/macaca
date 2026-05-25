## Context

The autonomy evolution service now owns lifecycle transitions, admission gates, benchmark scoring, release safety, governance ledger records, OS-code proposal metadata, and live tick checkpoints. The missing production shape is an execution bridge that runs those parts as one unattended loop and dispatches target-specific actions through service boundaries.

## Goals

- Run one end-to-end closure command from runtime-host without application-specific logic.
- Keep target mutation semantics behind service-owned adapters.
- Return a replayable result containing live tick, target execution, and audit evidence.
- Fail closed when a required target adapter command is missing or a provider is unavailable.

## Non-Goals

- Directly applying arbitrary Macaca source-code patches in the default provider.
- Moving Skill package mutation logic into autonomy evolution.
- Adding shell-owned semantics or application-specific workflow routing.

## Design

The implementation uses a Bridge/Adapter composition:

- `AutonomyEvolutionService` remains the owner of evolution semantics and gets a new `evaluate_os_code_proposal` command for the existing OS-code adapter Strategy.
- `runtime-host` owns `AutonomyEvolutionLiveExecutor`, a host-side Bridge that calls service commands in order:
  1. `autonomy.evolution.live.tick`
  2. target adapter command, based only on `EvolutionTargetType`
  3. `autonomy.evolution.live.audit`
- `SkillPackage` targets dispatch to `skill.evolution.materialization.operator.run`.
- `OsCodeProposal` targets dispatch to `autonomy.evolution.os_code.proposal.evaluate`.
- Other targets fail closed with structured unavailable diagnostics until their adapters exist.

The bridge records bounded DTOs only. It does not inspect raw prompts, generated Skill bodies, application manifests, provider payloads, or source patch bytes.

## Risks And Mitigations

- Risk: treating a live tick as success before the target adapter runs.
  Mitigation: the execution result has an independent target outcome and is accepted only when both live tick and target execution pass.
- Risk: leaking target payloads through audit.
  Mitigation: only command names, refs, decisions, and bounded reason codes are returned.
- Risk: OS-code evolution being mistaken for direct self-modification.
  Mitigation: default OS-code command remains proposal-only and records `source_mutation_performed = false`.
