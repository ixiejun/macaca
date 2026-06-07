# Change: Add Skill Autonomous Materialization Operator

## Why

The platform now has proposal capture, proposal processing, and a single-proposal
materialization command, but live monitoring still shows no autonomous bridge
from captured proposal backlog into governed `SKILL.md` materialization. Without
that bridge, self-evolution remains capture plus manual/service capability, not
a closed loop.

## What Changes

- Add a provider-neutral Skill service command that runs a governed autonomous
  materialization cycle.
- Compose existing proposal processing and proposal materialization Strategies
  instead of duplicating eligibility or file-write behavior.
- Support dry-run and apply modes with batch limits, policy refs, evidence refs,
  entitlement readiness, package guard readiness, rollback refs, and sanitized
  aggregate results.
- Expose body-free operator evidence through operations snapshots so monitoring
  can distinguish proposal capture, processing, materialization, activation, and
  later optimization.

## Impact

- Affected specs: `skill-governance-curation`
- Affected crates: `macaca-skill`, `macaca-runtime-host`, `macaca-sdk`,
  `macaca-web`
- Boundary impact: Skill service/runtime-host own semantics; Web/CLI remain thin
  adapters; kernel remains uninvolved.
