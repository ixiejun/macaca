# Complete Self-Evolving Skill OS Design

## Context

`docs/macaca-agent-self-evolving-skills-research.md` frames skills as governed
procedural experience assets, not prompt fragments. Recent implementation
slices intentionally landed only low-risk pieces: in-memory governance
telemetry, deterministic dry-run, draft proposal creation and snapshot, alias
mapping, metadata-only lifecycle commands, and read-only operations UI. Those
slices created the service surface but not the durable, auditable operating
system needed for autonomous skill growth.

The next design must complete the remaining research without turning Macaca into
a Hermes-style filesystem script. Macaca is a microkernel Agent OS. Skill
evolution and curation belong in replaceable Skill service providers and
collaborating services, not the kernel, SDK, Web, CLI, frontend, or application
business code.

## Goals

- Provide a durable Store/EventLog-backed Skill Governance Store as the source
  of truth for lifecycle, provenance, telemetry, aliases, proposals, curation
  runs, reports, rollback refs, policy decisions, and audit event ids.
- Complete lifecycle semantics for `Draft`, `Active`, `Stale`, `Archived`,
  `Quarantined`, `Superseded`, and `Rejected`.
- Integrate verified task completion with draft-only experience extraction
  through Task, Evidence, Memory, Knowledge, Context, Policy, Store, and Skill
  service boundaries.
- Add policy-gated promotion, rejection, patch proposal, safe mutation,
  curation run, rollback, semantic review, umbrella merge, and support-file
  demotion flows.
- Make Context Composer, Scheduler, Task, and operations surfaces consume
  lifecycle and alias state through Skill service snapshots instead of direct
  filesystem or local semantic rules.
- Preserve provider neutrality, optional module absence, traceability,
  sanitized logs, replayable audit, and application-agnostic behavior.

## Non-Goals

- Do not make the kernel aware of skill curation, skill merge, similarity,
  LLM review, marketplace update, or filesystem mutation semantics.
- Do not let Web, CLI, or frontend classify stale skills, resolve merge
  policies, rewrite skill files, or own approval semantics.
- Do not auto-patch bundled, marketplace, application-owned, encrypted, paid,
  or cross-tenant skills.
- Do not make an LLM curator provider required for deterministic curation.
- Do not hardcode application names, workflow names, skill names, provider
  names, model names, driver names, gateway names, chain names, or business
  domains.

## Ownership Model

| Layer | Ownership |
| --- | --- |
| Kernel | Service identity, typed service call routing, trace/audit identity, policy facade, package guard primitives. |
| Skill service | Evolution, curation, lifecycle, alias, safe mutation contracts, governance snapshots, proposal status, run status, rollback orchestration. |
| Store/EventLog service | Durable append-only records, read models, memento refs, report artifact refs, audit event ids, replay cursors. |
| Task/Autonomy services | Verified terminal success events, bounded task summaries, evidence refs, curation wake scheduling through service calls. |
| Memory/Knowledge services | Classification destinations for facts and structured knowledge, not skill file ownership. |
| Context service | Compact skill catalog composition using Skill service snapshots and alias/lifecycle filters. |
| Entitlement/Policy services | Approval, package ownership, tenant, executable script, mutation, and marketplace restrictions. |
| Runtime host | Built-in provider adapters, provider factories, optional provider wiring, sanitized diagnostics. |
| SDK/SystemFacade | Provider-neutral client methods and Null Object behavior. |
| Web/CLI/frontend | Thin adapters for display, approval request submission, refresh, report download, and rollback command forwarding. |

## Design Patterns

- **Command**: every cross-boundary operation is a typed command/result:
  proposal, promotion, rejection, curation run, rollback, safe mutation, alias
  resolution, status, and snapshot.
- **Facade**: SDK exposes focused Skill Evolution and Skill Curation clients;
  shells call clients instead of constructing providers.
- **Strategy**: governance store, similarity provider, semantic reviewer,
  archive policy, merge eligibility, approval policy, and mutation applier are
  replaceable strategies.
- **Decorator**: trace, policy, resource, entitlement, package guard, metering,
  and sanitization wrap service calls before side effects.
- **State**: lifecycle transitions are explicit and validated by a state machine
  rather than free-form string updates.
- **Observer**: usage, view, activation, resource read, patch, promotion,
  rollback, alias, and curation events enter audit/event streams.
- **Memento**: curation runs create before/after snapshots, rollback refs,
  report refs, and replay cursors.
- **Specification**: admission, sensitive-content rejection, package ownership,
  trust-level eligibility, merge eligibility, executable-script policy, and
  context visibility are executable rules.
- **Abstract Factory**: runtime-host owns provider construction for built-in,
  plugin, remote, mock, unavailable, and future semantic-review providers.

## Capability Slices

### 1. Durable Governance Store

Introduce a provider-neutral Skill Governance Store interface backed by
Store/EventLog. The first built-in implementation may bridge existing in-memory
state into an event-log read model, but the public contract must be durable and
replayable. Governance records store sanitized metadata only:

- lifecycle state, pinned status, source scope, package ownership, trust level.
- provenance fields: skill id, version, author kind, author agent id,
  application id, session id, task id, trace id, evidence refs, created/updated
  timestamps.
- telemetry fields: view, activation, resource read, patch, success, failure,
  and last timestamp counters.
- aliases, proposal summaries, curation run refs, report refs, rollback refs,
  policy decision ids, audit event ids.

Snapshots must exclude full `SKILL.md` bodies and raw provider/task payloads.

### 2. Lifecycle State Machine

Extend the lifecycle model to include `Draft`, `Active`, `Stale`, `Archived`,
`Quarantined`, `Superseded`, and `Rejected`. Transitions are validated by a
Skill service state machine:

- `Draft -> Active` only through policy-gated promotion.
- `Draft -> Rejected` through rejection or policy denial.
- `Active -> Stale/Archived/Quarantined/Superseded` through curation decisions.
- `Archived -> Active` through restore.
- `Quarantined -> Active/Archived/Rejected` only through approval or policy.
- `Superseded` requires an alias or redirect record.

Pinned skills are protected from archive, deletion, supersede, and merge apply
unless a future explicit approval override allows it.

### 3. Task Completion And Experience Extraction

Task/Autonomy services emit verified terminal success events with bounded
summaries and evidence refs. The Skill Evolution service classifies each
candidate into one of several generic destinations:

- memory fact.
- knowledge digest.
- existing skill patch proposal.
- new skill draft.
- support-file draft.
- discard/no-op with rationale.

The classification must be provider-neutral and application-agnostic. It may
use optional semantic providers, but deterministic fallback must remain
available and explicit.

### 4. Proposal Lifecycle And Safe Mutation

Add commands for:

- `skill.evolution.propose_patch`
- `skill.evolution.promote_draft`
- `skill.evolution.reject_draft`
- safe skill content mutation through service-owned patch/write/remove
  contracts.

Promotion and mutation must run trace, policy, package guard, entitlement,
sensitive-content scan, file boundary validation, atomic write, and audit
decorators before writing anything. Mutation commands may operate only on
allowed skill package paths such as `SKILL.md`, `references/`, `templates/`,
`scripts/`, and `assets/`; executable script changes require stronger policy.

### 5. Curation Runner And Rollback

Add curation status/run/rollback/snapshot commands. A run has:

- run id, trace id, provider id, dry-run flag, candidate counts.
- deterministic phase result.
- optional semantic review result.
- planned actions and policy decisions.
- before/after snapshot refs.
- report refs (`run.json` and `REPORT.md` or equivalent store artifacts).
- rollback ref and audit event ids.

Dry-run must never mutate governance state or files. Apply mode executes only
approved actions and writes rollback mementos before side effects.

### 6. Semantic Review Provider

Semantic review is optional. Provider absence returns structured unavailable for
semantic analysis while deterministic curation still runs. A semantic provider
may produce typed proposals for similarity, clustering, duplicate detection,
failure/success correlation, support-file demotion, or umbrella merge. It must
not directly mutate active skills.

### 7. Umbrella Merge And Support-File Demotion

Merge proposals must preserve class-level skills while avoiding skill explosion:

- reusable generic flows remain in `SKILL.md`.
- session-specific details move to `references/`.
- starter artifacts move to `templates/`.
- repeatable actions move to `scripts/`.
- absorbed skills become `Superseded` with alias/redirect records.

Merge eligibility must reject incompatible scope, ownership, permissions, trust
level, package source, executable semantics, or tenant boundaries.

### 8. Context Composer And Alias Integration

Context Composer consumes Skill service snapshots only. It must:

- include active skills in normal catalogs.
- exclude or annotate draft, stale, archived, quarantined, rejected, and
  superseded skills according to explicit profile rules.
- resolve aliases through the Skill service.
- emit context reports with visible count, filtered count, filter reasons,
  alias resolutions, and skill activation/read trace refs.

Task and Scheduler consumers must resolve skill references through the Skill
service rather than rewriting historical references by default.

### 9. Package, Store, And Entitlement Rules

Skill self-evolution respects package ownership:

- bundled skills are not auto-patched or auto-archived by agents.
- marketplace skills may receive local overlay/draft proposals but not upstream
  mutation.
- application-owned skills require application-scope policy and package guard.
- agent-private skills may evolve under agent/tenant policy.
- central user/tenant skills require stronger approval.
- paid or encrypted skills can expose metadata and aliases only unless
  entitlement grants mutation.

### 10. Operations Surfaces

Web/CLI/frontend stay thin. They may display governance state, curation reports,
proposal status, approvals, rollback refs, and bounded diagnostics. They may
submit approval or command requests through SDK clients. They must not classify
skills, merge content, apply lifecycle rules, or write files locally.

## Logging, Trace, And Audit

Every command logs key execution nodes with sanitized fields:

- command name, trace id, scope, actor kind, target skill id, lifecycle action,
  proposal id, run id, policy decision id, mutation kind, result state, denied
  reason, and bounded counts.
- logs must not include raw prompts, raw provider payloads, raw task output,
  secrets, package bytes, manifests, full skill bodies, credentials, or
  unbounded diagnostics.
- audit events are replayable and linked to Store/EventLog records.

## Migration Plan

1. Add durable governance store contracts and replayable read models without
   changing active skill behavior.
2. Migrate existing in-memory lifecycle, alias, proposal, and telemetry state to
   the governance store through compatibility adapters.
3. Add lifecycle state machine and richer provenance/telemetry fields.
4. Wire Task/Autonomy verified terminal success to draft-only evolution.
5. Add proposal promotion/rejection and safe mutation as policy-gated commands.
6. Add curation run/status/rollback with deterministic phase first.
7. Add optional semantic provider and umbrella merge proposals.
8. Wire Context Composer, Task, and Scheduler to alias/lifecycle snapshots.
9. Add shell approval mutations through SDK routes and UI controls.
10. Strengthen boundary, rollback, sanitization, and optional-provider gates.

## Risks And Mitigations

- Risk: skill mutation corrupts active packages. Mitigation: atomic writes,
  package guards, rollback mementos, policy approval, and mutation tests.
- Risk: LLM curator becomes a black box. Mitigation: typed proposals only,
  deterministic fallback, no direct mutation, sanitized bounded outputs.
- Risk: shell code grows semantics. Mitigation: route/UI tests and dependency
  gates prove Web/CLI/frontend call facades only.
- Risk: governance state diverges from files. Mitigation: event-log replay,
  reconciliation snapshots, before/after mementos, and explicit conflict states.
- Risk: curation removes useful skills. Mitigation: pinned protection,
  dry-run-first, approval gates, restore/rollback, and alias redirects.

## Open Questions

- Which Store/EventLog provider should be the first durable backend for local
  development and CI?
- Should active skill file mutation be enabled only after curation run rollback
  is fully proven, or can draft-only file materialization land earlier?
- Which approval policy level is required for central user/tenant skills versus
  agent-private skills?
