# Developer CI Pack

`pack.developer.ci.v1` provides provider-neutral CI project, pipeline, run,
status, trigger, cancel, rerun, log, artifact, test report, environment, and
provider capability discovery.

The pack exposes CI as a serviceized capability. Applications submit plans and
requests through typed commands; they do not call CI providers directly or
place raw provider logs into trace surfaces.

## Manifest Declaration

```toml
[service_contract]
optional_packs = ["pack.developer.ci.v1"]
```

Unavailable optional declarations report `developer_ci_provider_not_installed`.
Required declarations block readiness until a descriptor-compatible CI provider
is installed with policy, resource, approval, trace, audit, and redaction
decorators.

## Permission Scopes

- `ci.provider.inspect`, `ci.project.read`, `ci.pipeline.read`,
  `ci.run.read`, and `ci.status.read`.
- `ci.trigger.plan`, `ci.trigger.request`, `ci.cancel.plan`,
  `ci.cancel.request`, `ci.rerun.plan`, `ci.rerun.request`, `ci.log.read`,
  `ci.artifact.read`, `ci.test.read`, and `ci.environment.read`.

## Commands

- `ci.inspect_provider`, `ci.list_projects`, `ci.list_pipelines`,
  `ci.list_runs`, `ci.inspect_run`, and `ci.inspect_status`.
- `ci.plan_trigger`, `ci.trigger_run_request`, `ci.plan_cancel`,
  `ci.cancel_run_request`, `ci.plan_rerun`, and `ci.rerun_request`.
- `ci.list_logs`, `ci.get_log`, `ci.list_artifacts`,
  `ci.get_artifact_handle`, `ci.inspect_tests`, and
  `ci.inspect_environment`.

## DTOs And Results

Core DTOs include `CiProviderScope`, `CiProject`, `CiPipelineDefinition`,
`CiRun`, `CiJob`, `CiStep`, `CiStatus`, `CiTriggerPlan`, `CiMutationPlan`,
`CiLogChunk`, `CiArtifact`, `CiTestReport`, `CiEnvironment`, and
`CiProviderCapability`. Result statuses cover success, paging, partial and
streaming results, denied, unavailable, unsupported, conflict, stale status,
quota, timeout, cancellation, approval required, and provider failure.

## Command DTO Details

Every command wrapper carries a `DeveloperCommandEnvelope`:

- `subject_ref`: provider scope, project, pipeline, run, job, log, artifact,
  test report, or environment subject.
- `parameters`: reference-only arguments such as `project_ref`,
  `pipeline_ref`, `run_ref`, `job_ref`, `status_ref`, `trigger_plan_ref`,
  `mutation_plan_ref`, `log_ref`, `artifact_ref`, and `approval_ref`.
- `cursor` and `page_size`: bounded pagination for projects, pipelines, runs,
  logs, artifacts, tests, and environment records.
- `idempotency_key`: stable key for trigger, cancel, and rerun requests.

Result envelopes return `status`, optional `data`, optional paged data, and a
trace-safe error. Trigger, cancel, and rerun commands are split into planning
and request phases; request commands require policy and approval when they can
affect external systems.

## Supplier/API Mapping

- GitHub Actions workflow, run, job, step, artifact, log, environment, and
  rerun/cancel concepts map to CI DTO handles.
- GitLab CI/CD project, pipeline, job, bridge, artifact, environment, and trace
  concepts map to normalized project, pipeline, run, job, log, artifact, and
  environment refs.
- CircleCI and Jenkins pipeline, build, stage, node, test report, artifact, and
  queue concepts map to the same provider-neutral model.
- Provider-specific YAML, secrets, runners, deployment business rules, and raw
  logs remain provider-private.

## Examples

Inspect a run status:

```json
{
  "subject_ref": "ci-run:demo",
  "parameters": { "project_ref": "ci-project:demo" },
  "idempotency_key": "ci-demo-status"
}
```

Plan a trigger:

```json
{
  "subject_ref": "ci-pipeline:demo",
  "parameters": {
    "pipeline_ref": "pipeline:build",
    "environment_ref": "environment:test"
  },
  "idempotency_key": "ci-demo-trigger-plan"
}
```

Unavailable diagnostic:

```json
{
  "pack_id": "pack.developer.ci.v1",
  "required": false,
  "reason_code": "optional_pack_unresolved",
  "message": "developer_ci_provider_not_installed"
}
```

## App-Facing Example Matrix

Generic examples cover pipeline listing, run listing, run-status inspection,
trigger planning, trigger request planning, cancellation planning,
cancellation request planning, redacted log chunk retrieval, artifact listing,
artifact handle retrieval, test inspection, and environment metadata
inspection. All examples use synthetic project, pipeline, run, log-cursor,
artifact, test-report, and environment refs.

Diagnostic examples cover unavailable provider, missing project permission,
unsupported trigger mode, approval-required trigger, stale status,
log-redaction, artifact denied, provider quota, and network denied outcomes.
Diagnostics must use provider-neutral reason codes and must not include
provider names, credentials, real logs, artifact bytes, deployments, secrets, or
repository-specific workflows.

## Provider Conformance

Provider authors must prove descriptor completeness, status normalization,
trigger/cancel/rerun safety, log redaction, artifact handle safety, test report
support, environment protection, resource bounds, policy hooks, sanitized
trace/audit events, unavailable behavior, snapshot/replay metadata, and no raw
logs, credentials, secrets, environment values, artifact bytes, or provider
payload leakage.

## Trace And Audit

Trace and audit events may include project, pipeline, run, status, trigger-plan,
artifact, and test-report refs plus bounded counters and trace-safe errors.
They must not include raw logs, credentials, secrets, environment variables, or
raw provider payloads.

## Provider Replacement

Provider classes are descriptor labels such as `pipeline-service`,
`log-artifact-service`, `mutation-planner`, `mock`, and `unavailable`. Concrete
CI systems, log stores, artifact stores, and trigger executors stay behind
service adapters selected by composition roots.
