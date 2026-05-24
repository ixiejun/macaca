# Skill Proposal Materialization Lane Design

## Context

The live self-evolution report and the proposal-processing lane prove that
Macaca can capture real agent execution evidence, create governed Skill
experience proposals, score proposal quality, and mark eligible proposals as
`ReadyForMaterialization`. The remaining gap is the next service-owned step:
turning an eligible proposal into a bounded AgentSkills-compatible `SKILL.md`
draft with trace, policy, rollback, and governance evidence.

## Goals

- Add a Skill-service-owned materialization command after proposal processing.
- Require `ReadyForMaterialization`, trace, evidence refs, policy refs, package
  guard readiness, entitlement readiness, and ownership admission before writes.
- Use a Builder to convert provider-neutral proposal metadata into a bounded
  `SKILL.md` document without application-specific branches.
- Reuse the existing content-mutation Strategy for file writes, path checks,
  mementos, and sanitized mutation results.
- Promote the proposal into active governance metadata only after successful
  file materialization.
- Keep SDK, Web, CLI, frontend, and applications as adapters; they must not own
  eligibility, document construction, or file-write semantics.

## Non-Goals

- Do not automatically materialize every captured proposal.
- Do not generate executable scripts or mutate application-owned business files.
- Do not call LLM, MCP, semantic review, or remote providers in this slice.
- Do not hardcode application, workflow, provider, driver, gateway, chain,
  payment, model, or business-domain names.
- Do not return raw generated Skill bodies from snapshots, logs, or reports.

## Ownership Model

| Layer | Ownership |
| --- | --- |
| Skill service | Materialization command/result DTOs, validation, readiness contract, and sanitized audit shape. |
| Runtime host | Built-in local materialization Strategy, Builder orchestration, content-mutation delegation, and lifecycle promotion sequencing. |
| Existing mutation Strategy | Filesystem path policy, bounded writes, rollback memento creation, and sanitized mutation result. |
| SDK/SystemFacade | Future thin command forwarding and unavailable behavior. |
| Web/CLI/frontend | Future transport/display adapters only. |
| Store/EventLog | Future durable persistence for materialization refs and reports. |

## Design Patterns

- **Command**: `skill.evolution.materialization.apply` is a typed service
  command.
- **State**: the command is admitted only after a processing record reaches
  `ReadyForMaterialization`.
- **Builder**: `SkillDraftMaterializationBuilder` converts one proposal into a
  valid, bounded `SKILL.md` document.
- **Strategy**: runtime-host owns the local materialization Strategy and
  delegates the actual file mutation to the existing local content-mutation
  Strategy.
- **Specification**: readiness, policy, ownership, path, size, and sensitive
  text checks remain executable rules.
- **Memento**: content mutation returns rollback refs; materialization results
  expose refs only.
- **Observer**: materialization appends governance/telemetry facts after a
  successful write without taking over task execution.
- **Facade**: shells will call through SDK/focused clients instead of learning
  service internals.

## Flow

1. A real agent task creates a `SkillExperienceProposalRecord`.
2. Proposal processing marks the first high-quality duplicate-group proposal
   `ReadyForMaterialization`.
3. A caller submits `SkillProposalMaterializationCommand` with an approved
   package root and policy envelope.
4. The materializer verifies the proposal and processing record, builds bounded
   `SKILL.md` bytes, and either returns a preview or delegates to
   `SkillContentMutationCommand`.
5. On apply success, the provider promotes the proposal to active governance
   metadata and records sanitized evidence refs.
6. The result returns status, skill id, relative path, rollback ref, content
   digest, byte counts, policy refs, audit refs, and mutation state. It never
   returns the full generated body.

## Safety Rules

- Materialization must fail closed unless the proposal is Draft and processing
  state is `ReadyForMaterialization`.
- Apply mode must require evidence refs, policy decision refs, package readiness,
  entitlement readiness, and a non-protected or explicitly admitted ownership
  class.
- Generated content must stay bounded, UTF-8, AgentSkills-compatible, and free
  of obvious secret markers.
- Logs must include trace id, proposal id, skill id, status, bytes, rollback
  ref presence, policy/evidence counts, and mutation state; logs must not
  include the generated body.
- Dry-run preview may compute digest and planned bytes but must not write files,
  mutate proposal lifecycle, or create governance records.

## Verification

- OpenSpec validates in strict mode.
- `macaca-skill` tests cover command validation and generated result
  serialization without raw bodies.
- `macaca-runtime-host` tests prove dry-run does not write, apply writes
  `SKILL.md`, proposal lifecycle becomes promoted, governance snapshot includes
  an active record, and non-ready proposals are denied.
- Existing proposal-processing and content-mutation tests remain green.
- `git diff --check` and GitNexus change detection run before completion.
