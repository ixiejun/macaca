# Macaca Agent Self-Evolution 30-Day Monitoring Plan

## Purpose

This document defines a 30-day live monitoring program for Macaca Agent OS
self-evolution. The goal is to verify that real completed agent tasks can
produce reusable Skills, that later tasks can improve or reuse those Skills,
that duplicate Skills are merged through governed curation, and that inactive
Skills are archived only after enough time-based evidence exists.

The plan deliberately treats merge and archive as time-dependent governance
outcomes. A 30-day window is long enough to observe stale-skill pressure,
repeated reuse opportunities, duplicate-skill convergence, and telemetry trend
changes without faking long-term evidence.

## Scope

The monitor covers only generic Macaca OS self-evolution behavior:

- task completion evidence from `/api/chat/v2`;
- self-evolution observer checkpoints and proposal ids;
- Skill proposal processing, quality gates, and duplicate grouping;
- autonomous materialization operator runs;
- Skill catalog and load-path visibility;
- Skill usage telemetry, including activation, use, and successful task counts;
- curation recommendations for optimization, merge, and archive;
- rollback and audit references for any applied mutation.

The monitor must not encode application-specific workflow names, business logic,
provider names, raw prompts, raw model responses, secrets, credentials, package
bytes, or unbounded task output.

## Monitoring Cadence

The monitoring program runs one daily batch for 30 consecutive days. A daily
batch contains multiple task families, because a single task per day cannot
scientifically exercise proposal capture, reuse, optimization, duplicate
pressure, archive pressure, and audit in the same monitoring window.

The default batch contains 6 tasks per day:

| Order | Task Family | Purpose | Expected Signal |
| --- | --- | --- | --- |
| 1 | precipitation_seed | Ask the agent to capture a reusable procedure from current self-evolution evidence. | New artifact, observer event, proposal candidate. |
| 2 | reuse_followup | Ask the agent to reuse yesterday or earlier materialized Skills. | Skill snapshot visibility and usage telemetry delta. |
| 3 | optimization_probe | Ask the agent to improve or produce a no-change review for an existing Skill based on telemetry. | Optimization review or governed no-change evidence. |
| 4 | duplicate_merge_probe | Ask the agent to identify overlapping Skills and propose merge/suppress/keep decisions. | Duplicate groups, semantic review status, curation recommendation. |
| 5 | archive_readiness_probe | Ask the agent to evaluate inactivity using real threshold and diagnostic threshold separately. | Archive candidates without conflating forced tests with production proof. |
| 6 | audit_summary | Ask the agent to write a bounded daily summary using only stable refs. | Human-readable daily report artifact and API audit targets. |

The batch is executed once per calendar day by an operator or by Codex on behalf
of the operator. This keeps the trigger human-controlled while the evidence and
judgment remain service-owned and replayable.

### Daily Execution Script

Use the repository script:

```bash
scripts/run-self-evolution-daily-monitor.sh \
  --api http://127.0.0.1:3001 \
  --app-id <APP_ID> \
  --agent <AGENT_NAME>
```

Optional flags:

- `--day N`: override the monitoring day number.
- `--tasks-per-day N`: run fewer task families for a smoke pass.
- `--default-stale-days N`: curation threshold for the production archive track.
- `--forced-stale-days N`: diagnostic stale threshold, recorded separately.
- `--out-dir PATH`: report root, default
  `docs/self-evolution-monitoring`.

The script records raw snapshots under the daily evidence directory and writes a
sanitized Markdown report. It does not start or stop the Macaca server.

### Daily Codex Prompt

Each day, the operator can send Codex this prompt:

```text
Run today's Macaca self-evolution daily monitor.

Use:
- repo: /Users/quantum/Code/dev/agent
- script: scripts/run-self-evolution-daily-monitor.sh
- api: http://127.0.0.1:3001
- app_id: <APP_ID>
- agent: <AGENT_NAME>
- day: <DAY_NUMBER>

Requirements:
- run the daily batch, not a single task;
- keep all tasks application-neutral;
- capture proposal, materialization, telemetry, curation, archive, merge, and API audit evidence;
- separate real stale-threshold archive evidence from forced diagnostic stale-threshold evidence;
- write or update the daily report under docs/self-evolution-monitoring/daily/;
- summarize only stable refs and aggregate counts, not raw model output.
```

### Phase 1: Evidence Snapshot

Capture the current system state before sending a new task.

Required snapshots:

- `/api/status` availability.
- `/api/apps/{app_id}/skills?agent={agent}` visible catalog.
- `/api/apps/{app_id}/skills/operations?agent={agent}` operations snapshot.
- latest `skill-governance-events.jsonl` tail offsets.
- filesystem listing for workspace `skills/`, `available_skills/`, and
  `.macaca/skill-mutation-mementos/`.

The daily report must record only stable refs and aggregate counts, not full
raw payloads.

### Phase 2: Real Task Batch Execution

Send a real, generic self-evolution task batch through `/api/chat/v2`. The
batch includes repeated, adjacent, and pressure-test workloads so the system can
be evaluated across multiple self-evolution behaviors in the same daily wake.

Recommended rotation:

| Day Range | Task Family | Purpose |
| --- | --- | --- |
| 1-7 | precipitation_check | Verify reusable procedure capture and proposal creation. |
| 8-14 | reuse_followup | Ask later tasks to reuse already materialized Skills. |
| 15-21 | optimization_probe | Ask later tasks to improve existing Skills based on telemetry. |
| 22-27 | duplicate_merge_probe | Ask later tasks to detect and consolidate overlapping Skills. |
| 28-30 | archive_readiness_probe | Ask later tasks to evaluate inactivity and archive readiness. |

Every task must request a bounded artifact under `shared/` and must stay
application-neutral. The artifacts are used as evidence for proposal admission
and operator runs.

### Phase 3: Post-Task Governance Sweep

After the task completes, wait at least 30 seconds for observer and telemetry
events to flush. Then run the following governed checks:

- collect the new SSE terminal event and observer checkpoints;
- compare proposal counts before and after the task;
- verify whether a new proposal has evidence refs and a trace digest;
- run the materialization operator only when evidence refs exist;
- run curation in dry-run mode using the default stale threshold;
- run an additional forced-threshold curation dry run only as a diagnostic
  negative control, never as archive proof;
- call API audit for any newly materialized target skill;
- record telemetry deltas for activation, use, success, fail, and lifecycle.

## Daily Report Template

Each daily report must be written to:

```text
docs/self-evolution-monitoring/daily/YYYY-MM-DD.md
```

Required fields:

```markdown
# Self-Evolution Daily Monitor - YYYY-MM-DD

## Run Identity

- app_id:
- agent:
- task_family:
- session_id:
- task_id:
- sse_capture:
- report_author:

## Pre-Run Snapshot

- proposal_count:
- processing_records:
- processing_duplicate_groups:
- processing_ready:
- processing_waiting_proposals:
- materialization_operator_runs:
- materialization_operator_applied:
- curation_recommendations:
- governance_records:
- visible_workspace_skills:

## Task Artifact

- artifact_path:
- artifact_lines:
- artifact_bytes:
- artifact_purpose:
- application_neutral: yes/no

## Observer Evidence

- agent_execution_completed_seen: yes/no
- proposal_created: yes/no
- proposal_id:
- evidence_refs:
- trace_ref:

## Materialization Evidence

- operator_run_id:
- operator_status:
- mutated:
- selected_proposals:
- materialized_skill_ids:
- rollback_memento_refs:

## Telemetry Delta

| Skill | Activation Before | Activation After | Use Before | Use After | Success Before | Success After |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |

## Curation Evidence

- default_threshold_actions:
- forced_threshold_actions:
- duplicate_groups:
- semantic_merge_status:
- archive_candidates:
- applied_mutations:

## API Audit

- target_skill:
- audit_status:
- operations_refs:
- registry_refs:
- observer_refs:
- missing_refs:

## Daily Verdict

- precipitation: pass/fail/inconclusive
- materialization: pass/fail/inconclusive
- reuse_or_optimization: pass/fail/inconclusive
- merge_readiness: pass/fail/inconclusive
- archive_readiness: pass/fail/inconclusive
- blocking_gap:
```

## Monthly Verification Criteria

At day 30, the system can be called complete for Skill-level self-evolution only
if all required criteria pass.

### Required Pass Criteria

| Area | Required Evidence |
| --- | --- |
| Task-to-proposal | At least 20 daily tasks create proposal evidence or explicit governed rejection evidence. |
| Admission quality | No active Skill is admitted without evidence refs, trace refs, bounded reusable procedure text, and quality gate status. |
| Materialization | At least 3 proposal-linked Skills are materialized through the operator with rollback mementos. |
| Later reuse | At least 2 materialized Skills show later-day activation/use/success telemetry deltas. |
| Optimization | At least 1 Skill shows a telemetry-driven optimization review or a governed no-change decision after repeated use. |
| Duplicate merge | At least 1 duplicate group is detected and either merged, suppressed, or explicitly kept with a governed reason. |
| Archive readiness | Skills older than the configured stale threshold are archived or explicitly kept with evidence. During a 30-day run, archive can pass only if the configured threshold is <= 30 days or the skill has a policy-approved forced stale simulation. |
| Rollback | Every mutation has a rollback memento or rollback ref. |
| API audit | API-first audit passes for materialized target skills, including operations, registry/load-path, and observer evidence. |
| Reportability | All daily reports and the monthly report contain stable refs rather than raw provider payloads. |

### Incomplete Criteria

The system must not be called complete if any of these remain true at day 30:

- proposals accumulate but never progress to materialization, rejection, or
  duplicate suppression;
- later tasks only read files as ordinary artifacts and do not increment Skill
  telemetry;
- duplicate detection reports candidates but no governed merge/suppress/keep
  decision is recorded;
- stale candidates are detected only through forced-threshold dry runs and no
  real threshold-based curation path exists;
- API audit cannot connect operations, registry/load-path, and observer evidence
  for the same target skill;
- mutation records lack rollback mementos.

## Archive Testing Policy

Archive behavior is time-sensitive and must not be faked as a production pass.
The monitoring program uses two archive tracks:

1. **Real-time track:** default stale threshold, usually 30 days or longer. Only
   this track can prove production archive behavior.
2. **Diagnostic track:** forced stale threshold, such as 0 or 1 day. This track
   proves the curation evaluator can identify candidates, but it cannot prove
   production archive correctness unless a policy decision explicitly authorizes
   the shortened threshold for a test namespace.

The monthly report must keep these tracks separate.

## Merge Testing Policy

Duplicate merge can be verified before 30 days because it depends on similarity
and governance decisions rather than age. The monitor must check:

- duplicate group count;
- duplicate signature or semantic similarity evidence;
- selected primary and secondary Skill ids;
- merge/suppress/keep recommendation;
- whether semantic review was available;
- whether the mutation was applied or only recommended;
- rollback refs for any applied merge.

If the semantic merge provider is unavailable, the report must mark merge as
`inconclusive` or `detected-only`, not pass.

## Monthly Report Template

The day-30 report must be written to:

```text
docs/self-evolution-monitoring/monthly/YYYY-MM-self-evolution-verification.md
```

Required sections:

```markdown
# Macaca Self-Evolution Monthly Verification - YYYY-MM

## Executive Verdict

- complete_skill_level_self_evolution: yes/no
- complete_agent_self_evolution: yes/no
- strongest_proven_layer:
- weakest_blocking_layer:

## Evidence Coverage

| Day | Task Family | Proposal | Materialized | Reused | Optimized | Merge Decision | Archive Decision | Audit |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |

## Metrics

- total_tasks:
- successful_tasks:
- proposals_created:
- proposals_rejected:
- proposals_materialized:
- duplicate_groups_detected:
- duplicate_groups_resolved:
- archive_candidates_detected:
- archive_actions_applied:
- rollback_mementos:
- audit_pass_count:
- audit_fail_count:

## Findings

## Blocking Gaps

## Required Fixes Before Claiming Complete Self-Evolution
```

## Operating Principle

The monthly result is evidence-driven. A human may schedule the daily monitor,
but the pass condition depends on service-owned evidence: observer events,
proposal processing, materialization, telemetry, curation decisions, rollback
mementos, and API audit. If any claim depends on manually reading files without
service telemetry, the result is partial and must be labeled as such.
