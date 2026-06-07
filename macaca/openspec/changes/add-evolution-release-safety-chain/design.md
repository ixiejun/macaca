## Context

`add-autonomy-evolution-control-plane`,
`add-evolution-admission-quality-gates`, and
`add-normalized-evolution-benchmarking` established the control-plane state
machine, admission Specifications, and paired benchmark Strategy. This change
adds the release safety chain that decides whether an admitted and benchmarked
candidate may move through quarantine, canary, promotion, active monitoring,
rollback, supersedence, or rejection.

## Goals

- Keep release safety semantics inside the Autonomy Evolution service boundary.
- Make all release actions typed, traceable, policy-gated, and replayable.
- Use rollback mementos as bounded references, not embedded target state.
- Preserve application neutrality: no app names, workflow names, provider names,
  driver names, or business rules in OS-layer code.
- Provide Null Object behavior when the service is absent.

## Non-Goals

- This change does not mutate Skill packages, application packages, or Macaca
  source code.
- This change does not implement a production Store/EventLog governance ledger;
  it prepares replayable refs for the later ledger slice.
- This change does not add Web, CLI, or frontend semantics.

## Decisions

- **Command pattern:** Add `EvolutionReleaseCommand` and
  `EvolutionReleaseResult` as the cross-boundary release surface.
- **Strategy pattern:** Add `EvolutionReleaseSafetyStrategy` with a default
  conservative implementation. Future providers can replace it without changing
  SDK or runtime-host callers.
- **Specification pattern:** Model each safety condition as a structured
  finding with a reason code and bounded evidence refs.
- **Memento pattern:** Represent rollback state through replayable
  `EvolutionRollbackMemento` refs. The service validates their presence and
  scope but does not store target bytes.
- **State pattern:** Release actions map to explicit lifecycle states:
  `Quarantined`, `CanaryRunning`, `Promoted`, `ActiveMonitoring`, `RolledBack`,
  `Superseded`, `Rejected`, and `Inconclusive`.
- **Facade/Adapter patterns:** SDK remains a thin facade and runtime-host remains
  a command adapter. Neither owns release semantics.

## Policy Inputs

The release gate evaluates provider-neutral metadata:

- capability diff summary and diff evidence refs
- package ownership refs
- tenant/application/session/task scope
- trust level
- resource permission refs
- executable change flag
- blast-radius score
- benchmark decision refs
- rollback memento refs

All refs are bounded strings. Raw package bodies, manifests, prompts, provider
payloads, credentials, signatures, and unbounded output are excluded.

## Risks And Mitigations

- **Risk:** A canary pass could be treated as proof of broad production safety.
  **Mitigation:** The result records only the requested release action and
  requires separate promotion/monitoring commands with policy refs.
- **Risk:** Rollback could fake success without replayable target state.
  **Mitigation:** Rollback requires at least one scoped rollback memento ref and
  fails closed when mementos are missing.
- **Risk:** Blast-radius thresholds become application-specific.
  **Mitigation:** The default threshold is numeric and provider-neutral; stricter
  per-tenant policy can be introduced later by replacing the Strategy.

## Verification

- Service tests cover dry-run, canary pass, canary fail, rollback from canary
  failure, missing rollback memento denial, and executable high-blast-radius
  denial.
- SDK tests cover unavailable release behavior.
- Runtime-host tests cover command decoding.
- OpenSpec strict validation, targeted Rust tests, `git diff --check`, and
  GitNexus detect-changes run before commit.
