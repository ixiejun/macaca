# S7 Application Framework 服务化实施计划

## Scope

Implement S7 from `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`: move Application Framework lifecycle and application-facing host commands behind an Application Service boundary compatible with `ServiceRuntime` and `SystemFacade`.

S7 covers:

- Application Service contract for discover, load/admit, start, stop, remove, status, snapshot, session lifecycle envelope, host dispatch, and GenUI surface lookup.
- YAML application lifecycle through Application Service.
- WASM package/application metadata-only admission with structured runtime-unavailable execution.
- `ApplicationHost` command dispatch through Application Service.
- SDK `SystemApplicationClient`.
- Web startup/status/chat-preflight migration to service-backed application client.
- Deprecated compatibility anchors for direct `AppRuntime`, `AppLoader`, and Web direct registry/runtime usage.

S7 does not cover:

- Gateway serviceization. That belongs to S8.
- Store / entitlement full serviceization. That belongs to S9.
- Payment / A2A, Web3, and EVM phases.
- Moving all `/api/chat/v2` coordinator execution, PlanLoop, WorkerLoop, task review, LLM calls, Driver/Skill/MCP calls into Application Service.
- Removing legacy wrappers before dependency gates prove all callers migrated.

## Required Governance Inputs

- `macaca/docs/agent-os-microkernel-boundaries.md`
- `macaca/docs/route-c-serviceization-allowlist.md`
- `macaca/docs/route-c-architecture-governance.md`
- `macaca/docs/route-c-regression-matrix.md`
- `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`
- `docs/superpowers/plans/2026-05-08-s1-service-runtime-v1-plan.md`
- `docs/superpowers/plans/2026-05-08-s3-sdk-system-facade-convergence-plan.md`
- `docs/superpowers/plans/2026-05-09-s6-driver-skill-mcp-serviceization-plan.md`
- `docs/superpowers/plans/2026-05-09-s7-application-framework-serviceization-brainstorm.md`

## Architecture Decision

Use one focused Application Service with provider-neutral DTOs and a runtime-host adapter:

- `ApplicationService`: owns application discover/start/stop/status/snapshot/session envelope/host dispatch commands.
- `ApplicationSystemServiceProvider`: adapts existing `AppRegistry`, `AppRuntime`, `ApplicationHost`, ABI adapters, and lifecycle state to `ServiceRuntime`.
- `SystemApplicationClient`: SDK focused client used by Web/CLI/Gateway shells.
- Web remains a shell/adapter and must prefer `SystemApplicationClient` for application lifecycle and status.

Design patterns:

- Facade: `ApplicationService` and `SystemApplicationClient` hide application framework internals from shells.
- Adapter / Bridge: existing `AppRuntime`, `AppRegistry`, YAML loader, WASM metadata adapter, and `ApplicationHost` are adapted behind service commands.
- Abstract Factory: YAML, WASM metadata-only, headless, and future package-installed application runtimes are selected by runtime kind / ABI metadata, not by app name.
- Command: all lifecycle, session, host, and GenUI actions are typed commands before `ServiceCommand` payload conversion.
- Strategy: application admission, runtime kind handling, entry agent resolution, session policy, and host dispatch backend stay replaceable.
- State: `ApplicationLifecycle` / ABI lifecycle state is service truth; `AppStatus` remains a compatibility projection.
- Null Object: missing runtime or unsupported WASM execution returns structured unavailable.
- Observer: discover/start/stop/session/host/GenUI events emit structured logs with trace ids and sanitized metadata.
- Memento: service snapshots expose sanitized lifecycle and runtime metadata without dumping full manifests, prompts, or secrets.
- Specification: command constructors and provider admission validate trace, scope, runtime kind, manifest validity, compatibility, and permission hooks.

Rejected alternatives:

- Descriptor-only S7: rejected because Web would remain the application lifecycle coordinator.
- Move all chat/session execution into Application Service: rejected because it crosses S4/S5/S12 and risks creating a macro-service.
- Move `AppRuntime` semantics into runtime-host: rejected because Application Framework semantics belong to `macaca-app`; runtime-host should only wrap providers.
- Implement Store/package-installed application runtime in S7: rejected because Store/Entitlement belongs to S9.

## Proposed OpenSpec Change

Expected change id:

- `add-application-framework-service-v1`

Expected artifacts:

- `openspec/changes/add-application-framework-service-v1/proposal.md`
- `openspec/changes/add-application-framework-service-v1/design.md`
- `openspec/changes/add-application-framework-service-v1/tasks.md`
- `openspec/changes/add-application-framework-service-v1/specs/application-service/spec.md`
- `openspec/changes/add-application-framework-service-v1/specs/application-runtime-host-provider/spec.md`
- `openspec/changes/add-application-framework-service-v1/specs/application-sdk-client/spec.md`
- `openspec/changes/add-application-framework-service-v1/specs/application-web-adapter/spec.md`

The proposal should state:

- S7 is additive-first and preserves YAML application loading, `/api/chat/v2`, session trace, goal resume, driver/skill/MCP service-backed toolkit, app list/status routes, and no-network route C baseline.
- Direct `AppRuntime`, `AppLoader`, `AppRegistry`, and Web application startup paths remain as deprecated compatibility anchors until consumers fully migrate.
- Service calls require trace context, application/session scope where applicable, and policy/decorator admission through `ServiceRuntime` or equivalent SDK client boundary.
- WASM execution remains metadata-only and returns structured runtime-unavailable.
- Service snapshots must not expose prompt bodies, raw full manifests, secrets, API keys, env values, or raw host command payloads.
- No application name, workflow name, provider name, driver name, skill name, MCP server name, gateway name, chain name, or business-specific name can be hardcoded into service control flow.

## Implementation Slices

### Slice S7.1: Impact And Boundary Audit

Files to inspect before editing:

- `macaca/crates/macaca-app/src/runtime.rs`
- `macaca/crates/macaca-app/src/abi.rs`
- `macaca/crates/macaca-app/src/host.rs`
- `macaca/crates/macaca-app/src/lifecycle.rs`
- `macaca/crates/macaca-app/src/loader.rs`
- `macaca/crates/macaca-app/src/registry.rs`
- `macaca/crates/macaca-app/src/genui.rs`
- `macaca/crates/macaca-runtime-host/src/service_runtime.rs`
- `macaca/crates/macaca-runtime-host/src/service_provider.rs`
- `macaca/crates/macaca-sdk/src/service_client.rs`
- `macaca/crates/macaca-sdk/src/system_facade.rs`
- `macaca/crates/macaca-web/src/lib.rs`
- `macaca/crates/macaca-web/src/state.rs`
- `macaca/crates/macaca-web/src/routes.rs`
- `macaca/crates/macaca-web/src/chat_orchestrator.rs`
- `macaca/crates/macaca-web/src/genui_routes.rs`
- `macaca/crates/macaca-integration-tests/tests/route_c_dependency_boundaries/allowlist.rs`

Required actions:

1. Run GitNexus impact before modifying existing structs/functions/traits.
2. Classify every direct application path as service contract, runtime-host provider, SDK client, Web adapter, CLI adapter, kernel compat, test, or provider-internal.
3. Identify allowlist rows that remain after S7 and any new rows required by temporary compatibility.
4. Confirm every touched file remains under 500 lines; split `service_contract`, `service_adapter`, `application_client`, and provider files before adding large logic.
5. Warn before editing HIGH or CRITICAL impact symbols.

### Slice S7.2: Application Service Contract In `macaca-app`

Files:

- Add: `macaca/crates/macaca-app/src/service_contract.rs`
- Add or update: `macaca/crates/macaca-app/src/service_adapter.rs`
- Update: `macaca/crates/macaca-app/src/lib.rs`

Behavior:

- Define constants:
  - `APPLICATION_SERVICE_ID`
  - `application.discover`
  - `application.load`
  - `application.start`
  - `application.stop`
  - `application.remove`
  - `application.status`
  - `application.snapshot`
  - `application.session.start`
  - `application.session.resume`
  - `application.session.stop`
  - `application.host.dispatch`
  - `application.genui.surface`
- Define typed commands/results:
  - `ApplicationDiscoverCommand`
  - `ApplicationLoadCommand`
  - `ApplicationStartCommand`
  - `ApplicationStopCommand`
  - `ApplicationRemoveCommand`
  - `ApplicationStatusCommand`
  - `ApplicationSnapshotCommand`
  - `ApplicationSessionStartCommand`
  - `ApplicationSessionResumeCommand`
  - `ApplicationSessionStopCommand`
  - `ApplicationHostDispatchServiceCommand`
  - `ApplicationGenUiSurfaceCommand`
- Define sanitized views:
  - `ApplicationServiceAppView`
  - `ApplicationServiceAgentView`
  - `ApplicationServiceRuntimeView`
  - `ApplicationServiceSessionView`
  - `ApplicationServiceSnapshot`
  - `ApplicationServiceUnavailable`

Rules:

- Commands that mutate lifecycle or session state require `TraceContext`.
- Commands must carry explicit application/session scope where applicable.
- Snapshots must expose ids, names, counts, runtime kind, lifecycle state, status, entry agent, and diagnostics only.
- No prompt body, full manifest body, raw agent config, secrets, env, or host command payload dumps.
- Detailed English comments must explain why DTOs are provider-neutral and safe to expose.

### Slice S7.3: Admission Specification And Lifecycle Projection

Files:

- Add: `macaca/crates/macaca-app/src/service_admission.rs`
- Update: `macaca/crates/macaca-app/src/runtime.rs`
- Update: `macaca/crates/macaca-app/src/abi.rs`
- Update: `macaca/crates/macaca-app/src/lifecycle.rs`

Behavior:

- Add small Specification objects:
  - `ApplicationTraceSpec`
  - `ApplicationManifestSpec`
  - `ApplicationRuntimeKindSpec`
  - `ApplicationScopeSpec`
- Add projection helpers between `AppStatus` and `ApplicationLifecycleState`.
- Model WASM metadata-only start as structured runtime unavailable through service path.
- Mark direct `AppRuntime::start_app_from_file`, `AppRuntime::start_app`, and direct `AppLoader` startup path as deprecated where feasible, without deleting or changing old semantics.

Rules:

- Do not hardcode application names or workflow names.
- Do not remove current YAML behavior.
- Logs must include application id/name, runtime kind, trace id, command, status, and counts, but not raw manifest/prompt.

### Slice S7.4: Runtime-Host Application Service Provider

Files:

- Add: `macaca/crates/macaca-runtime-host/src/application_service_provider.rs`
- Update: `macaca/crates/macaca-runtime-host/src/lib.rs`
- Update: `macaca/crates/macaca-runtime-host/Cargo.toml`

Behavior:

- Add `ApplicationSystemServiceProvider`.
- Translate `ServiceCommand` payloads into typed application service commands.
- Delegate:
  - discovery/list to `AppRegistry`
  - YAML start/stop/remove/status to `AppRuntime`
  - ABI metadata to `ApplicationAbiAdapter`
  - host command dispatch to `ApplicationHost`
  - GenUI surface command to ApplicationHost/GenUI seam, returning structured unavailable if no app surface exists
- Return structured unavailable when app runtime, registry, kernel compatibility handle, or host backend is absent.
- Emit structured logs for provider start, discover, load, start, stop, session, host dispatch, GenUI, failures, and snapshot.

Rules:

- Runtime-host provider owns service lifecycle orchestration, not application semantics.
- Provider must not branch on app name, workflow name, package name, or business names.
- Provider must not expose `Kernel` through service DTOs.
- Keep all comments in English and explain runtime ownership / adapter behavior.

### Slice S7.5: SDK Focused Application Client

Files:

- Add: `macaca/crates/macaca-sdk/src/application_client.rs`
- Update: `macaca/crates/macaca-sdk/src/lib.rs`
- Update: `macaca/crates/macaca-sdk/src/system_facade.rs`
- Update: `macaca/crates/macaca-sdk/Cargo.toml`

Behavior:

- Add `SystemApplicationClient` trait.
- Add service-backed client over `SystemServiceClient`.
- Add unavailable/null-object client.
- Add `SystemFacade::application_client()` accessor and high-level helpers only when they remain thin wrappers.

Rules:

- SDK must not depend on `macaca-runtime-host`.
- SDK must not construct `AppRuntime`, `AppRegistry`, `Kernel`, Web state, provider, or application workflow.
- Missing service returns structured unavailable, not panic or hidden success.
- Log command start/completion/failure with trace id and app/session scope.

### Slice S7.6: Web Startup Migration

Files:

- Update: `macaca/crates/macaca-web/src/lib.rs`
- Update: `macaca/crates/macaca-web/src/state.rs`
- Update: `macaca/crates/macaca-web/src/service_runtime_client.rs`
- Add if useful: `macaca/crates/macaca-web/src/application_runtime_client.rs`

Behavior:

- Register and start `ApplicationSystemServiceProvider` during Web startup.
- Add service-backed `application_client` to `AppState`.
- Use Application Service to discover/start applications and obtain:
  - app ids/names
  - agent names
  - app directories
  - app skill directories
  - lifecycle/status snapshot
- Keep direct `AppRegistry` and `AppRuntime` fields as deprecated compatibility anchors.
- Ensure one failed app returns diagnostics and does not prevent Web from booting.

Rules:

- Preserve existing auto-start behavior for discovered YAML apps.
- Preserve current `started_apps`, executor registration, and skill directory semantics through service result views.
- Do not remove app registry routes before route migration is complete.

### Slice S7.7: Web Routes And Chat Preflight Migration

Files:

- Update: `macaca/crates/macaca-web/src/routes.rs`
- Update: `macaca/crates/macaca-web/src/chat_orchestrator.rs`
- Update: `macaca/crates/macaca-web/src/genui_routes.rs`

Behavior:

- Migrate app list/detail/agents/status routes to `SystemApplicationClient` with compatibility serializers.
- Use Application Service for `/api/apps/reload` discovery/reload/start where possible.
- In `/api/chat/v2`, use Application Service for:
  - entry agent resolution
  - app/session lifecycle preflight
  - executor readiness metadata
  - session start/resume/stop envelope
- Keep coordinator execution in framework runner for this phase.
- Migrate GenUI surface query to Application Service, falling back to current no-surface response when unavailable.

Rules:

- Preserve `/api/chat/v2` response shape and SSE session id event.
- Preserve session EventLog persistence, RunTracer, resume signal, task board, and existing cleanup behavior.
- Do not move prompt text into Application Service logs or snapshots.

### Slice S7.8: Governance And Allowlist

Files:

- Update: `macaca/docs/route-c-architecture-governance.md`
- Update: `macaca/docs/route-c-serviceization-allowlist.md`
- Update if needed: `macaca/crates/macaca-integration-tests/tests/route_c_dependency_boundaries.rs`

Behavior:

- Document Application Service ownership:
  - `macaca-app` owns Application Framework service contract and semantics.
  - `macaca-runtime-host` owns provider wrapper and service lifecycle.
  - `macaca-sdk` owns focused client.
  - Web/CLI are adapters only.
- Document remaining debt:
  - Web direct `macaca-app` dependency may remain for compatibility serializers and deprecated state.
  - AppRuntime direct kernel registration remains until a later agent/application registration service seam.
- Remove allowlist rows only when dependency gate proves direct Cargo edge is gone.

Rules:

- Any new allowlist exception must include replacement path, phase, expiry condition, and owner/status.

### Slice S7.9: Tests And Regression Verification

Required verification:

```bash
openspec validate add-application-framework-service-v1 --strict
cargo fmt --all --check
cargo test -p macaca-app application_abi
cargo test -p macaca-app package_manifest
cargo test -p macaca-app lifecycle
cargo test -p macaca-app runtime
cargo test -p macaca-runtime-host service_runtime
cargo test -p macaca-sdk application_client
cargo test -p macaca-web chat
cargo test -p macaca-web genui
cargo test -p macaca-integration-tests route_c_dependency_boundaries
cargo test -p macaca-integration-tests --test route_c_baseline
cargo check --workspace
npx gitnexus detect-changes -r agent --scope staged
```

Minimum regression scenarios:

- RC-APP-001: YAML application loads and registers agents.
- RC-CHAT-001: `/api/chat/v2` creates session and emits session id.
- RC-GOAL-001: goal/task/resume pipeline remains intact.
- RC-TRACE-001: service and session traces remain real-time.
- RC-SKILL-001 / RC-DRIVER-001: S6 service-backed toolkit paths remain unaffected.

## Execution Order

1. Create OpenSpec change `add-application-framework-service-v1` from this plan.
2. Validate OpenSpec before code.
3. Run GitNexus impact for existing symbols before edits.
4. Implement contract DTOs in `macaca-app`.
5. Implement admission specs and lifecycle projection.
6. Implement runtime-host provider.
7. Implement SDK client/facade accessor.
8. Register service in Web startup and add AppState client.
9. Migrate routes/chat preflight/GenUI surface in small compatibility-preserving patches.
10. Update governance and allowlist.
11. Run verification matrix.
12. Update OpenSpec tasks with actual results.
13. Do not archive until code/spec/gates are aligned and approved.

## Completion Definition

S7 is complete when:

- Application lifecycle commands are available through Application Service.
- Web startup starts YAML apps through Application Service first.
- Web app list/detail/agents/reload and chat preflight prefer `SystemApplicationClient`.
- WASM metadata-only applications return structured runtime-unavailable through the service path.
- Old direct application runtime APIs are deprecated/searchable but not deleted.
- Route C regression tests and dependency gates pass.
- Governance docs describe Application Service ownership and remaining debt.

