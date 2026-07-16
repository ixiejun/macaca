# Change: Add Developer CI Pack

## Why

Developers need `pack.developer.ci.v1` as an industrial CI/CD capability for
pipeline discovery, workflow/run/job inspection, status diagnostics, trigger
planning, trigger requests, cancel/rerun requests, log retrieval, artifact
lookup/download handles, test report summaries, environment/runner metadata,
and provider health. It must not be a thin wrapper around one vendor's workflow
API or a repository-specific release process.

CI operations are networked and side-effectful. Triggering a pipeline can spend
money, deploy code, publish artifacts, or expose secrets in logs. Cancelling or
rerunning jobs can affect team workflows. Macaca must expose CI operations as
provider-neutral typed commands with repository/ref scope, credential
references, policy gates, approvals, quotas, sanitized logs/artifacts,
trace/audit records, snapshots, replay, and structured unavailable diagnostics.

## Research And Supplier/API Baseline

Official references considered for this pack:

- GitHub Actions REST API covers workflow runs, jobs, logs, artifacts, reruns,
  cancellations, workflow dispatch, status, conclusions, and run attempts.
  Reference: https://docs.github.com/en/rest/actions
- GitLab API covers pipelines, jobs, bridges, artifacts, traces, pipeline
  triggers, retry/cancel operations, statuses, variables, environments, and
  permissions. Reference: https://docs.gitlab.com/api/
- CircleCI API v2 covers pipelines, workflows, jobs, tests, artifacts, insights,
  rerun/cancel operations, and project-level pipeline parameters. Reference:
  https://circleci.com/docs/api/v2/
- Jenkins Remote Access API covers jobs, builds, queue items, build parameters,
  progressive console text, artifacts, build result/status, and crumb/API token
  authentication. Reference:
  https://www.jenkins.io/doc/book/using/remote-access-api/

Macaca maps these supplier concepts into provider-neutral pipeline, run, job,
step, log, artifact, trigger, cancellation, rerun, test report, environment,
runner, and capability DTOs. Concrete CI clients, tokens, provider workflows,
and repository-specific release semantics remain behind replaceable providers.

## What Changes

- Add provider-neutral `pack.developer.ci.v1` under the `developer` family.
- Define command namespace `ci.*` for:
  - provider/project/pipeline discovery
  - run/workflow/job/step listing and inspection
  - status and conclusion diagnostics
  - trigger planning and trigger requests
  - cancel/rerun planning and requests
  - log listing, retrieval, streaming handles, and redaction
  - artifact listing and artifact download handles
  - test report and annotation summaries
  - environment, runner, queue, and provider capability inspection
- Define DTOs for CI provider handles, projects, pipeline definitions, runs,
  jobs, steps, statuses, conclusions, trigger plans, cancellation/rerun plans,
  logs, artifacts, test reports, annotations, environments, runners, queues,
  provider capabilities, and diagnostics.
- Define permission scopes, policy defaults, repository/ref/environment gates,
  credential redaction, approval rules, entitlement checks, structured
  unavailable behavior, SDK discovery, developer documentation, trace/audit
  events, snapshots, replay, and boundary gates.
- Require detailed developer documentation at
  `docs/developer-packs/developer/ci.md` before implementation completion.

## Impact

- Affected specs: `pack-developer-ci`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral protocol DTOs, pack descriptors,
  admission validators, SDK discovery and command builders, CI service provider
  or unavailable provider, runtime-host provider adapters, trace/audit schemas,
  replay tests, dependency-boundary gates, and developer documentation.
- Non-goals: no concrete GitHub Actions/GitLab/CircleCI/Jenkins provider
  implementation in this proposal; no application-specific release/deploy
  workflow; no provider-name routing in OS layers; no raw tokens, secrets, logs,
  artifact bytes, or provider payloads in observability; no SDK/shell/kernel
  provider construction; no fake success when provider, repository/ref scope,
  entitlement, permission, remote access, or host support is absent.
