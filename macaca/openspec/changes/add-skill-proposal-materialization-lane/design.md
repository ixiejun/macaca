## Context

The proposal-processing lane intentionally does not materialize skills. It
creates an auditable readiness gate so a separate command can perform
policy-gated package mutation. This change adds that next command without
moving ownership into Web, CLI, frontend, application code, or the kernel.

## Goals

- Convert a ready proposal into a bounded `SKILL.md` draft.
- Preserve trace, policy, evidence, rollback, and governance refs.
- Reuse existing safe content mutation and proposal lifecycle commands.
- Expose sanitized materialization results without returning generated bodies.
- Keep the implementation generic for all applications and task families.

## Non-Goals

- Do not automatically run materialization after every proposal.
- Do not activate or select the new Skill in later tasks in this slice.
- Do not add application-specific task-family logic.
- Do not write scripts, assets, manifests, package bytes, or provider payloads.
- Do not expose raw generated Skill bodies in logs, snapshots, or reports.

## Decisions

- Decision: add materialization as a separate Skill service command.
  - Reason: writing `SKILL.md` is a privileged Skill package mutation and must
    stay behind service policy, trace, and rollback boundaries.
- Decision: require `ReadyForMaterialization` from proposal processing.
  - Reason: raw Draft proposals are too noisy to write autonomously.
- Decision: use a Builder for `SKILL.md` content.
  - Reason: content construction is a separate concern from policy checks and
    filesystem mutation.
- Decision: delegate file writes to `SkillContentMutationCommand`.
  - Reason: path allowlists, ownership policy, sensitive text checks, and
    memento creation already live there.

## Design Patterns

- **Command**: materialization is `skill.evolution.materialization.apply`.
- **State**: Draft proposal plus `ReadyForMaterialization` processing record is
  the only admitted source state.
- **Builder**: a bounded document builder creates AgentSkills-compatible
  frontmatter and instructions.
- **Strategy**: runtime-host provides the built-in local Strategy.
- **Specification**: readiness, policy, ownership, and content checks remain
  executable validations.
- **Memento**: the mutation Strategy returns rollback refs.
- **Observer**: successful materialization records governance telemetry after
  the file write.

## Data Model

The command includes trace, scope, proposal id, target package root, ownership,
dry-run flag, rationale, evidence refs, policy decision refs, audit refs, and
policy hints. The result includes status, proposal id, skill id, relative path,
content digest, planned/written bytes, rollback ref, mutation refs, evidence
refs, policy refs, audit refs, and timestamps.

The result must not contain raw prompts, provider payloads, unbounded task
output, full generated `SKILL.md` bodies, manifests, credentials, secrets,
package bytes, or executable scripts.

## Risks And Mitigations

- Risk: non-ready proposal writes a low-value Skill.
  Mitigation: fail closed unless processing state is `ReadyForMaterialization`.
- Risk: file side effect succeeds while governance promotion fails.
  Mitigation: pre-check proposal state, write through mutation memento, then
  promote immediately with the same evidence/policy refs.
- Risk: generated content leaks sensitive data.
  Mitigation: reuse mutation sensitive-text validation and keep result/logs
  body-free.
- Risk: callers confuse materialization with optimization.
  Mitigation: docs and DTO comments state that later activation/reuse metrics
  remain separate evidence.
