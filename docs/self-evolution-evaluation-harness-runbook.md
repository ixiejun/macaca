# Self-Evolution Evaluation Harness Runbook

This runbook describes how operators evaluate governed Skill self-evolution
without giving Web, CLI, or frontend shells ownership of scoring semantics.

## Boundary Rules

- Build evaluation records from service-owned evidence refs only.
- Store checkpoint refs, snapshot refs, rollback refs, policy decision ids, and
  audit event ids; do not store raw prompts, provider payloads, manifests,
  package bytes, credentials, raw task output, or full skill bodies.
- Use generic task family ids such as `spec_change_loop`,
  `runtime_verification_loop`, or `bug_trace_loop`; do not use application,
  workflow, provider, model, driver, gateway, chain, or business-domain names.
- Let the Skill service score and render reports through SDK/SystemFacade
  commands. Shells may submit records and display returned reports only.

## Baseline And Evolved Runs

1. Run the task family once with the evolved skill state hidden or unused.
2. Record baseline metrics: completion success, verified artifact count, human
   intervention count, elapsed seconds, tool call count, retry count, policy
   violation count, skill activation count, proposal acceptance counts, reuse
   score, and regression count.
3. Promote or apply the governed Skill evolution through curation commands.
4. Run the same generic task family with the evolved skill state available.
5. Record evolved metrics with the same field set.

## Required White-Box Checkpoints

Append or construct refs for:

- verified task completion.
- ExperienceCandidate.
- classification result.
- proposal id.
- curation run id.
- promotion or apply evidence.
- active catalog snapshot.
- later skill read or activation.
- policy decision id when present.
- audit event ids.
- before snapshot ref, after snapshot ref, and rollback ref when present.

## Web Report Command

Submit the provider-neutral record to the Web shell adapter:

```bash
curl -s -X POST \
  http://127.0.0.1:3001/api/apps/<app-id>/skills/operations/evaluation/report \
  -H 'Content-Type: application/json' \
  -d @evaluation-record.json
```

`evaluation-record.json` shape:

```json
{
  "include_markdown": true,
  "record": {
    "evaluation_id": "eval-001",
    "trace_id": "trace-eval-001",
    "task_family_id": "spec_change_loop",
    "lifecycle": "EvolvedRecorded",
    "white_box": {
      "verified_task_completion_ref": "event://task/complete",
      "experience_candidate_ref": "event://candidate/1",
      "classification_ref": "event://classification/1",
      "proposal_id": "proposal-1",
      "curation_run_id": "curation-1",
      "promotion_or_apply_ref": "event://promotion/1",
      "active_catalog_snapshot_ref": "store://catalog/after",
      "later_skill_activation_ref": "event://activation/1",
      "policy_decision_id": "policy://decision/1",
      "audit_event_ids": ["audit://event/1"],
      "before_snapshot_ref": "store://catalog/before",
      "after_snapshot_ref": "store://catalog/after",
      "rollback_ref": "store://rollback/1"
    },
    "baseline": {
      "completion_success": true,
      "verified_artifact_count": 2,
      "human_intervention_count": 3,
      "elapsed_seconds": 120,
      "tool_call_count": 20,
      "retry_count": 2,
      "policy_violation_count": 0,
      "skill_activation_count": 0,
      "accepted_proposal_count": 0,
      "total_proposal_count": 1,
      "reuse_score": 0,
      "regression_count": 0
    },
    "evolved": {
      "completion_success": true,
      "verified_artifact_count": 2,
      "human_intervention_count": 1,
      "elapsed_seconds": 90,
      "tool_call_count": 16,
      "retry_count": 1,
      "policy_violation_count": 0,
      "skill_activation_count": 1,
      "accepted_proposal_count": 1,
      "total_proposal_count": 1,
      "reuse_score": 1,
      "regression_count": 0
    },
    "report_refs": {
      "json_report_ref": null,
      "markdown_report_ref": null
    }
  }
}
```

The response contains the Skill service score, sanitized JSON summary, optional
Markdown summary, and the route trace id. A `Failed` or `Inconclusive` score is
evidence that self-evolution must not be claimed complete for that run.
