# Change: Add Self-Evolution Evaluation Harness

## Why

Macaca can now govern skill self-evolution, but operators still need a generic,
auditable way to prove that evolution happened and that it improves later real
tasks. Without a measurement harness, self-evolution can degrade into generated
artifacts that look plausible but are not reused, not safer, and not better.

## What Changes

- Add a self-evolution evaluation contract that records the white-box chain from
  verified task completion through later skill activation.
- Add generic black-box baseline/evolved metrics for real task families.
- Require sanitized report refs, rollback refs, policy decisions, audit event
  ids, and pass/fail scoring.
- Keep evaluation generic: no application-specific workflow, provider, model,
  driver, gateway, chain, or business-domain branches.

## Impact

- Affected specs: `skill-governance-curation`
- Affected code: Skill governance/evaluation DTOs, scoring helpers, runtime-host
  provider wiring, SDK facade, shell report adapters, and targeted tests.
