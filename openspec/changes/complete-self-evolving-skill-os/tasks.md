## 1. Specification And Governance Alignment

- [x] 1.1 Review `docs/macaca-agent-self-evolving-skills-research.md` and the
  completed skill changes to keep this proposal aligned with current behavior.
- [x] 1.2 Re-read `macaca-os-architecture-governance.md`,
  `macaca-os-microkernel-boundaries.md`, and
  `macaca-os-serviceization-allowlist.md` before each implementation slice.
- [x] 1.3 Confirm every slice assigns ownership to Skill, Store/EventLog, Task,
  Memory, Knowledge, Context, Policy, Entitlement, Runtime Host, SDK, or shell
  without adding kernel semantics.
- [x] 1.4 Keep OpenSpec proposal, design, tasks, and delta spec updated before
  code changes in each slice.
- [x] 1.5 Run `openspec validate complete-self-evolving-skill-os --strict`
  after every spec update.
- [x] 1.6 Record the selected design patterns for each implementation slice in
  the implementation design notes.

## 2. Durable Skill Governance Store

- [x] 2.1 Define provider-neutral Store/EventLog-backed governance record DTOs
  for lifecycle, pinned status, source scope, ownership, trust, evidence refs,
  policy decision refs, audit event ids, and timestamps.
- [x] 2.2 Define durable `SkillProvenance` with skill id, version, author kind,
  author agent id, application id, session id, task id, trace id, evidence refs,
  source scope, trust level, created timestamp, and updated timestamp.
- [x] 2.3 Define durable `SkillUsageTelemetry` with view, activation, resource
  read, patch, successful task, failed task, and last timestamp counters.
- [x] 2.4 Define durable `SkillAliasMap` with source skill, target skill,
  reason, run id, validity window, and resolution policy.
- [x] 2.5 Define durable proposal, curation run, report, snapshot, and rollback
  reference records.
- [x] 2.6 Add append-only governance event types and replayable read-model
  builders.
- [x] 2.7 Implement a built-in local governance store strategy behind the Skill
  service provider.
- [x] 2.8 Add an unavailable governance store strategy that returns structured
  unavailable states without panics or fake success.
- [x] 2.9 Migrate existing in-memory governance, alias, proposal, and lifecycle
  state behind the new store interface through a compatibility adapter.
- [x] 2.10 Add structured logs for store append, read-model replay, snapshot
  build, and store-unavailable decisions.
- [x] 2.11 Test replay restores lifecycle, telemetry, aliases, proposals,
  curation runs, and rollback refs from event records.

## 3. Lifecycle State Machine

- [x] 3.1 Extend lifecycle model to include `Draft`, `Active`, `Stale`,
  `Archived`, `Quarantined`, `Superseded`, and `Rejected`.
- [x] 3.2 Implement a state-machine validator for allowed lifecycle
  transitions.
- [x] 3.3 Require policy decision refs and evidence refs for every mutating
  transition.
- [x] 3.4 Deny archive, supersede, merge apply, or deletion of pinned skills
  unless an explicit approval override contract is added.
- [x] 3.5 Add quarantine commands and quarantine release/restore paths.
- [x] 3.6 Add supersede transitions that require alias or redirect records.
- [x] 3.7 Add rejected proposal transitions that preserve evidence and rationale
  without activating skills.
- [x] 3.8 Update snapshots to expose lifecycle filters without returning skill
  instruction bodies.
- [x] 3.9 Test every allowed and denied lifecycle transition.

## 4. Rich Provenance And Telemetry Capture

- [x] 4.1 Extend usage recording to distinguish view, activation, resource read,
  patch, lifecycle, success, and failure events.
- [x] 4.2 Add provenance capture at skill discovery, proposal creation,
  promotion, patch, curation, merge, archive, restore, quarantine, and rollback.
- [x] 4.3 Attach session, task, application, tenant, trace, and evidence refs
  when available, while keeping fields optional and provider-neutral.
- [x] 4.4 Ensure no raw prompt, raw task output, provider payload, manifest,
  package bytes, secret, credential, or full skill body enters telemetry.
- [x] 4.5 Add bounded telemetry aggregation for snapshot and status commands.
- [x] 4.6 Add tests for success/failure counters influencing dry-run rationale
  without hardcoding application-specific outcomes.

## 5. Task Completion Experience Extraction

- [x] 5.1 Add a Task/Autonomy service integration point for verified terminal
  success events.
- [x] 5.2 Define bounded `ExperienceCandidate` DTOs with summary, trace digest,
  memory digest refs, evidence refs, and provenance.
- [x] 5.3 Require Evidence Gate validation before Skill Evolution receives a
  candidate.
- [x] 5.4 Add generic classification results for memory fact, knowledge digest,
  existing skill patch proposal, new skill draft, support-file draft, and
  discard/no-op.
- [x] 5.5 Route memory and knowledge destinations through their service facades
  instead of writing skill files.
- [x] 5.6 Keep draft creation non-mutating for active catalogs by default.
- [x] 5.7 Add logs for candidate received, evidence accepted/rejected,
  classification result, proposal id, and no-op rationale.
- [x] 5.8 Test missing evidence, unverified task status, oversize summaries, and
  sanitized proposal output.

## 6. Proposal Lifecycle Commands

- [x] 6.1 Add `skill.evolution.propose_patch` typed command and result.
- [x] 6.2 Add `skill.evolution.promote_draft` typed command and result.
- [x] 6.3 Add `skill.evolution.reject_draft` typed command and result.
- [x] 6.4 Add policy and approval decorators before promotion or rejection
  side effects.
- [x] 6.5 Keep rejected proposals durable and auditable with rationale and
  evidence refs.
- [x] 6.6 Ensure promoted drafts update lifecycle, provenance, telemetry, alias
  state, and snapshots atomically through the governance store.
- [x] 6.7 Add SDK/SystemFacade methods and unavailable Null Object behavior for
  all proposal lifecycle commands.
- [x] 6.8 Test promotion, rejection, duplicate promotion denial, missing trace,
  missing evidence, denied policy, and unavailable provider behavior.

## 7. Safe Skill Content Mutation

- [x] 7.1 Define service-owned mutation commands for create, patch, write support
  file, remove support file, archive materialization, and restore
  materialization.
- [x] 7.2 Restrict mutable paths to `SKILL.md`, `references/`, `templates/`,
  `scripts/`, and `assets/` inside the governed skill package root.
- [x] 7.3 Add path traversal, symlink, executable script, size limit, encoding,
  and sensitive-content validation.
- [x] 7.4 Require package guard, entitlement, resource, and policy approval
  before any file side effect.
- [x] 7.5 Implement atomic write and rollback memento creation before mutation.
- [x] 7.6 Keep bundled, marketplace, application-owned, paid, encrypted, and
  cross-tenant skills protected according to ownership policy.
- [x] 7.7 Emit sanitized logs for mutation plan, validation, policy result,
  memento ref, write result, and rollback eligibility.
- [x] 7.8 Test allowed support-file draft writes, denied executable script
  mutation, denied protected package mutation, rollback memento creation, and
  sanitized diagnostics.

## 8. Curation Status, Run, Snapshot, And Rollback

- [x] 8.1 Add `skill.curation.status` typed command with interval, idle,
  budget, provider, last run, next eligible run, and unavailable states.
- [x] 8.2 Add `skill.curation.run` typed command supporting dry-run and
  approval-gated apply modes.
- [x] 8.3 Add `skill.curation.snapshot` typed command for durable governance
  and package memento references.
- [x] 8.4 Add `skill.curation.rollback` typed command that restores lifecycle,
  telemetry, alias, report, and package refs from memento state.
- [x] 8.5 Model `SkillCurationRun` with run id, trace id, started/finished
  timestamps, provider id, dry-run flag, candidate count, actions, snapshots,
  report ref, rollback ref, policy decisions, and audit event ids.
- [x] 8.6 Generate bounded `run.json` and `REPORT.md` store artifacts or
  equivalent durable report refs.
- [x] 8.7 Ensure dry-run does not mutate active governance state, aliases, files,
  scheduler refs, or context snapshots.
- [x] 8.8 Add deterministic stale, archive, quarantine, size, invalid metadata,
  missing dependency, and protected skill phases.
- [x] 8.9 Add structured logs for curation start, phase boundaries, candidate
  counts, policy decisions, report refs, rollback refs, and completion.
- [x] 8.10 Test status, dry-run immutability, apply with approval, rollback,
  pinned protection, protected ownership, absent provider, and report
  sanitization.

## 9. Optional Semantic Review Provider

- [x] 9.1 Define a semantic review provider trait and typed proposal result
  contract.
- [x] 9.2 Add unavailable provider behavior that preserves deterministic
  curation and records semantic analysis as unavailable.
- [x] 9.3 Add optional similarity, clustering, duplicate detection,
  success/failure correlation, and reference graph inputs.
- [x] 9.4 Require semantic providers to output typed proposals only, never direct
  mutations.
- [x] 9.5 Add budget, resource, prompt sanitization, provider payload
  sanitization, and bounded output decorators.
- [x] 9.6 Add runtime-host provider factory wiring without adding SDK, kernel,
  Web, CLI, or frontend dependencies.
- [x] 9.7 Test absent semantic provider fallback, typed proposal validation,
  provider error handling, and no raw provider payload in logs/reports.

## 10. Umbrella Merge And Support-File Demotion

- [x] 10.1 Define merge proposal DTOs with source skills, target umbrella skill,
  support-file movements, alias effects, risk score, and policy refs.
- [x] 10.2 Add merge eligibility specifications for scope, ownership,
  permissions, trust level, package source, executable semantics, tenant, and
  capability compatibility.
- [x] 10.3 Add support-file demotion proposals for session-specific references,
  starter templates, repeatable scripts, and assets.
- [x] 10.4 Apply merge only through approval-gated safe mutation and lifecycle
  transition commands.
- [x] 10.5 Mark absorbed skills `Superseded` and create alias/redirect records.
- [x] 10.6 Ensure merge reports explain rationale with bounded summaries and
  evidence refs.
- [x] 10.7 Test merge eligibility rejection, support-file demotion, alias
  creation, superseded filtering, pinned protection, and rollback.

## 11. Alias Resolution Across Consumers

- [x] 11.1 Keep `SkillAliasMap` resolution in the Skill service as the only
  source of redirect/warn/deny behavior.
- [x] 11.2 Wire Context Composer skill catalog building through Skill service
  alias resolution.
- [x] 11.3 Wire Task service skill references through Skill service alias
  resolution before execution.
- [x] 11.4 Wire Scheduler/Autonomy skill references through Skill service alias
  resolution before wake execution.
- [x] 11.5 Avoid rewriting historical scheduler/task refs by default; preserve
  transparent alias resolution and audit evidence.
- [x] 11.6 Add logs for alias hit, miss, warn, deny, loop prevention, and
  expired alias decisions.
- [x] 11.7 Test alias resolution in context, task, scheduler, superseded skill,
  expired alias, deny alias, and loop cases.

## 12. Context Composer Integration

- [x] 12.1 Consume frozen Skill service governance snapshots instead of reading
  skill directories directly.
- [x] 12.2 Filter normal catalogs to active skills by default.
- [x] 12.3 Exclude or annotate draft, stale, archived, quarantined, rejected,
  and superseded skills according to explicit profile settings.
- [x] 12.4 Add context reports for visible count, filtered count, filter reasons,
  alias resolutions, skill reads, activations, and trace refs.
- [x] 12.5 Ensure draft skills can appear only in explicit experimental
  profiles.
- [x] 12.6 Add tests for lifecycle filtering, alias resolution, activation
  telemetry, and no raw skill body in compact catalog.

## 13. Package, Store, Entitlement, And Ownership Policy

- [x] 13.1 Model skill package ownership for bundled, marketplace,
  application-owned, agent-private, central user, tenant, paid, encrypted, and
  plugin-provided skills.
- [x] 13.2 Add policy specifications for which ownership classes can be patched,
  archived, restored, superseded, merged, or aliased automatically.
- [x] 13.3 Require local overlay/draft behavior for marketplace skills rather
  than upstream mutation.
- [x] 13.4 Require application-scope policy for application-owned skill changes.
- [x] 13.5 Restrict paid/encrypted skills to metadata and alias operations unless
  entitlement grants mutation.
- [x] 13.6 Add tests proving protected ownership classes cannot be auto-patched,
  auto-archived, or mutated by generic agent flows.

## 14. Operations UI And CLI Mutation Adapters

- [x] 14.1 Add Web route commands for approval-gated pin, unpin, archive,
  restore, quarantine, promote, reject, run, apply, and rollback through the SDK
  facade.
- [x] 14.2 Add frontend controls that submit typed commands without owning
  lifecycle, merge, alias, archive, or approval semantics.
- [x] 14.3 Add CLI commands that call the SDK facade and print bounded reports.
- [x] 14.4 Display policy denials, unavailable providers, report refs,
  rollback refs, and audit ids without raw payloads.
- [x] 14.5 Add route logs for trace id, command, target id, bounded counts,
  policy result, and service error class.
- [x] 14.6 Test shell adapters do not import provider crates or implement
  semantic classification.

## 15. Boundary, Security, And Audit Gates

- [x] 15.1 Add dependency-boundary tests proving kernel has no curation,
  evolution, semantic provider, or mutation provider dependency.
- [x] 15.2 Add SDK boundary tests proving SDK does not construct runtime-host
  providers, stores, package guards, or semantic providers.
- [x] 15.3 Add Web/CLI/frontend boundary tests proving shells call facade routes
  and do not own curation semantics.
- [x] 15.4 Add service-provider tests proving no presentation shell dependency.
- [x] 15.5 Add optional-provider tests proving absent semantic, marketplace,
  store, or entitlement providers return structured unavailable/denied states.
- [ ] 15.6 Add audit replay tests for proposal, promotion, mutation, curation
  run, rollback, and alias histories.
- [ ] 15.7 Add sanitization tests proving logs, snapshots, reports, and route
  payloads do not include raw prompts, secrets, provider payloads, manifests,
  package bytes, credentials, raw signatures, full skill bodies, or unbounded
  outputs.
- [ ] 15.8 Add `git diff --check`, targeted cargo checks/tests, frontend
  lint/build where applicable, OpenSpec strict validation, and GitNexus
  `detect_changes` to each implementation slice's completion gate.

## 16. Documentation And Operator Runbooks

- [ ] 16.1 Document Skill Governance Store record classes and replay behavior.
- [ ] 16.2 Document lifecycle states, transition rules, and protected ownership
  classes.
- [ ] 16.3 Document curation run phases, deterministic fallback, semantic
  provider absence, report refs, and rollback flow.
- [ ] 16.4 Document approval policy expectations for private, central,
  application-owned, marketplace, bundled, paid, and encrypted skills.
- [ ] 16.5 Document Context Composer visibility rules and alias behavior.
- [ ] 16.6 Document operation examples using generic skill ids and service
  commands only, with no application-specific workflow names.
