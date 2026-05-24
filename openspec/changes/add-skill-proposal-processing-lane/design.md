## Context

The current self-evolution path has a working Agent Execution observer and a
working Skill proposal store.  Repeated live runs show proposal capture, but no
closed-loop Skill optimization.  The most immediate gap is a missing service
lane between Draft proposal capture and future package materialization.

## Goals

- Process Draft proposals through a Skill-service-owned state machine.
- Add deterministic quality and duplicate signals that can run without optional
  semantic review providers.
- Provide backlog health and processing snapshots for app-scoped operations.
- Make future materialization safer by requiring an explicit
  `ReadyForMaterialization` state before any writer mutates Skill packages.
- Preserve sanitized trace, evidence, policy, and audit refs.

## Non-Goals

- Do not write or patch Skill package files.
- Do not activate, invoke, or register newly evolved Skills.
- Do not score optimization improvements for later task runs.
- Do not add application-specific task families or business rules.
- Do not require LLM, MCP, semantic search, or remote telemetry providers.

## Decisions

- Decision: add processing to `service.skill`.
  - Reason: proposal quality, lifecycle pressure, and materialization readiness
    are Skill governance semantics and must not live in Web, CLI, kernel, or
    application code.
- Decision: use a deterministic Strategy first.
  - Reason: the live failure is backlog pressure, not semantic intelligence.
    Optional semantic review can later decorate or replace the Strategy.
- Decision: keep materialization out of this slice.
  - Reason: Runs 20-30 show duplicate Draft noise.  The platform needs
    quality and suppression before safe autonomous writing.
- Decision: store processing records separately from proposal records.
  - Reason: the lane can be replayed, reset, or backed by Store/EventLog later
    without mutating the original proposal evidence.

## Design Patterns

- **Command**: `skill.evolution.processing.run` and
  `skill.evolution.processing.snapshot` are typed service commands.
- **State**: processing records use explicit states: `Queued`, `Reviewing`,
  `ReadyForMaterialization`, `SuppressedDuplicate`, and `Rejected`.
- **Strategy**: the built-in deterministic scorer is replaceable by future
  semantic or policy providers.
- **Specification**: score thresholds and duplicate signatures are executable
  rules.
- **Observer**: processing observes stored proposal metadata and does not own
  Agent Execution.
- **Memento**: run ids, snapshot refs, and before/after counts prepare the lane
  for durable rollback.
- **Facade**: SDK methods hide runtime-host provider details from shells.

## Data Model

Processing records contain bounded metadata only:

- proposal id, trace id, task id, optional application/session scope.
- proposal lifecycle and processing state.
- quality score, score reasons, duplicate signature, duplicate group size.
- decision reason, evidence ids, policy decision refs, audit event ids.
- created and updated timestamps.

The model must not contain raw prompts, raw provider payloads, full task output,
full Skill bodies, manifests, package bytes, credentials, secrets, private keys,
raw signatures, or unbounded diagnostics.

## Processing Rules

- Dry-run computes processing records and counters without mutation.
- Apply mode requires trace, evidence refs, and policy decision refs.
- The first high-enough-quality proposal in a duplicate group may become
  `ReadyForMaterialization`.
- Repeated low-information proposals in the same duplicate group become
  `SuppressedDuplicate`.
- Proposals with missing required evidence or policy denial become `Rejected`.
- Existing proposal lifecycle remains unchanged in this slice.
- Skill package files and active catalog state remain unchanged.

## Observability

Every run logs trace id, app/session scope when available, proposal count,
duplicate group count, ready count, suppressed count, rejected count, evidence
ref count, policy decision count, and mutation status.  Snapshots expose the
same counts plus processing records so operations surfaces can show backlog
pressure without reimplementing rules.  Snapshot reads also synthesize
metadata-only `Queued` records for proposals that have not yet been processed;
those records are read-only observability views and must not mutate proposal
lifecycle or Skill package state.

## Risks And Mitigations

- Risk: operators treat `ReadyForMaterialization` as a completed Skill.
  Mitigation: docs, DTO comments, and logs state that readiness is not package
  materialization, activation, reuse, or optimization.
- Risk: deterministic duplicate grouping is too coarse.
  Mitigation: retain all suppressed proposals with evidence refs and group size;
  future Strategies can refine grouping without shell changes.
- Risk: in-memory processing state is lost on restart.
  Mitigation: expose provider limitations and shape run/snapshot refs for a
  future Store/EventLog provider.
