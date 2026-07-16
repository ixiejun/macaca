## ADDED Requirements

### Requirement: Macaca SHALL provide Developer CI Pack as a serviceized capability

Macaca SHALL provide `pack.developer.ci.v1` as a provider-neutral industrial
pack for CI provider/project discovery, pipeline definitions, run listing, run
inspection, status diagnostics, trigger planning, trigger requests, cancel
planning, cancel requests, rerun planning, rerun requests, log retrieval,
artifact handle lookup, test report inspection, environment/runner/queue
metadata, provider capability inspection, and unavailable diagnostics.
Applications SHALL declare the pack in manifests, admission SHALL resolve it
into effective capabilities, and all operations SHALL run through typed service
commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.developer.ci.v1` as required and a CI service provider is registered, healthy, entitled, project-compatible, pipeline-compatible, trigger-compatible where requested, log/artifact-compatible where requested, quota-compatible, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, provider/project support, pipeline model, trigger modes, cancel/rerun support, log support, artifact support, test report support, environment support, permission scopes, policy templates, resource limits, health, diagnostics, documentation links, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing credentials, tokens, secrets, raw logs, raw artifact bytes, raw provider payloads, raw manifests, package bytes, private keys, signatures, or unbounded output

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.developer.ci.v1` as required but provider, project scope, pipeline support, trigger mode, cancel/rerun support, log/artifact support, credential reference, permission, entitlement, approval, resource budget, network support, or host support is absent
- **THEN** admission SHALL block readiness with structured unavailable, denied, unsupported, validation, stale-status, approval-required, conflict, quota, timeout, or failure diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, instantiate another provider implicitly, trigger runs, cancel runs, rerun jobs, contact providers, retrieve raw logs, download artifact bytes, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.developer.ci.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report
- **AND** SDK command helpers SHALL return Null Object unavailable diagnostics instead of creating callable service calls

### Requirement: CI commands SHALL use typed canonical service calls

Every `pack.developer.ci.v1` operation SHALL be represented as a typed
command/result DTO and SHALL traverse the canonical service runtime path with
trace, policy, provider/project/ref/environment scope checks, resource,
entitlement, approval, health, snapshot, redaction, replay, and structured error
behavior.

#### Scenario: Runs are listed
- **WHEN** `ci.list_runs` is invoked with provider scope, project handle, pipeline handle, ref/commit filters, status filters, actor filter, time range, and page limits
- **THEN** Macaca SHALL validate run-read permission, project/ref scope, provider capability, freshness, redaction, and resource budget before provider access
- **AND** it SHALL return bounded run pages with run handles, pipeline handles, repository/ref/commit handles, trigger kind, status, conclusion, timestamps, duration, queue state, freshness, and replay pointer

#### Scenario: Run is inspected
- **WHEN** `ci.inspect_run` is invoked with a run handle and requested detail depth
- **THEN** Macaca SHALL validate run-read permission, provider state, job/step detail limits, redaction, and resource budget
- **AND** it SHALL return typed run, attempt, job, step, status, conclusion, timing, dependency, and diagnostic summaries without raw provider payloads

#### Scenario: Status is normalized
- **WHEN** `ci.inspect_status` is invoked for a ref, commit, pipeline, run, or job
- **THEN** Macaca SHALL normalize provider status and conclusion into provider-neutral state/conclusion fields
- **AND** it SHALL retain provider raw status code only as bounded metadata with freshness and blocking diagnostics

#### Scenario: Command is denied before provider call
- **WHEN** policy, provider scope, project scope, ref scope, environment scope, permission, entitlement, approval, resource, network, credential reference, log/artifact redaction, or provider capability checks reject a `ci.*` command
- **THEN** Macaca SHALL return a typed denied, approval-required, validation, stale-status, conflict, quota, timeout, unavailable, or unsupported result before invoking the concrete provider or performing side effects
- **AND** audit evidence SHALL include bounded reason codes without credentials, tokens, secrets, raw logs, raw artifact bytes, raw provider payloads, or unbounded output

### Requirement: CI DTOs SHALL model provider scopes, projects, pipelines, runs, jobs, steps, statuses, trigger plans, mutation plans, logs, artifacts, tests, environments, and provider capability

`pack.developer.ci.v1` SHALL define portable DTOs for CI provider scopes,
projects, pipeline definitions, runs, jobs, steps, statuses, trigger plans,
mutation plans, log chunks, artifacts, test reports, environments, provider
capabilities, result pages, streaming cursors, partial results, and diagnostics.
Provider-specific fields SHALL remain bounded adapter metadata and SHALL NOT
become OS-layer routing branches.

#### Scenario: Developer inspects pipeline schema
- **WHEN** SDK schemas expose `CiPipelineDefinition`
- **THEN** the schema SHALL identify pipeline handle, display name handle, trigger modes, parameter schema, supported refs, environment requirements, concurrency policy, and lifecycle state
- **AND** provider-specific workflow ids SHALL NOT be required for portable application logic

#### Scenario: Developer inspects run and job schemas
- **WHEN** SDK schemas expose `CiRun`, `CiJob`, and `CiStep`
- **THEN** the schemas SHALL include handles, pipeline/run/job relationships, repository/ref/commit handles, actor handle, trigger kind, attempt number, status, conclusion, runner handle, dependencies, timestamps, retries, queue state, and annotation summary
- **AND** raw provider payloads and raw logs SHALL NOT be exposed in schema examples or observability

#### Scenario: Developer inspects log and artifact schemas
- **WHEN** SDK schemas expose `CiLogChunk` and `CiArtifact`
- **THEN** the schemas SHALL include handles, ranges, redaction status, cursor, retention, size class, checksum handle, download capability, and sensitivity class
- **AND** raw log text and artifact bytes SHALL be represented by handles or bounded/redacted chunks according to policy

#### Scenario: Provider reports capability limits
- **WHEN** SDK discovery inspects the active CI provider
- **THEN** Macaca SHALL report provider kind, pipeline model, trigger support, cancel/rerun support, log support, artifact support, test report support, environment support, auth modes, rate limits, lifecycle, health, and capability hash
- **AND** callers SHALL use this metadata instead of provider-name branches

### Requirement: CI side effects SHALL be planned, requested, approval-aware, and auditable

`pack.developer.ci.v1` SHALL separate trigger, cancel, and rerun planning from
trigger, cancel, and rerun request commands. Side-effecting requests SHALL
require permissions, current provider state validation, credential references,
idempotency keys, policy checks, approval when required, and audit records.

#### Scenario: Trigger is planned without side effects
- **WHEN** `ci.plan_trigger` is invoked with pipeline handle, ref/commit handle, parameters, environment handle, actor handle, and resource budget
- **THEN** Macaca SHALL validate trigger mode, parameter schema, repository/ref scope, environment policy, secret/variable policy, quota impact, credential reference, provider capability, and approval requirements
- **AND** it SHALL return trigger plan, expected cost, quota impact, required approvals, idempotency key, validation diagnostics, and replay pointer without triggering a run

#### Scenario: Trigger request is protected
- **WHEN** `ci.trigger_run_request` is invoked with a validated trigger plan for protected refs, deployment-capable environments, expensive jobs, or external side effects
- **THEN** Macaca SHALL require trigger permission, valid credential reference, valid provider state, and approval when policy requires it
- **AND** it SHALL emit sanitized audit evidence with trigger plan handle, run handle when successful, approval status, and replay pointer

#### Scenario: Cancellation is planned before request
- **WHEN** `ci.plan_cancel` is invoked for a run, job, or queue item
- **THEN** Macaca SHALL validate cancel support, target state, ownership policy, environment impact, permission, quota, and approval requirements without cancelling
- **AND** it SHALL return mutation plan and diagnostics

#### Scenario: Rerun request validates provider state
- **WHEN** `ci.rerun_request` is invoked for run, failed jobs, job subset, or attempt
- **THEN** Macaca SHALL validate rerun support, target state, parameters, repository/ref scope, provider quota, approval state, and idempotency key before side effects
- **AND** it SHALL return rerun diagnostics and audit evidence

### Requirement: Logs, artifacts, tests, and environments SHALL be bounded, redacted, and policy-controlled

`pack.developer.ci.v1` SHALL treat logs, artifacts, test reports, annotations,
environments, runners, and queues as potentially sensitive CI evidence.
Access SHALL be bounded, redacted, permission-checked, resource-limited, and
traceable.

#### Scenario: Log chunk is retrieved
- **WHEN** `ci.get_log` is invoked with log handle, range or cursor, redaction profile, and size limit
- **THEN** Macaca SHALL validate log-read permission, retention, redaction, size limits, secret-risk handling, and provider capability
- **AND** it SHALL return bounded log chunks or stream cursor diagnostics without raw unredacted logs in traces, audits, snapshots, or SDK diagnostics

#### Scenario: Artifact handle is returned
- **WHEN** `ci.get_artifact_handle` is invoked with artifact handle, run/job scope, and access policy
- **THEN** Macaca SHALL validate artifact permission, retention, size class, sensitivity class, checksum handle, provider capability, and redaction/export policy
- **AND** it SHALL return a bounded download/access handle rather than raw artifact bytes in observability

#### Scenario: Test report is inspected
- **WHEN** `ci.inspect_tests` is invoked for a run or job
- **THEN** Macaca SHALL validate test-read permission, report availability, result size, redaction, and provider capability
- **AND** it SHALL return suite summaries, test counts, failures, flakes, durations, annotations, and evidence handles

#### Scenario: Environment metadata is protected
- **WHEN** `ci.inspect_environment` is invoked for deployment-capable environments, runners, or queue metadata
- **THEN** Macaca SHALL validate environment-read permission, provider support, redaction, and policy sensitivity
- **AND** it SHALL return bounded environment/runner/queue metadata without secrets or deployment credentials

### Requirement: CI Pack SHALL enforce permissions, scopes, resource limits, entitlements, approvals, and redaction

`pack.developer.ci.v1` SHALL define permission scopes for provider inspection,
project reading, pipeline reading, run reading, status reading, trigger planning,
trigger requests, cancel planning, cancel requests, rerun planning, rerun
requests, log reading, artifact reading, test report reading, and environment
reading. Policy SHALL run before side effects and SHALL account for provider
scope, project scope, repository/ref scope, environment scope, credential
references, network access, provider quota, output size, approval, and redaction.

#### Scenario: Trigger permission is missing
- **WHEN** an application can read CI runs but lacks `ci.trigger.request`
- **THEN** `ci.trigger_run_request` SHALL return a typed denied result before contacting the provider
- **AND** no run SHALL be triggered

#### Scenario: Network is denied
- **WHEN** a CI command requires remote provider access but network policy denies the provider scope
- **THEN** Macaca SHALL return a typed denied or unavailable result before contacting the provider
- **AND** audit evidence SHALL identify the network policy reason by stable code

#### Scenario: Resource limits reject CI operation
- **WHEN** run listing, log retrieval, artifact handle lookup, test report inspection, or trigger planning exceeds run page size, job count, step count, log bytes, artifact size, test report count, provider quota, network transfer, timeout, memory, storage, streaming output, or snapshot limits
- **THEN** Macaca SHALL return typed quota, timeout, cancellation, or partial-result diagnostics
- **AND** it SHALL emit bounded resource counters and stable reason codes

### Requirement: CI Pack SHALL expose industrial metadata and developer documentation

`pack.developer.ci.v1` SHALL expose descriptor metadata for provider/project
support, pipeline model, trigger modes, cancel/rerun support, log support,
artifact support, test report support, environment support, auth modes, command
schemas, permission scopes, policy templates, resource budgets, approval
requirements, lifecycle state, compatibility, health probes, snapshots,
unavailable diagnostics, redaction profiles, SDK examples, provider capability
hashes, and developer documentation.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.developer.ci.v1`
- **THEN** it SHALL return command namespace `ci.*`, provider/project support, pipeline model, trigger modes, cancel/rerun support, log support, artifact support, test report support, environment support, auth modes, supported commands, permissions, policy templates, examples, lifecycle, availability, health, diagnostics, compatibility, redaction profile, provider capability hash, and documentation links
- **AND** examples SHALL use generic handles and synthetic CI data rather than application-specific workflows, provider names, credentials, real logs, artifact bytes, deployments, or repository-specific conventions

#### Scenario: Developer documentation is published
- **WHEN** the pack implementation is marked complete
- **THEN** `docs/developer-packs/developer/ci.md` SHALL document manifest declaration, required versus optional behavior, permissions, provider scopes, projects, pipelines, runs, jobs, steps, statuses, conclusions, logs, artifacts, tests, annotations, environments, runners, queues, trigger/cancel/rerun lifecycle, credential references, network policy, unavailable diagnostics, provider replacement, trace/audit interpretation, operational limits, and conformance tests
- **AND** SDK discovery metadata and the industrial catalog index SHALL link to that guide

### Requirement: CI Pack SHALL be traceable, auditable, replayable, and sanitized

`pack.developer.ci.v1` SHALL emit sanitized trace/audit events and bounded
snapshots for declaration, admission, provider inspection, project listing,
pipeline listing, run listing, run inspection, status inspection, trigger
planning, trigger requests, cancel planning, cancel requests, rerun planning,
rerun requests, log listing, log retrieval, artifact listing, artifact handle
creation, test inspection, environment inspection, policy/resource decisions,
provider calls, unavailable states, and replay.

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a CI pack snapshot
- **THEN** the snapshot SHALL include descriptor version, provider capability hashes, project/run summary, pipeline definition hashes, recent status summary, log/artifact availability, command availability, provider health, policy template hash, resource counters, bounded mutation-plan summaries, and sanitized replay pointers
- **AND** it SHALL exclude raw credentials, tokens, secrets, raw logs, raw artifact bytes, raw provider payloads, manifests, package bytes, private keys, signatures, and unbounded output

#### Scenario: Trigger request is audited
- **WHEN** `ci.trigger_run_request`, `ci.cancel_run_request`, or `ci.rerun_request` runs
- **THEN** Macaca SHALL emit sanitized audit events with provider scope, project handle, plan handle, target run/job handles where applicable, credential reference status, approval status, idempotency key hash, result code, and replay pointer
- **AND** raw credentials, tokens, secrets, raw provider payloads, and raw logs SHALL NOT enter audit records

#### Scenario: Log retrieval is audited
- **WHEN** `ci.list_logs` or `ci.get_log` runs
- **THEN** Macaca SHALL emit sanitized audit events with log handle, chunk range, redaction status, size class, retention class, result code, and replay pointer
- **AND** raw unredacted log text SHALL NOT enter audit records

### Requirement: CI Pack implementation SHALL preserve Macaca boundaries

The `pack.developer.ci.v1` implementation SHALL remain owned by CI service
providers behind the service runtime. The microkernel, SDK, shells, and generic
application framework SHALL remain provider-neutral and free of
application-specific or provider-specific routing branches.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete GitHub Actions, GitLab, CircleCI, Jenkins, HTTP client wrapper, credential manager, terminal client, or provider adapter imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.developer.ci.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, capability hash, and result codes rather than provider-specific business branches

#### Scenario: SDK helper builds service call only
- **WHEN** an SDK helper such as `sdk.packs.developer.ci.trigger_run_request(command)` is used
- **THEN** the helper SHALL build a canonical traced service call with command DTO, permission metadata, provider/project scope, repository/ref scope, resource limits, redaction profile, and replay context
- **AND** it SHALL NOT construct providers, access credentials, call remote CI APIs, retrieve raw logs, download artifact bytes, trigger runs, cancel runs, rerun jobs, route by provider name, or bypass policy
