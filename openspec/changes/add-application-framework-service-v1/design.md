## Context

S7 is the Route C slice that serviceizes Application Framework lifecycle. `macaca-app` already owns manifest loading, registry, runtime, ABI adapters, ApplicationHost, lifecycle state, and GenUI validation. `macaca-web` still directly discovers and starts applications and then uses discovered paths to bootstrap skills, executors, and chat orchestration. The new service boundary must reduce Web orchestration without breaking YAML applications, `/api/chat/v2`, task/goal resume, trace replay, or S6 Driver/Skill/MCP service-backed toolkit paths.

## Goals

- Make Application Framework lifecycle available through a provider-neutral Application Service.
- Keep `macaca-app` as the owner of application semantics.
- Let `macaca-runtime-host` host the service provider wrapper and lifecycle orchestration only.
- Let `macaca-sdk` expose a focused `SystemApplicationClient` for Web/CLI/Gateway shells.
- Make Web prefer service-backed discover/start/status/session preflight and GenUI surface lookup.
- Preserve current user-visible behavior and keep old direct APIs deprecated and searchable.
- Enforce traceable/auditable commands and sanitized snapshots.

## Non-Goals

- Do not move full `/api/chat/v2` coordinator execution into Application Service.
- Do not move PlanLoop, WorkerLoop, review, LLM, Memory, Driver, Skill, or MCP execution into Application Service.
- Do not implement Store/Entitlement distribution semantics.
- Do not implement real WASM execution; WASM remains metadata-only with structured runtime-unavailable.
- Do not remove compatibility fields or old direct runtime APIs before consumers fully migrate and dependency gates prove direct edges can disappear.

## Decisions

- Decision: Use a focused Application Service Facade.
  Alternatives considered: descriptor-only service, generic app/tool mega-service, full chat service. A focused facade is the smallest boundary that satisfies S7 without absorbing unrelated service responsibilities.

- Decision: Keep Application Framework semantics in `macaca-app`.
  Alternatives considered: moving `AppRuntime` into `macaca-runtime-host`. Rejected because runtime-host would become a macro-host; it should wrap providers and run service lifecycle, not own application semantics.

- Decision: Use Adapter/Bridge for existing `AppRuntime`, `AppRegistry`, `ApplicationHost`, YAML loader, WASM metadata adapter, and GenUI runtime.
  Rationale: S7 must be additive-first and preserve existing YAML behavior. Adapters allow serviceizing the boundary without rewriting application internals.

- Decision: Use Command DTOs for every lifecycle, session, host, and GenUI action.
  Rationale: command payloads can carry trace, application/session scope, policy hints, resource hooks, and sanitized results. This keeps remote service transport possible later.

- Decision: Treat ABI lifecycle state as service truth and project legacy `AppStatus` as a compatibility view.
  Rationale: `ApplicationLifecycleState` already models route-C application lifecycle; `AppStatus` is useful for existing routes but too small for service-level status.

- Decision: Use Specification objects for admission validation.
  Rationale: trace, manifest validity, runtime kind, application/session scope, and WASM unavailable behavior should be tested once instead of copied across Web, runtime, and loader paths.

- Decision: Use Null Object/unavailable results for missing runtimes and unsupported WASM execution.
  Rationale: optional capabilities must not panic or hang; unavailable must be explicit and auditable.

- Decision: Keep Web chat execution local for this phase, but make Web use Application Service for preflight.
  Rationale: moving chat execution crosses S4/S5/S12. S7 should only serviceize application lifecycle and session envelope while preserving `/api/chat/v2` semantics.

## Risks / Trade-offs

- Risk: `AppRuntime` still registers agents through kernel-compatible APIs.
  Mitigation: do not expose `Kernel` in service DTOs; isolate this behind runtime-host provider compatibility and mark direct runtime APIs deprecated. A later agent-registration service seam can remove this debt.

- Risk: Application Service becomes a macro-service.
  Mitigation: prohibit LLM/task/tool/driver/skill/MCP execution inside Application Service. It only owns lifecycle, session envelope, host command dispatch, and GenUI surface lookup.

- Risk: Web still needs app paths and skill dirs during transition.
  Mitigation: service start/snapshot returns sanitized app directory and skill directory views so Web consumes metadata instead of reinterpreting manifests.

- Risk: logging leaks prompts, raw manifests, host payloads, or secrets.
  Mitigation: logs and snapshots contain only ids, names, counts, runtime kind, lifecycle status, trace id, and diagnostics. No prompt body, raw manifest body, raw agent config, env, API key, secret, or full host payload.

- Risk: WASM metadata-only behavior conflicts with legacy `AppLoader` rejection.
  Mitigation: service path admits WASM metadata and returns runtime-unavailable for execution; legacy direct loader behavior remains deprecated compatibility until migration completes.

- Risk: Route C dependency gate still shows `macaca-web -> macaca-app`.
  Mitigation: keep allowlist debt documented until direct Cargo edges are actually gone; do not remove allowlist rows prematurely.

## Migration Plan

1. Add OpenSpec change and validate before code.
2. Add provider-neutral Application Service DTOs and service descriptor in `macaca-app`.
3. Add admission specifications and lifecycle/status projection helpers.
4. Add runtime-host provider adapter over existing application framework primitives.
5. Add SDK `SystemApplicationClient` and facade accessor.
6. Register Application Service during Web startup and add `AppState` client.
7. Migrate Web app routes, app reload, chat preflight, and GenUI surface lookup to service-first with compatibility fallback.
8. Mark direct old APIs/fields as deprecated but keep behavior unchanged.
9. Update governance/allowlist docs and dependency tests as required.
10. Run Route C verification matrix and update tasks with real results.

## Rollback

- Disable the Web service-first path and fall back to existing direct `AppRegistry`/`AppRuntime` startup.
- Keep direct APIs intact throughout the migration, so rollback does not require restoring deleted code.
- Revert SDK focused client and runtime-host provider registration without changing the underlying application runtime.

## Open Questions

- Whether Web should keep app path/skill directory metadata in `AppState` after service migration or request it lazily from Application Service.
- Whether application session envelope commands should emit EventLog records in S7 or defer complete session lifecycle ownership to S12.
- Whether dependency gate can remove any Web application allowlist edge in this slice; this must be decided from `cargo metadata`, not by assumption.

