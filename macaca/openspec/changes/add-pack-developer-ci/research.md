# Developer CI Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
explicit non-goals, existing platform inventory, and GitNexus memo evidence for
`pack.developer.ci.v1`. CI support must expose pipeline, workflow, job, run,
attempt, log, artifact, test, rerun, cancel, trigger, variable, environment,
permission, and status operations through serviceized commands. It must not
become a release/deployment workflow engine, terminal runner, repository parser,
or raw provider API pass-through.

## Source Baseline

- GitHub Actions REST workflow runs, artifacts, reruns, cancellations, and
  dispatch: <https://docs.github.com/rest/actions/workflow-runs>
- GitLab pipelines and jobs APIs:
  <https://docs.gitlab.com/api/pipelines/>
  and <https://docs.gitlab.com/api/jobs/>
- CircleCI API v2:
  <https://circleci.com/docs/api/v2/>
  and <https://circleci.com/docs/guides/toolkit/api-intro/>
- Jenkins Remote Access API:
  <https://www.jenkins.io/doc/book/using/remote-access-api/>

## Supplier API Notes

- GitHub Actions contributes workflow runs, jobs, logs, artifacts, rerun,
  cancel/force-cancel, workflow dispatch, attempts, statuses, conclusions, and
  repository-scoped permissions. Macaca should model run attempts and artifacts
  without binding to GitHub-specific event names.
- GitLab contributes pipelines, jobs, bridges, artifacts, traces, triggers,
  retry/cancel/play operations, variables, environments, permissions, and
  status metadata. Macaca should model child/downstream relationships and trace
  streaming bounds.
- CircleCI contributes pipelines, workflows, jobs, tests, artifacts, insights,
  rerun/cancel/approve operations, pipeline parameters, and project/org
  identity. Macaca should model approvals and parameterized triggers generically.
- Jenkins contributes jobs, builds, queue items, build parameters, progressive
  console text, artifacts, result/status, crumb handling, and token
  authentication. Macaca should treat crumb/token behavior as provider
  authentication capability behind secret references.

## Macaca-Owned Abstractions

`pack.developer.ci.v1` should define `CiProviderRef`, `CiPipeline`,
`CiWorkflow`, `CiRun`, `CiAttempt`, `CiJob`, `CiStep`, `CiLogCursor`,
`CiArtifact`, `CiTestReport`, `CiTriggerRequest`, `CiVariable`,
`CiEnvironment`, `CiApproval`, `CiStatus`, and `CiProviderCapability`.

The DTOs must carry provider-neutral run identity, commit/ref linkage,
attempt/version data, job graph, status/conclusion, bounded log cursors,
artifact handles, test summaries, trigger idempotency, variable redaction,
approval state, permissions, provider capability hashes, and replay pointers.
Raw provider payloads, tokens, full unbounded logs, secrets, artifacts beyond
declared handles, and deployment-specific workflow semantics are rejected.

## Explicit Non-Goals

- Do not implement concrete GitHub Actions, GitLab CI, CircleCI, Jenkins,
  release, deployment, terminal execution, repository parsing, or artifact
  storage providers in this research phase.
- Do not define release trains, deployment environments, test policies,
  organization-specific build workflows, or application-specific CI behavior in
  OS layers.
- Do not expose raw provider API requests, provider workflow YAML, secrets, or
  provider-specific routing as stable SDK contracts.

## Existing Macaca Platform Inventory

- Generic descriptors, `SystemFacade`, service-call tracing,
  unavailable/null-object behavior, policy/resource gates, persistence
  snapshots, repository pack adjacency, terminal pack adjacency, and
  secrets-reference handles provide reusable substrate.
- Current evidence does not prove CI DTOs, providers, SDK helpers, WASM ABI,
  tests, dependency gates, or developer docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
