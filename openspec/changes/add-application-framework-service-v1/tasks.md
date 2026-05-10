## 1. Preparation and Impact Audit

- [x] 1.1 Read the S7 plan, Route C overview, microkernel boundary, serviceization allowlist, architecture governance, and regression matrix.
- [x] 1.2 Inspect current `macaca-app`, `macaca-runtime-host`, `macaca-sdk`, `macaca-web`, and integration-test application call paths before editing.
- [x] 1.3 Run GitNexus impact before modifying existing structs, functions, traits, or methods; report direct callers, affected processes, and risk level.
- [x] 1.4 Classify every direct application path as service contract, runtime-host provider, SDK client, Web adapter, CLI adapter, kernel compatibility, test, or provider-internal.
- [ ] 1.5 Confirm every touched file remains under 500 LOC or split it before adding logic. Existing Web files `lib.rs`, `routes.rs`, and `chat_orchestrator.rs` already exceed 500 LOC and remain migration debt; new service files stay under 500 LOC.

## 2. OpenSpec

- [x] 2.1 Create `add-application-framework-service-v1` proposal, design, tasks, and delta specs.
- [x] 2.2 Validate with `openspec validate add-application-framework-service-v1 --strict`.
- [x] 2.3 Confirm scope stays on Application Framework serviceization and does not absorb Gateway, Store/Entitlement, Payment, Web3, EVM, or full Web/CLI thin shell phases.

## 3. Application Service Contract

- [x] 3.1 Add `APPLICATION_SERVICE_ID` and operation constants for discover, load, start, stop, remove, status, snapshot, session start/resume/stop, host dispatch, and GenUI surface lookup.
- [x] 3.2 Add typed commands/results for application lifecycle, session envelope, host dispatch, and GenUI surface operations.
- [x] 3.3 Add sanitized application, agent, runtime, session, unavailable, and snapshot view DTOs.
- [x] 3.4 Ensure command/result DTOs never expose prompt bodies, full manifest bodies, raw agent configs, env values, API keys, secrets, or raw host payloads.
- [x] 3.5 Add detailed English comments explaining provider-neutral DTO ownership and why snapshots are safe to expose.

## 4. Admission Specification and Lifecycle Projection

- [x] 4.1 Add `ApplicationTraceSpec`, `ApplicationManifestSpec`, `ApplicationRuntimeKindSpec`, and `ApplicationScopeSpec`.
- [x] 4.2 Add projection helpers between `AppStatus` and `ApplicationLifecycleState`.
- [x] 4.3 Model WASM metadata-only admission and structured runtime-unavailable execution through the service path.
- [x] 4.4 Mark direct `AppRuntime` / `AppLoader` startup APIs as deprecated where feasible without deleting or changing legacy behavior.
- [x] 4.5 Add structured logs for manifest validation, runtime kind admission, lifecycle projection, and rejected commands.

## 5. Runtime-Host Provider

- [x] 5.1 Add `ApplicationSystemServiceProvider`.
- [x] 5.2 Translate `ServiceCommand` payloads into typed application service commands and structured results.
- [x] 5.3 Delegate discovery/list to `AppRegistry`, YAML lifecycle to `AppRuntime`, ABI metadata to application ABI adapters, host dispatch to `ApplicationHost`, and GenUI lookup to the GenUI/ApplicationHost seam.
- [x] 5.4 Return structured unavailable when runtime, registry, kernel compatibility handle, or host backend is not configured.
- [x] 5.5 Ensure runtime-host provider does not branch on application names, workflow names, package names, provider names, or business-specific names.
- [x] 5.6 Add structured logs for provider start, discover, load, start, stop, remove, session, host dispatch, GenUI, failure, and snapshot emission.

## 6. SDK Focused Client

- [x] 6.1 Add `SystemApplicationClient` trait.
- [x] 6.2 Add a service-backed client over `SystemServiceClient`.
- [x] 6.3 Add an unavailable/null-object client for shells without configured Application Service.
- [x] 6.4 Add `SystemFacade::application_client()` accessor and only thin helper methods where useful.
- [x] 6.5 Validate SDK remains a client/facade layer and does not construct `AppRuntime`, `AppRegistry`, `Kernel`, Web state, provider, or application workflow.

## 7. Web Startup Migration

- [x] 7.1 Register and start Application Service during Web startup with explicit trace context.
- [x] 7.2 Add service-backed `application_client` to `AppState`.
- [x] 7.3 Use Application Service to discover/start YAML applications and obtain app ids, names, agent names, app dirs, skill dirs, lifecycle status, and diagnostics.
- [x] 7.4 Preserve existing auto-start, `started_apps`, executor registration, and skill directory behavior through service result views.
- [x] 7.5 Keep direct `AppRegistry` and `AppRuntime` fields only as deprecated compatibility anchors.
- [x] 7.6 Ensure one failed application reports diagnostics and does not block Web startup.

## 8. Web Routes and Chat Preflight Migration

- [x] 8.1 Migrate app list/detail/agents/status routes to `SystemApplicationClient` with compatibility serializers and fallback.
- [x] 8.2 Migrate `/api/apps/reload` discovery/reload/start to Application Service where possible.
- [x] 8.3 In `/api/chat/v2`, use Application Service for entry-agent resolution, app/session lifecycle preflight, executor readiness metadata, and session start/resume/stop envelope.
- [x] 8.4 Keep coordinator execution, framework runner, PlanLoop, WorkerLoop, EventLog persistence, RunTracer, resume signal, and SSE response shape unchanged.
- [x] 8.5 Migrate GenUI surface query to Application Service, falling back to current no-surface response when unavailable.
- [x] 8.6 Ensure prompt text is never written into Application Service logs or snapshots.

## 9. Governance and Allowlist

- [x] 9.1 Update Route C governance with Application Service ownership rules.
- [x] 9.2 Update serviceization allowlist with S7 migration state and remaining debt.
- [x] 9.3 Remove allowlist rows only when dependency gates prove direct Cargo edges are gone.
- [x] 9.4 Update dependency boundary tests for any allowlist changes. No test allowlist row changes were needed; boundary test passed with existing rules.

## 10. Verification

- [x] 10.1 Run `openspec validate add-application-framework-service-v1 --strict`.
- [x] 10.2 Run `cargo fmt --all --check`.
- [x] 10.3 Run `cargo test -p macaca-app application_abi`.
- [x] 10.4 Run `cargo test -p macaca-app package_manifest`.
- [x] 10.5 Run `cargo test -p macaca-app lifecycle`.
- [x] 10.6 Run `cargo test -p macaca-app runtime`.
- [x] 10.7 Run `cargo test -p macaca-runtime-host service_runtime`.
- [x] 10.8 Run `cargo test -p macaca-sdk application_client`.
- [x] 10.9 Run `cargo test -p macaca-web chat`.
- [x] 10.10 Run `cargo test -p macaca-web genui`.
- [x] 10.11 Run `cargo test -p macaca-integration-tests route_c_dependency_boundaries`.
- [x] 10.12 Run `cargo test -p macaca-integration-tests --test route_c_baseline`.
- [x] 10.13 Run `cargo check --workspace`.
- [x] 10.14 Run `npx gitnexus detect-changes -r agent --scope unstaged`. Result: MEDIUM, with noise from unrelated untracked external project directories and S7 Web/app route changes.
