# Developer CI Pack Design

## Context

`pack.developer.ci.v1` exposes CI/CD system access as a Macaca OS serviceized
capability. It lets applications inspect and coordinate pipeline status, logs,
artifacts, test reports, and CI actions without embedding GitHub Actions,
GitLab CI, CircleCI, Jenkins, provider tokens, or release workflow semantics
into generic OS layers.

CI systems bridge code, secrets, compute, environments, artifacts, and external
deployment targets. Trigger/cancel/rerun operations are side effects; logs and
artifacts can contain secrets; status can be stale or provider-specific. The
pack therefore models CI resources with typed handles, plans, requests,
redaction, policy gates, approvals, resource budgets, trace/audit records, and
provider replacement.

## Supplier Capability Matrix

| Supplier/platform | Industrial capability | Macaca abstraction |
| --- | --- | --- |
| GitHub Actions | Workflow runs, jobs, logs, artifacts, reruns, cancellations, workflow dispatch, attempts, status/conclusion | Pipeline definition, run, job, attempt, log, artifact, trigger plan, cancel/rerun request, status DTO |
| GitLab CI/CD | Pipelines, jobs, bridges, artifacts, traces, triggers, retry/cancel, variables, environments | Pipeline/run/job, bridge/downstream metadata, artifact/log handles, trigger inputs, environment metadata |
| CircleCI API v2 | Pipelines, workflows, jobs, tests, artifacts, insights, rerun/cancel, parameters | Pipeline/workflow/run/job model, test report, artifact, rerun plan, provider insight metadata |
| Jenkins Remote API | Jobs, builds, queue, parameters, progressive console text, artifacts, build result/status | Job/build/run, queue item, parameter set, progressive log cursor, artifact handle, result status |

The pack exposes provider-neutral contracts. Provider adapters may translate to
remote APIs, local CI servers, hosted CI, or mocks, but callers see stable DTOs
and structured results.

## Goals

- Provide stable pack id `pack.developer.ci.v1` and command namespace `ci.*`.
- Support CI provider/project discovery, pipeline definitions, run listing,
  run/job/step inspection, status diagnostics, trigger planning, trigger
  requests, cancel/rerun planning and requests, log retrieval/stream handles,
  artifact listing/download handles, test report summaries, annotations,
  environment/runner/queue metadata, and provider capability inspection.
- Preserve safety with repository/ref scopes, credential references, environment
  policies, approval tokens, redaction, quotas, and audit.
- Keep concrete CI providers behind replaceable service providers.
- Require developer documentation at `docs/developer-packs/developer/ci.md`.

## Non-Goals

- Do not implement concrete GitHub Actions, GitLab, CircleCI, Jenkins, SSH,
  credential, or CI-provider adapters in this proposal.
- Do not define application-specific release, deployment, review, branch, issue,
  or incident workflows.
- Do not execute terminal commands or parse repository state directly; those are
  terminal/repository/code pack responsibilities.
- Do not expose raw credentials, secrets, provider tokens, raw log text, raw
  artifact bytes, raw provider payloads, prompts, manifests, package bytes,
  private keys, signatures, or unbounded output in observability.
- Do not silently trigger, cancel, rerun, deploy, or mutate CI state without a
  typed request, policy checks, and approval where required.

## Ownership And Boundaries

- Pack id: `pack.developer.ci.v1`.
- Family: `developer`.
- Backing service owner: CI service provider.
- SDK surface: `sdk.packs.developer.ci`.
- Command namespace: `ci.*`.
- Microkernel owns identity, policy facade, resource primitives, service-call
  evidence, trace/audit primitives, and registry metadata only.
- Application framework owns manifest declarations, app-scoped permissions, and
  effective capability projection.
- Runtime host owns provider adapter registration, credential bridge
  composition, network bridges, decorators, and sanitized diagnostics through
  approved composition roots.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `ci.inspect_provider` | Inspect CI provider/project capability | Returns sanitized capability, auth, quota, and health metadata |
| `ci.list_projects` | List CI projects/repositories visible to the declared scope | Requires provider/project permission and bounded paging |
| `ci.list_pipelines` | List pipeline/workflow definitions | Returns provider-neutral definitions and trigger support |
| `ci.list_runs` | List runs by project, pipeline, ref, commit, status, actor, or time range | Returns bounded run pages and freshness metadata |
| `ci.inspect_run` | Inspect run, attempts, jobs, status, conclusion, trigger, and timing | Returns typed run/job/step summaries |
| `ci.inspect_status` | Inspect status/conclusion/check summary for ref/commit/run | Returns normalized status and provider raw status code as bounded metadata |
| `ci.plan_trigger` | Plan a run trigger from pipeline, ref, parameters, environment, and actor | Validates input schema, repository/ref scope, secrets policy, quotas, and approvals |
| `ci.trigger_run_request` | Request a run trigger from a validated plan | Requires trigger permission, credential reference, approval where required, and audit |
| `ci.plan_cancel` | Plan cancellation of run/job/queue item | Validates ownership, state, policy, and approval |
| `ci.cancel_run_request` | Request cancellation | Requires cancel permission and emits mutation audit |
| `ci.plan_rerun` | Plan rerun of run/job/failed jobs/attempt | Validates provider support, parameters, state, and quota |
| `ci.rerun_request` | Request rerun from validated plan | Requires trigger/rerun permission and audit |
| `ci.list_logs` | List available log streams/chunks | Returns log handles, ranges, retention, and redaction metadata |
| `ci.get_log` | Retrieve bounded log chunk or stream cursor | Applies secret redaction, size limits, and retention policy |
| `ci.list_artifacts` | List artifacts by run/job/name/type | Returns artifact handles and metadata without bytes |
| `ci.get_artifact_handle` | Create/read artifact download handle | Requires artifact permission, retention policy, and redaction |
| `ci.inspect_tests` | Inspect test reports, annotations, failures, flakes, and durations | Returns bounded test summaries and evidence handles |
| `ci.inspect_environment` | Inspect deployment/environment/runner/queue metadata where supported | Returns bounded provider-neutral metadata |

Every command must define typed command DTOs, typed success results, typed
partial/paged/streaming results, validation results, typed denied/unavailable/
unsupported/conflict/quota/timeout/cancellation/approval-required/failure
results, redaction profile, idempotency semantics for side effects, and replay
metadata.

## DTO Model

Core DTOs:

- `CiProviderScope`: provider scope handle, project/repository handle,
  credential reference, network policy, rate limit profile, and health.
- `CiProject`: project handle, repository handle, default ref, visibility,
  permission state, pipeline count, provider capability hash, and redaction
  class.
- `CiPipelineDefinition`: pipeline handle, display name handle, trigger modes,
  parameter schema, supported refs, environment requirements, concurrency
  policy, and lifecycle state.
- `CiRun`: run handle, pipeline handle, repository/ref/commit handles, actor
  handle, trigger kind, attempt number, status, conclusion, started/completed
  timestamps, duration, queue state, and freshness.
- `CiJob`: job handle, run handle, name handle, status, conclusion, runner
  handle, steps, started/completed timestamps, retries, and dependency metadata.
- `CiStep`: step handle, job handle, name handle, status, conclusion, log
  handle, timing, and annotation summary.
- `CiStatus`: normalized state, conclusion, provider raw status code handle,
  freshness, required check flag, blocking reason, and diagnostics.
- `CiTriggerPlan`: plan handle, pipeline handle, ref/commit handles, parameter
  handles, environment handle, expected cost, quota impact, required approvals,
  idempotency key, and validation diagnostics.
- `CiMutationPlan`: plan handle, mutation kind, target run/job/queue handles,
  state preconditions, required approvals, idempotency key, and recovery notes.
- `CiLogChunk`: log handle, chunk range, redaction status, line count, byte
  count, cursor, retention, and secret-risk flags.
- `CiArtifact`: artifact handle, run/job handle, name handle, type, size class,
  retention, checksum handle, download capability, and sensitivity class.
- `CiTestReport`: report handle, run/job handle, suite summaries, test counts,
  failures, flakes, durations, annotations, and evidence handles.
- `CiEnvironment`: environment handle, name handle, deployment/protection state,
  runner class, queue state, approval requirements, and redaction class.
- `CiProviderCapability`: provider kind, pipeline model, trigger support,
  cancel/rerun support, log support, artifact support, test report support,
  environment support, auth modes, rate limits, lifecycle, and health.

Provider-specific extensions may appear only as bounded `adapter_metadata`
behind capability hashes and must not drive OS-layer routing.

## Permission, Policy, Resource, Entitlement, And Approval Model

Permission scopes:

- `ci.provider.inspect`
- `ci.project.read`
- `ci.pipeline.read`
- `ci.run.read`
- `ci.status.read`
- `ci.trigger.plan`
- `ci.trigger.request`
- `ci.cancel.plan`
- `ci.cancel.request`
- `ci.rerun.plan`
- `ci.rerun.request`
- `ci.log.read`
- `ci.artifact.read`
- `ci.test.read`
- `ci.environment.read`

Policy defaults:

- Every command is scoped to application id, tenant id, session id, task id,
  trace id, provider scope, project/repository handle, ref/commit handle, and
  environment handle when available.
- Trigger/rerun/cancel commands require explicit plan/request separation,
  idempotency key, provider state validation, credential reference, and audit
  reason.
- Environment-affecting or deployment-capable runs, protected refs, expensive
  jobs, external side effects, or cancellation of another actor's run require
  approval.
- Logs and artifacts require redaction and bounded output. Raw logs and raw
  artifact bytes must not enter observability.
- Remote operations require network permission, provider quota, rate limits,
  timeout, cancellation, and structured unavailable behavior.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, family, version, lifecycle, command schemas,
provider/project support, pipeline model, trigger modes, cancel/rerun support,
log support, artifact support, test report support, environment support,
permission scopes, policy templates, resource limits, approval rules, provider
capability hashes, health, compatibility, diagnostics, examples, redaction
profiles, and documentation links.

The developer guide at `docs/developer-packs/developer/ci.md` must cover:

- manifest declaration and optional/required behavior
- provider scopes, projects, pipelines, runs, jobs, steps, statuses, conclusions,
  logs, artifacts, tests, annotations, environments, runners, queues, and
  provider capabilities
- trigger/cancel/rerun plan and request lifecycle
- repository/ref/environment scopes, credential references, network policy,
  approvals, quotas, log/artifact redaction, unavailable diagnostics, provider
  replacement, trace/audit interpretation, and conformance tests

Examples must use synthetic project/run/job/artifact handles. They must not
include provider names, real tokens, private logs, artifact bytes, deployment
workflows, or repository-specific conventions.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `ci_pack_declared`
- `ci_pack_admission_validated`
- `ci_provider_inspected`
- `ci_projects_listed`
- `ci_pipelines_listed`
- `ci_runs_listed`
- `ci_run_inspected`
- `ci_status_inspected`
- `ci_trigger_planned`
- `ci_trigger_requested`
- `ci_cancel_planned`
- `ci_cancel_requested`
- `ci_rerun_planned`
- `ci_rerun_requested`
- `ci_logs_listed`
- `ci_log_retrieved`
- `ci_artifacts_listed`
- `ci_artifact_handle_created`
- `ci_tests_inspected`
- `ci_environment_inspected`
- `ci_pack_policy_decision`
- `ci_pack_service_call_requested`
- `ci_pack_service_call_succeeded`
- `ci_pack_service_call_failed`
- `ci_pack_unavailable`
- `ci_pack_snapshot_recorded`

Snapshots include descriptor version, provider capability hashes, project/run
summary, pipeline definition hashes, recent status summary, log/artifact
availability, command availability, provider health, policy template hash,
resource counters, bounded mutation-plan summaries, and sanitized replay
pointers. Snapshots must exclude raw credentials, tokens, secrets, raw logs, raw
artifact bytes, raw provider payloads, manifests, package bytes, private keys,
signatures, and unbounded output.

## Design Patterns

- **Facade**: SDK clients expose discovery and command builders only.
- **Command**: every operation is a typed command/result DTO.
- **Strategy**: CI provider adapters, status normalizers, log readers, artifact
  readers, trigger planners, mutation validators, and unavailable behavior are
  replaceable.
- **Decorator**: trace, policy, entitlement, resource, approval, metering,
  network policy, credential redaction, log/artifact redaction, and mutation
  safety wrap service calls.
- **Specification**: admission validates provider scope, pipeline support,
  command availability, permissions, ref/environment policy, provider state,
  quota, and compatibility.
- **Observer**: run status, job status, log availability, artifact availability,
  health, trace, and audit events are subscribable.
- **Memento**: run snapshots, status hashes, trigger plans, mutation plans, log
  cursors, artifact handles, and replay pointers preserve recovery state.
- **Abstract Factory**: concrete CI providers are created only by approved
  runtime-host composition roots.

## Risks And Mitigations

- Risk: CI pack becomes a provider workflow wrapper. Mitigation: use
  provider-neutral pipeline/run/job/step DTOs and keep PR/release/deploy
  orchestration in higher workflow packs.
- Risk: logs leak secrets. Mitigation: log handles, redaction status, bounded
  chunks, secret-risk flags, and strict observability exclusions.
- Risk: triggers deploy or spend resources unexpectedly. Mitigation:
  plan/request split, quotas, environment/ref policy, approvals, and audit.
- Risk: provider status semantics are inconsistent. Mitigation: normalized
  status/conclusion plus bounded provider raw status code metadata.
- Risk: SDK helpers become a second execution path. Mitigation: helpers build
  canonical service commands and never call CI APIs directly.
