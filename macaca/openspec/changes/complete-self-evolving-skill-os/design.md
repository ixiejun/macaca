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

## Implementation Design Notes

### Slice 2A: Governance Event Records And Replay Read Model

- Patterns: Command DTOs remain the service boundary; Strategy is represented
  by provider-neutral Store/EventLog event records that the built-in local
  provider can append today and a durable provider can persist later; Observer
  is used for append-only governance events; Memento is represented by snapshot
  and rollback reference records; Specification remains reserved for later
  policy/package eligibility rules.
- Ownership: `macaca-skill` owns the provider-neutral governance event and
  read-model contracts. `macaca-runtime-host` owns the built-in local append
  strategy beside the existing compatibility state. SDK, Web, CLI, frontend,
  applications, and kernel do not gain governance semantics.
- Safety: replay events store only identifiers, counters, refs, bounded
  metadata, policy decision ids, audit event ids, and timestamps. They do not
  accept raw prompts, raw task output, provider payloads, manifests, package
  bytes, secrets, credentials, signatures, or full `SKILL.md` bodies.

### Slice 4A: Provenance Event Capture For Existing Governance Events

- Patterns: Observer remains the append-only governance event stream; Memento is
  extended with derived provenance events that replay alongside lifecycle,
  alias, proposal, curation, snapshot, and rollback records; Command DTOs stay
  unchanged for callers, so shells and SDK clients do not learn provenance
  semantics; Strategy remains open because future Store/EventLog providers can
  persist the same derived provenance events without changing consumers.
- Ownership: `macaca-skill` owns sanitized provenance event DTOs and replay
  projection. `macaca-runtime-host` only logs and stores the derived records in
  the local compatibility adapter. Kernel, SDK, Web, CLI, frontend, and
  applications do not classify or mutate provenance.
- Safety: provenance events copy only skill ids, optional app/session/task/agent
  refs, trace ids, evidence refs, policy/audit refs, action labels, and bounded
  target refs. They deliberately exclude raw prompts, task output, provider
  payloads, manifests, package bytes, credentials, signatures, and full skill
  bodies.

### Slice 4B: Optional Scope References In Provenance

- Patterns: Command DTOs carry optional app/session/tenant/task references as
  metadata; Observer projects those refs into provenance events; Facade remains
  unchanged because shells still submit typed service commands instead of
  interpreting governance semantics.
- Ownership: `macaca-skill` owns the scope/provenance DTOs and keeps tenant
  identity as an audit/routing ref. `macaca-runtime-host` copies lifecycle task
  refs and additional evidence refs into the local compatibility event only.
  No kernel, SDK, Web, CLI, frontend, or application-specific behavior is
  introduced.
- Safety: optional refs are ids only. The slice does not add raw prompt, raw
  task output, provider payload, manifest, package bytes, credential, signature,
  or full skill body fields to telemetry, provenance, snapshots, or logs.

### Slice 4C: Metadata Sanitization At Governance Event Boundaries

- Patterns: Decorator-style sanitization runs inside the governance event and
  proposal constructors before Store/EventLog replay can observe metadata;
  Observer remains append-only, but it only receives bounded id/ref metadata.
- Ownership: `macaca-skill` owns the prompt-safe metadata rules because they are
  part of the Skill service contract. Runtime-host providers call the same DTO
  constructors and do not maintain separate sanitization semantics.
- Safety: metadata keys containing raw prompt, task output, provider payload,
  package bytes, manifest body, skill body, secrets, credentials, or signatures
  are dropped, and retained reference values are bounded before serialization.

### Slice 4D: Bounded Telemetry Aggregates And Generic Effectiveness Signals

- Patterns: Observer continues to collect sanitized usage events, while a small
  Facade-friendly aggregate DTO exposes bounded counters on snapshot and status
  commands. The deterministic curation Specification consumes only generic
  success/failure counters and never branches on application, workflow, or
  domain names.
- Ownership: `macaca-skill` owns `SkillTelemetryAggregate` and deterministic
  curation rules. `macaca-runtime-host` only computes the aggregate from its
  local governance read model and passes it through typed service results. SDK,
  Web, CLI, frontend, kernel, and applications do not compute curation
  rationale.
- Safety: aggregates contain counters only. They do not expose prompts, task
  output, provider payloads, manifests, package bytes, secrets, credentials, or
  full skill bodies.

### Slice 5A: Verified Task Experience Candidate Contract

- Patterns: Command remains the service integration point from Task/Autonomy to
  Skill Evolution; Specification is used for explicit terminal-success,
  evidence-gate, bounded-size, and generic-destination validation; Observer
  logs candidate accepted/rejected and proposal-created events without
  executing mutations.
- Ownership: `macaca-skill` owns the provider-neutral candidate DTO,
  destination enum, and validation contract. `macaca-runtime-host` only enforces
  the contract at the service boundary and records sanitized logs. Task,
  Memory, Knowledge, Context, SDK, Web, CLI, frontend, kernel, and
  applications do not write skill files or infer skill curation semantics in
  this slice.
- Safety: the candidate stores bounded summaries, trace digest refs, memory
  digest refs, evidence refs, destination labels, and proposal metadata only.
  Unverified terminal tasks, rejected/missing evidence gates, missing evidence,
  and oversize summaries are rejected before proposal creation. Draft proposals
  remain non-mutating for active catalogs and package files.

### Slice 5B: Memory And Knowledge Destination Routing

- Patterns: Facade routes `MemoryFact` and `KnowledgeDigest` destinations to
  the Memory runtime facade while Command keeps Skill Evolution inputs and
  outputs typed; Strategy is preserved because runtime-host injects an optional
  replaceable Memory runtime; Observer logs routed, unavailable, and failed
  destination outcomes with bounded refs only.
- Ownership: Skill Evolution still owns reusable experience classification and
  governance proposals. Memory owns fact persistence. The Memory knowledge
  compiler owns knowledge digest compilation. Runtime-host performs provider
  wiring at the composition root. Web only passes the existing runtime facade
  into the Skill service provider and does not implement classification,
  lifecycle, memory, or knowledge semantics.
- Safety: routed payloads use bounded summaries, trace ids, task ids, evidence
  refs, trace digest refs, and memory digest refs. Route results expose
  synthetic `memory://` or `knowledge://candidate/` refs and structured
  skipped/unavailable/failed states. No raw prompts, raw task outputs, provider
  payloads, manifests, package bytes, credentials, signatures, or full skill
  bodies enter route results or logs.

### Slice 7A: Safe Skill Content Mutation Foundation

- Patterns: Command DTOs describe create, patch, support-file write/remove,
  archive materialization, and restore materialization without shell-owned
  semantics; Strategy is the runtime-host local mutation applier that can later
  be replaced by Store/Package providers; Decorator-style validation performs
  trace, policy, entitlement, package guard, path, size, encoding, executable,
  and sensitive-content checks before side effects; Specification captures
  allowed paths and protected ownership classes; Memento records rollback refs
  before atomic writes or removals.
- Ownership: `macaca-skill` owns provider-neutral mutation contracts and
  validation rules. `macaca-runtime-host` owns the built-in local filesystem
  Strategy and sanitized logs. Kernel, SDK, Web, CLI, frontend, applications,
  and Context/Task consumers do not write skill files or infer mutation policy
  in this slice.
- Safety: mutation results expose only refs, bounded counts, status, and
  sanitized denial reasons. Commands may carry content because the Skill
  service is the mutation boundary, but logs, reports, snapshots, and results
  never echo raw content, prompts, provider payloads, manifests, package bytes,
  credentials, signatures, or full skill bodies.

### Slice 8A: Read-Only Curation Status

- Patterns: Command exposes `skill.curation.status` as a typed read-only
  status request; Facade compatibility is preserved because shells can render
  provider, interval, idle, last-run, next-run, and unavailable fields without
  owning curation scheduling; Observer reads the append-only governance event
  stream for last-run refs; Strategy remains the local provider state and can
  later be replaced by Store/EventLog-backed status.
- Ownership: `macaca-skill` owns the status DTO. `macaca-runtime-host` owns
  local status projection from provider state. Scheduler/runtime policy owns
  cadence input, and shells only display the returned DTO.
- Safety: status is non-mutating and does not inspect skill bodies, package
  bytes, raw provider payloads, prompts, or task output.

### Slice 8B: Governed Curation Run Command

- Patterns: Command exposes `skill.curation.run` as the single typed entrypoint
  for dry-run and approval-gated apply runs; Facade adds SDK routing without
  shell semantics; Strategy keeps the built-in deterministic local runner
  replaceable; Observer records bounded run events for status and replay;
  Memento fields are modeled as report, snapshot, and rollback refs before the
  later snapshot/rollback implementation slices attach concrete artifacts.
- Ownership: `macaca-skill` owns the provider-neutral curation command/result
  DTOs and `SkillCurationRunRecord`. `macaca-runtime-host` owns the local
  Strategy and sanitized logs. SDK forwards commands through the Skill service
  facade. Web, CLI, frontend, kernel, and applications do not classify,
  archive, merge, alias, or mutate skills in this slice.
- Safety: dry-run and the first approval-gated apply path produce bounded
  recommendations and run refs only; they do not touch skill package files,
  aliases, scheduler refs, context snapshots, raw provider payloads, prompts,
  manifests, package bytes, credentials, signatures, or full skill bodies.

### Slice 8C: Curation Snapshot Command

- Patterns: Command exposes `skill.curation.snapshot` as a typed ref-producing
  snapshot operation; Facade routes the command through SDK; Strategy records a
  local governance snapshot ref that future Store/EventLog providers can
  replace; Observer appends a snapshot-ref event for replay.
- Ownership: `macaca-skill` owns snapshot command/result DTOs and snapshot ref
  contracts. `macaca-runtime-host` owns the built-in local projection from
  governance records and event-log read models. Shells receive refs and counts
  only and do not interpret lifecycle, rollback, or package state.
- Safety: snapshot responses include governance counts, curation run ids,
  rollback refs, and optional package memento refs only. They never embed
  package bytes, manifests, raw provider payloads, prompts, credentials,
  signatures, or full skill bodies.

### Slice 8D: Curation Rollback Command

- Patterns: Command exposes `skill.curation.rollback` as the single typed
  rollback entrypoint; Memento restores the local governance read state from a
  pre-apply snapshot; Observer appends a bounded rollback event after restore so
  replay remains auditable; Facade keeps SDK/Web/CLI/frontend as command
  forwarders rather than rollback semantic owners.
- Ownership: `macaca-skill` owns rollback command/result DTOs and validation
  rules. `macaca-runtime-host` owns the built-in local rollback Strategy and
  in-memory memento projection until a Store/EventLog provider supplies durable
  artifacts. Kernel, SDK, Web, CLI, frontend, Context, Task, and applications do
  not reconstruct lifecycle, telemetry, alias, report, or package refs.
- Safety: rollback requires trace, rollback ref, approval refs, and policy
  decision refs. Results expose restored counts and refs only; logs and DTOs do
  not include raw prompts, raw task output, provider payloads, manifests,
  package bytes, credentials, signatures, or full skill bodies.

### Slice 8E: Bounded Curation Report Refs

- Patterns: Memento and Observer records now carry separate bounded refs for
  `run.json` and `REPORT.md`, while Command results expose those refs without
  embedding artifact bodies.
- Ownership: `macaca-skill` owns the provider-neutral ref fields on the curation
  run contract. `macaca-runtime-host` creates local Store-style refs in the
  built-in Strategy. Durable Store/EventLog providers can replace the ref
  backing without changing SDK or shell callers.
- Safety: report refs are identifiers only. They do not contain raw report
  bodies, provider payloads, prompts, manifests, package bytes, credentials,
  signatures, or full skill instructions.

### Slice 8F: Dry-Run Immutability Gate

- Patterns: Specification is represented by a regression test proving dry-run
  curation is recommendation-only for active governance and alias read models;
  Observer may still append run evidence for audit.
- Ownership: `macaca-runtime-host` verifies the built-in local Strategy keeps
  governance records, alias maps, rollback refs, and package memento refs
  unchanged during dry-run. Scheduler and Context refs are not owned by this
  provider and remain untouched by construction.
- Safety: the test asserts no rollback/package memento refs are produced by
  dry-run, so active package files, scheduler refs, and context snapshots are
  not implied or faked.

### Slice 8G: Deterministic Curation Phases

- Patterns: Specification owns deterministic phase evaluation inside
  `macaca-skill`, while runtime-host remains a Strategy executor that dispatches
  typed service commands.  Governance observations keep only allowlisted
  diagnostics, not raw provider metadata.
- Phases: the deterministic report now covers protected, quarantine, size,
  invalid metadata, missing dependency, stale, archive, consolidation, and keep
  phases.  A recommendation may include multiple phases, while its action is
  the highest-priority non-destructive plan for operators and future apply
  policies.
- Safety: package size, metadata validity, missing dependency, and quarantine
  hints are stored as bounded, sanitized governance diagnostics.  Reports still
  contain refs, counts, rationales, and evidence ids only; skill bodies, package
  bytes, prompts, provider payloads, and application semantics stay out of the
  Skill service contract.

### Slice 8H: Curation Run Structured Logs

- Patterns: Observer is applied at runtime-host Strategy boundaries.  The local
  provider logs run start, deterministic phase input, deterministic phase
  completion, rollback ref recording, and final run recording without moving
  curation semantics into shell or presentation layers.
- Audit fields: structured logs include trace id, run id, dry-run flag,
  threshold inputs, candidate counts, phase counts, policy decision ref counts,
  audit event id counts, report refs, rollback refs, and mutation status.
- Safety: logs aggregate phase counts only.  They never print skill bodies,
  package bytes, prompts, raw provider payloads, unbounded metadata, or
  application-specific workflow names.

### Slice 8I: Curation Report Test Gate

- Coverage: runtime-host curation tests now cover status, dry-run
  immutability, approval-gated apply rejection, rollback restore, pinned
  protection, protected ownership, absent semantic provider behavior, and
  bounded report refs.
- Safety: the added report test verifies protected ownership produces a
  protected recommendation, semantic review is explicitly unavailable, report
  refs are store refs only, rollback is absent for dry-run, and no mutation is
  reported.

### Slice 9A: Optional Semantic Review Null Strategy

- Patterns: Strategy is used for the optional semantic review provider;
  Command-style DTOs carry `SkillSemanticReviewRequest` and
  `SkillSemanticReviewResult`; Null Object is represented by the unavailable
  provider so deterministic curation keeps running when no semantic provider is
  configured.
- Ownership: `macaca-skill` owns the provider-neutral semantic review contract
  and typed proposal result. `macaca-runtime-host` records the default
  unavailable result at the curation boundary until the later provider factory
  wiring slice lands. SDK, Web, CLI, frontend, applications, and kernel do not
  construct or own semantic review providers.
- Safety: the unavailable provider records absence as structured metadata only.
  The curation report carries typed proposals, diagnostics, status, provider id,
  and mutation=false; it never stores raw prompts, provider payloads, secrets,
  package bytes, manifests, or full skill bodies.

### Slice 9B: Semantic Review Input And Proposal Validation

- Patterns: Command-style DTOs now include optional sanitized inputs for
  similarity, clustering, duplicate detection, success/failure correlation, and
  reference graph review. Specification-style validation rejects direct
  mutation and malformed typed proposals before future providers can affect
  policy-gated apply flows.
- Ownership: `macaca-skill` owns the input and proposal contracts. Runtime-host
  currently supplies empty optional inputs for the unavailable provider; richer
  provider factories remain a later runtime-host slice.
- Safety: all semantic inputs carry ids, counts, score hints, support-file refs,
  and evidence refs only. Provider outputs are typed proposals with bounded
  rationale and confidence, and `mutated=true` is explicitly invalid.

### Slice 9C: Semantic Review Bounds And Error Sanitization

- Patterns: Decorator-style contracts are represented by provider budget,
  resource-limit, and sanitization-policy DTOs on the semantic review request.
  Specification validation bounds proposal count, rationale length, evidence
  refs, diagnostic count, and diagnostic bytes.
- Ownership: `macaca-skill` owns the provider-neutral validation and sanitized
  error result constructor. Runtime-host can enforce the same budget/resource
  fields when 9.6 provider factory wiring lands.
- Safety: provider errors are converted into structured `Failed` semantic
  review results with sanitized diagnostics. The test gate proves prompt,
  provider-payload, and secret markers are redacted before reports/logs can
  expose them.

### Slice 9D: Runtime-Host Semantic Provider Factory

- Patterns: Runtime-host now owns a small Factory that returns the semantic
  review provider behind the existing `SkillSemanticReviewProvider` Strategy
  contract. The current factory returns the Null Object unavailable provider,
  preserving deterministic curation while giving future providers a single
  wiring point.
- Boundaries: provider construction remains private to runtime-host. No SDK,
  kernel, Web, CLI, or frontend contract changes are required, and the stable
  unavailable provider id lives with the provider type rather than being copied
  across crates.
- Traceability: the factory logs the selected provider id before curation calls
  the provider, and curation continues to log sanitized provider status,
  proposal count, mutation flag, and run identifiers.

### Slice 10A: Umbrella Merge Proposal DTOs

- Patterns: `macaca-skill` defines a metadata-only proposal contract for future
  umbrella merge flows. The DTO separates source skill ids, target umbrella id,
  support-file movement plans, alias effects, bounded risk score, policy refs,
  evidence refs, and rationale before any apply command exists.
- Boundaries: the contract is service-owned and exported through `macaca-skill`
  only. It does not add SDK, kernel, Web, CLI, or frontend dependencies, and it
  does not mutate lifecycle, aliases, or files.
- Safety: validation rejects missing identities, source-equals-target merges,
  invalid risk scores, blank evidence refs, oversized rationale, and excessive
  source/movement/alias/evidence counts.

### Slice 10B: Merge Eligibility Specification

- Patterns: `macaca-skill` now exposes an executable Specification object for
  merge eligibility. The policy compares sanitized facts rather than paths or
  raw skill bodies.
- Compatibility: the default policy requires exact matches for source scope,
  ownership class, permission set, trust level, package source, executable
  semantics, tenant id, and capability signature.
- Safety: evaluation returns a bounded allowed/issues decision only. It never
  mutates lifecycle records, aliases, support files, packages, scheduler refs,
  or context snapshots.

### Slice 10C: Support-File Demotion Proposal Classes

- Patterns: `macaca-skill` now models standalone support-file demotion proposals
  so semantic review and curation can describe detail movement before any apply
  command exists.
- Destinations: session-specific references map to `references/`, starter
  templates map to `templates/`, repeatable scripts map to `scripts/`, and
  assets map to `assets/`.
- Safety: validation rejects missing identities, blank paths, oversized
  rationale, blank evidence refs, and any demotion kind whose destination does
  not match the expected support-file area.

### Slice 10D: Approval-Gated Merge Apply Envelope

- Patterns: merge apply is modeled as a command envelope that validates an
  already bounded merge proposal, approval refs, policy decision refs,
  lifecycle transition refs, safe mutation refs, and audit event refs.
- Boundaries: the command carries refs to narrower lifecycle and content
  mutation command surfaces rather than raw skill bodies or file payloads.
- Safety: a provider cannot treat merge apply as a hidden mutation path; apply
  admission fails unless approval, policy, lifecycle, and safe mutation refs are
  all present.

### Slice 10E: Merge Apply Supersede, Alias, Report, And Rollback

- Patterns: runtime-host implements `skill.curation.merge_apply` as a focused
  Strategy that orchestrates existing Skill service commands instead of owning
  new shell semantics. The merge envelope stays the Command boundary, source
  lifecycle changes reuse the State machine through `supersede`, alias writes
  reuse the service-owned alias map, rollback refs use the Memento pattern, and
  lifecycle/alias/rollback events remain Observer evidence in the governance
  event log.
- Boundaries: `macaca-skill` owns the provider-neutral result DTO and command
  name. `macaca-runtime-host` owns the built-in local apply Strategy. Web, CLI,
  SDK, kernel, and frontend do not classify merge content, mutate skill files,
  or construct providers.
- Safety: apply requires one alias effect per absorbed source, approval refs,
  policy refs, lifecycle refs, safe-mutation refs, and audit refs. Pinned source
  skills are denied through the shared pinned mutation guard. The result exposes
  bounded summaries, evidence refs, report refs, rollback refs, superseded ids,
  and alias metadata only; it never returns skill bodies, package bytes,
  provider payloads, prompts, manifests, credentials, or support-file content.

### Slice 11A: Task, Scheduler, And Heartbeat Alias Resolution

- Patterns: runtime-host owns a shared alias-resolution Strategy that calls the
  Skill service through the existing ServiceRuntime Facade immediately before
  autonomous Agent Execution. The Scheduled Agent Task provider uses a
  Builder-style safe metadata path to carry explicit `skill.alias.*` refs into
  Scheduler targets without owning Skill governance.
- Boundaries: Skill alias decisions remain in `service.skill`. Scheduled Agent
  Task stores historical refs as sanitized metadata, Scheduler stores only the
  target payload ref and safe metadata, and Runtime Host performs the just-in-time
  service call before Agent Execution. No kernel, SDK, Web, CLI, frontend, or
  application-specific code branches on skill names or lifecycle rules.
- Traceability: alias requests, hits, misses, unavailable replies, service
  errors, and timeouts are logged with trace id, dispatch source, service id,
  requested skill id, and bounded result facts. Execution metadata records the
  requested id plus effective target id/name/kind/evidence count when the Skill
  service resolves an alias, preserving original task/scheduler refs for audit
  replay.

### Slice 13A: Package Ownership Policy Specification

- Patterns: `macaca-skill` now exposes a service-owned Specification for
  package ownership policy. The policy evaluates sanitized ownership facts and
  operation classes before curation, evolution, merge, alias, or mutation paths
  can change active lifecycle state or package bytes.
- Boundaries: the ownership decision lives in the Skill service contract, while
  runtime-host only invokes it as part of the local mutation Strategy. Shells,
  SDK callers, Context, Task, Scheduler, and the kernel do not implement
  marketplace, application-owned, paid, encrypted, bundled, plugin, central, or
  tenant semantics locally.
- Safety: marketplace skills require local overlay/draft behavior instead of
  upstream mutation, application-owned skills require application-scope approval,
  paid/encrypted skills require mutation entitlement for content or lifecycle
  mutation, and protected ownership classes are marked `Protected` in
  deterministic curation instead of being auto-archived. Alias and metadata
  operations remain available for observability without exposing raw skill
  bodies or package bytes.

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
