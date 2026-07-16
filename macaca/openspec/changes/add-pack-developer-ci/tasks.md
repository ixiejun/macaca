## 1. Supplier API Research And Scope

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design patterns, OpenSpec rules, and the industrial catalog umbrella proposal before implementation.
- [x] 1.2 Study GitHub Actions REST API for workflow runs, jobs, logs, artifacts, reruns, cancellations, workflow dispatch, attempts, status, and conclusions.
- [x] 1.3 Study GitLab API for pipelines, jobs, bridges, artifacts, traces, triggers, retry/cancel operations, variables, environments, permissions, and statuses.
- [x] 1.4 Study CircleCI API v2 for pipelines, workflows, jobs, tests, artifacts, insights, rerun/cancel operations, and pipeline parameters.
- [x] 1.5 Study Jenkins Remote Access API for jobs, builds, queue items, build parameters, progressive console text, artifacts, result/status, crumb handling, and token authentication.
- [x] 1.6 Produce a supplier capability comparison memo mapping GitHub Actions, GitLab CI/CD, CircleCI, and Jenkins concepts into Macaca provider-neutral CI DTOs and commands.
- [x] 1.7 Define explicit non-goals for concrete provider adapters, release/deployment workflows, terminal execution, repository parsing, raw provider API pass-through, and provider-specific routing.
- [x] 1.8 Record GitNexus CRITICAL/HIGH findings as memo only before implementation commits.

## 2. Contract, Descriptor, And DTOs

- [x] 2.1 Define `pack.developer.ci.v1` descriptor metadata: pack id, family, lifecycle, stability, provider/project support, pipeline model, trigger modes, cancel/rerun support, log support, artifact support, test report support, environment support, auth modes, command schemas, permission scopes, policy templates, resource budgets, approval requirements, data-governance class, SDK metadata, documentation link, compatibility, and diagnostics.
- [x] 2.2 Define provider-neutral DTOs for `CiProviderScope`, `CiProject`, `CiPipelineDefinition`, `CiRun`, `CiJob`, `CiStep`, `CiStatus`, `CiTriggerPlan`, `CiMutationPlan`, `CiLogChunk`, `CiArtifact`, `CiTestReport`, `CiEnvironment`, and `CiProviderCapability`.
- [x] 2.3 Define typed command/result DTOs for `ci.inspect_provider`, `ci.list_projects`, `ci.list_pipelines`, `ci.list_runs`, `ci.inspect_run`, `ci.inspect_status`, `ci.plan_trigger`, `ci.trigger_run_request`, `ci.plan_cancel`, `ci.cancel_run_request`, `ci.plan_rerun`, `ci.rerun_request`, `ci.list_logs`, `ci.get_log`, `ci.list_artifacts`, `ci.get_artifact_handle`, `ci.inspect_tests`, and `ci.inspect_environment`.
- [x] 2.4 Define typed success, paged result, partial result, streaming cursor, validation issue, denied, unavailable, unsupported, conflict, stale-status, quota, timeout, cancellation, approval-required, and failure DTOs.
- [x] 2.5 Define stable descriptor hashing, provider capability hashing, pipeline definition hashing, run state hashing, job state hashing, status normalization, trigger-plan hashing, mutation-plan hashing, log cursor hashing, artifact handle hashing, and redaction metadata.
- [x] 2.6 Add descriptor and DTO compatibility tests for valid descriptors, rejected invalid descriptors, stable hashes, schema evolution, status normalization, trigger plans, mutation plans, log chunks, artifact handles, test reports, redaction profiles, and serde compatibility.

## 3. Admission, Permission, Policy, Resource, Entitlement, And Approval

- [x] 3.1 Implement manifest declaration validation for required and optional `pack.developer.ci.v1` declarations.
- [x] 3.2 Implement permission validation for `ci.provider.inspect`, `ci.project.read`, `ci.pipeline.read`, `ci.run.read`, `ci.status.read`, `ci.trigger.plan`, `ci.trigger.request`, `ci.cancel.plan`, `ci.cancel.request`, `ci.rerun.plan`, `ci.rerun.request`, `ci.log.read`, `ci.artifact.read`, `ci.test.read`, and `ci.environment.read`.
- [ ] 3.3 Implement provider/project/repository/ref/environment scope checks for declared projects, repository handles, ref/commit handles, protected refs, environment handles, runner classes, and denied scopes.
- [ ] 3.4 Implement policy checks for pipeline trigger mode, parameter schema, secret/variable handling, credential reference, network access, environment protection, protected refs, cancellation ownership, rerun state, log redaction, artifact sensitivity, and output redaction.
- [ ] 3.5 Implement resource reservation for run page size, job count, step count, log bytes, artifact size, test report count, provider quota, network transfer, timeout, memory, storage, streaming output, and retained snapshots.
- [ ] 3.6 Implement entitlement checks and structured unavailable/denied diagnostics for missing provider, disabled pack, missing credential reference, missing project permission, unsupported trigger mode, unsupported cancel/rerun, absent log/artifact support, unsupported test reports, disabled network, missing entitlement, provider quota, and host resource denial.
- [ ] 3.7 Implement approval behavior for environment-affecting triggers, deployment-capable runs, protected refs, expensive jobs, external side effects, cancellation of another actor's run, rerun with modified parameters, and artifact/log export outside the application boundary.
- [ ] 3.8 Add tests proving denied, validation, quota, unavailable, stale-status, conflict, unsupported, and approval-required paths do not call concrete providers, trigger runs, cancel runs, rerun jobs, retrieve raw logs, or expose artifact bytes.

## 4. Service Provider And Runtime Integration

- [ ] 4.1 Implement or bind the CI service provider behind the service runtime; do not construct CI providers from SDK, shell, kernel, or application code.
- [x] 4.2 Add a deterministic unavailable provider that returns typed unavailable/unsupported diagnostics and complete discovery metadata.
- [ ] 4.3 Add mock provider support for provider inspection, projects, pipelines, runs, run inspection, status, trigger planning/request, cancel planning/request, rerun planning/request, logs, artifacts, tests, environments, and provider capability inspection.
- [ ] 4.4 Add lifecycle, health, snapshot, shutdown, timeout, cancellation, bounded streaming, paged results, stale-status diagnostics, and rate-limit diagnostics.
- [ ] 4.5 Add Strategy implementations for CI provider adapters, status normalizers, trigger planners, cancel/rerun validators, log readers, artifact handle providers, test report readers, environment inspectors, and unavailable behavior.
- [ ] 4.6 Add side-effect safety support for idempotency keys, provider state validation, queue/run/job preconditions, approval state, environment/ref policy, cancellation ownership, rerun parameter validation, and non-mutating plan commands.
- [ ] 4.7 Add provider capability reporting for available, degraded, preview, unavailable, unsupported, retired, project-specific, pipeline-specific, trigger-limited, cancel-limited, rerun-limited, log-limited, artifact-limited, environment-limited, network-limited, and quota-limited states.

## 5. SDK, WASM ABI, Application Framework, And Examples

- [x] 5.1 Extend SDK discovery for `pack.developer.ci.v1` with command schemas, provider/project support, pipeline model, trigger modes, cancel/rerun support, log support, artifact support, test report support, environment support, auth modes, examples, availability, diagnostics, documentation link, provider class, capability hash, compatibility, and redaction profile.
- [x] 5.2 Extend application admission so required declarations block readiness when unavailable and optional declarations degrade explicitly with effective capability mementos.
- [x] 5.3 Add SDK command helper builders for all `ci.*` commands; helpers must only build canonical traced service calls and must never construct CI clients, access credentials, call remote APIs, retrieve raw logs, download artifact bytes, or bypass policy.
- [ ] 5.4 Extend WASM/app ABI descriptors so applications can discover CI commands, declare permissions, receive unavailable diagnostics, and submit typed service calls through the canonical execution path.
- [x] 5.5 Add generic app-facing examples for listing pipelines, listing runs, inspecting run status, planning a trigger, requesting a trigger, planning cancellation, requesting cancellation, retrieving redacted log chunks, listing artifacts, retrieving artifact handles, inspecting tests, and inspecting environment metadata.
- [x] 5.6 Add unavailable-provider, missing-project-permission, unsupported-trigger-mode, approval-required-trigger, stale-status, log-redaction, artifact-denied, provider-quota, and network-denied examples that demonstrate diagnostics without provider names, credentials, real logs, artifact bytes, deployments, or repository-specific workflows.

## 6. Trace, Audit, Replay, Security, And Gates

- [ ] 6.1 Emit sanitized declaration, admission, provider-inspection, project-list, pipeline-list, run-list, run-inspection, status-inspection, trigger-plan, trigger-request, cancel-plan, cancel-request, rerun-plan, rerun-request, log-list, log-read, artifact-list, artifact-handle, test-inspection, environment-inspection, policy, entitlement, resource, approval, health, snapshot, unavailable, and failure events.
- [ ] 6.2 Ensure traces, audits, snapshots, SDK diagnostics, and examples exclude raw credentials, tokens, secrets, raw logs, raw artifact bytes, raw provider payloads, prompts, manifests, package bytes, private keys, signatures, and unbounded output.
- [ ] 6.3 Add replay tests proving every `ci.*` command is trace-addressable through the canonical service path and that snapshots contain enough bounded metadata for recovery diagnostics.
- [ ] 6.4 Add dependency gates proving kernel, SDK, shells, and generic application framework do not import concrete GitHub Actions, GitLab, CircleCI, Jenkins, HTTP client wrappers, credential managers, terminal clients, or provider adapters.
- [ ] 6.5 Add no-direct-provider-call gates proving SDK helpers, WASM ABI handlers, app admission, web, CLI, and frontend paths route through descriptor-owned service commands.
- [ ] 6.6 Add boundary tests proving optional provider absence returns structured unavailable diagnostics and never crashes, hangs, silently falls back, triggers runs, cancels runs, retrieves raw logs, downloads artifact bytes, contacts providers, or fakes success.
- [ ] 6.7 Run `openspec validate add-pack-developer-ci --strict`, targeted cargo tests, boundary gates, file-size gates, and audit replay checks before marking implementation complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/developer/ci.md` with purpose, capability model, manifest declaration, required versus optional behavior, permissions, provider scopes, projects, pipelines, runs, jobs, steps, statuses, conclusions, logs, artifacts, tests, annotations, environments, runners, queues, trigger/cancel/rerun lifecycle, unavailable diagnostics, provider replacement, and operational limits.
- [x] 7.2 Document all command DTOs and result DTOs with field-level explanations, idempotency semantics, redaction behavior, pagination/streaming behavior, timeout/cancellation behavior, plan/request behavior, approval behavior, log/artifact retention behavior, and structured error codes.
- [x] 7.3 Document supplier/API mapping: GitHub Actions, GitLab CI/CD, CircleCI, and Jenkins concepts mapped to Macaca abstractions, including what is intentionally not exposed as OS semantics.
- [x] 7.4 Add generic examples for run listing, status inspection, trigger planning/request, cancellation planning/request, rerun planning/request, log retrieval, artifact handle lookup, test report inspection, environment inspection, and unavailable diagnostics using synthetic CI data only.
- [x] 7.5 Add conformance checklist and test guidance for provider authors: descriptor completeness, status normalization, trigger/cancel/rerun safety, log redaction, artifact handle safety, test report support, environment protection, resource bounds, policy hooks, trace/audit events, unavailable behavior, snapshot/replay, and redaction.
- [x] 7.6 Cross-link the guide from SDK discovery metadata and the industrial pack catalog index before marking `add-pack-developer-ci` complete.
