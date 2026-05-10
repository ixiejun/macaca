# S9 Store / Entitlement 服务化实施计划

## Scope

Implement S9 from `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`: move Store / Entitlement from Phase 08 runtime helper contracts into provider-neutral system services compatible with `ServiceRuntime` and `SystemFacade`.

S9 covers:

- Store Service contract for package inspect/resolve/install/status/snapshot.
- Entitlement Service contract for query/upsert/revoke/authorize install/authorize start/authorize call/audit query/metering record/snapshot.
- Runtime-host service providers that adapt existing Phase 08 commerce contracts, `EntitlementStore`, `EntitlementRuntimeFacade`, app commercial guard, and encrypted package authorization seams.
- SDK focused clients: `SystemStoreClient`, `SystemEntitlementClient`, and service-backed `SystemPackageClient`.
- Web/CLI package-manager and entitlement-related upper consumers migrated to service-first paths.
- Deprecated compatibility anchors for direct `EntitlementRuntimeFacade`, direct package inspection helper, and direct entitlement guard calls.

S9 does not cover:

- Real payment provider integration, quote/intent/receipt settlement, or A2A payment lifecycle. That belongs to S10.
- Web3/EVM on-chain entitlement verification. That belongs to S11.
- Full marketplace UI, package recommendation, billing operations, or business-specific store workflows.
- Removing Phase 08 APIs before all consumers migrate and dependency gates prove direct edges can be removed.
- Adding a new `macaca-store` crate in this slice.

## Required Governance Inputs

- `macaca/docs/agent-os-microkernel-boundaries.md`
- `macaca/docs/route-c-serviceization-allowlist.md`
- `macaca/docs/route-c-architecture-governance.md`
- `macaca/docs/route-c-regression-matrix.md`
- `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`
- `docs/superpowers/plans/2026-05-10-s9-store-entitlement-serviceization-brainstorm.md`
- `openspec/changes/add-store-entitlement-v0/*`

## Architecture Decision

Use two focused services, not one commerce macro-service:

- `StoreService`: owns package source metadata, package inspect/resolve/install/status, and store snapshot.
- `EntitlementService`: owns entitlement state, authorization decisions, decision audit, metering records, and entitlement snapshot.

Design patterns:

- Facade: Store/Entitlement services and SDK clients hide repository/runtime details from Web/CLI/Application/Skill.
- Adapter / Bridge: Phase 08 `EntitlementRuntimeFacade`, `EntitlementStore`, `CommercialPackageGuard`, `EncryptedPackageAuthorizer`, and `PackageRuntimeGuard` are adapted behind service commands.
- Chain of Responsibility: install/start/call decisions flow through signature metadata, compatibility, entitlement state, license/subscription policy, metering, and audit.
- Strategy: source resolution, entitlement lookup, license policy, offline grace, metering, and encrypted package authorization remain replaceable.
- Command: all service operations are typed commands before `ServiceCommand` payload conversion.
- Specification: trace, package/developer scope, operation kind, entitlement state, metering fields, and redaction rules are validated centrally.
- Null Object: missing Store/Entitlement service returns structured unavailable, while free/open package paths remain allowed.
- Observer: every allow/deny/metering/install/status/audit node emits structured logs with trace ids and sanitized metadata.
- Memento: service snapshots expose counts/status/health and sanitized diagnostics, not raw package bodies or secrets.

Rejected alternatives:

- SDK helper only: rejected because it does not create a ServiceRuntime-owned boundary.
- Entitlement-only S9: rejected as incomplete because Store package source/install/status would remain unsystematized.
- New `macaca-store` crate: deferred to a future extraction phase; current code can implement service contracts in existing proto/runtime-host/sdk boundaries without expanding workspace complexity.
- Kernel-owned entitlement logic: rejected by microkernel boundaries because Store/Entitlement is replaceable service behavior.

## Proposed OpenSpec Change

Expected change id:

- `add-store-entitlement-services-v1`

Expected artifacts:

- `openspec/changes/add-store-entitlement-services-v1/proposal.md`
- `openspec/changes/add-store-entitlement-services-v1/design.md`
- `openspec/changes/add-store-entitlement-services-v1/tasks.md`
- `openspec/changes/add-store-entitlement-services-v1/specs/store-service/spec.md`
- `openspec/changes/add-store-entitlement-services-v1/specs/entitlement-service/spec.md`
- `openspec/changes/add-store-entitlement-services-v1/specs/store-entitlement-sdk-client/spec.md`
- `openspec/changes/add-store-entitlement-services-v1/specs/store-entitlement-consumer-migration/spec.md`

The proposal should state:

- S9 builds on `add-store-entitlement-v0`; it does not replace or delete Phase 08 guard code immediately.
- Store/Entitlement calls require trace context, package/developer scope, operation name, and policy/decorator admission through `ServiceRuntime` or equivalent SDK client boundary.
- Free/open package flows remain runnable without Store requirement.
- Paid/subscription/metered install/start/call paths return structured denied/unavailable states when entitlement is missing, expired, revoked, region blocked, usage exceeded, or service unavailable.
- Service snapshots and logs must not expose raw package payloads, encrypted package bytes, license secrets, API keys, private keys, credentials, prompt bodies, or raw manifest bodies.
- No app/workflow/provider/driver/gateway/model/chain/business-specific name can be hardcoded into service control flow.

## Implementation Slices

### Slice S9.1: Impact And Boundary Audit

Files to inspect before editing:

- `macaca/crates/macaca-proto/src/commerce.rs`
- `macaca/crates/macaca-persist/src/entitlement_store.rs`
- `macaca/crates/macaca-runtime-host/src/entitlement.rs`
- `macaca/crates/macaca-runtime-host/src/service_runtime.rs`
- `macaca/crates/macaca-runtime-host/src/service_provider.rs`
- `macaca/crates/macaca-app/src/commercial_package.rs`
- `macaca/crates/macaca-app/src/runtime_guard.rs`
- `macaca/crates/macaca-skill/src/encrypted_package.rs`
- `macaca/crates/macaca-sdk/src/package_client.rs`
- `macaca/crates/macaca-sdk/src/service_client.rs`
- `macaca/crates/macaca-sdk/src/system_facade.rs`
- `macaca/crates/macaca-web/src/lib.rs`
- `macaca/crates/macaca-web/src/routes.rs`
- `macaca/crates/macaca-cli/src/commands.rs`
- `macaca/crates/macaca-integration-tests/tests/route_c_dependency_boundaries/allowlist.rs`

Required actions:

1. Run GitNexus impact before modifying existing structs/functions/traits.
2. Classify every Store/Entitlement path as service contract, runtime-host provider, SDK client, app guard, skill guard, Web adapter, CLI adapter, or deprecated compatibility anchor.
3. Identify dependency allowlist rows that remain after S9 and any new temporary exceptions.
4. Confirm each touched Rust file stays under 500 lines; split DTO, provider, client, and admission logic before adding large code.
5. Warn before editing HIGH or CRITICAL impact symbols.

### Slice S9.2: Store / Entitlement Service DTOs In `macaca-proto`

Files:

- Add: `macaca/crates/macaca-proto/src/store_service.rs`
- Add: `macaca/crates/macaca-proto/src/entitlement_service.rs`
- Update: `macaca/crates/macaca-proto/src/lib.rs`

Behavior:

- Define service ids and command names:
  - `STORE_SERVICE_ID`
  - `store.package.inspect`
  - `store.package.resolve`
  - `store.package.install`
  - `store.package.status`
  - `store.snapshot`
  - `ENTITLEMENT_SERVICE_ID`
  - `entitlement.query`
  - `entitlement.upsert`
  - `entitlement.revoke`
  - `entitlement.authorize.install`
  - `entitlement.authorize.start`
  - `entitlement.authorize.call`
  - `entitlement.audit.query`
  - `entitlement.metering.record`
  - `entitlement.snapshot`
- Define typed commands/results:
  - `StorePackageInspectCommand`
  - `StorePackageResolveCommand`
  - `StorePackageInstallCommand`
  - `StorePackageStatusCommand`
  - `StoreSnapshotCommand`
  - `EntitlementQueryCommand`
  - `EntitlementUpsertCommand`
  - `EntitlementRevokeCommand`
  - `EntitlementAuthorizeInstallCommand`
  - `EntitlementAuthorizeStartCommand`
  - `EntitlementAuthorizeCallCommand`
  - `EntitlementAuditQueryCommand`
  - `EntitlementMeteringRecordCommand`
  - `EntitlementSnapshotCommand`
- Define sanitized views:
  - `StorePackageView`
  - `StoreInstallResult`
  - `StoreServiceSnapshot`
  - `EntitlementDecisionView`
  - `EntitlementAuditPage`
  - `EntitlementServiceSnapshot`
  - `StoreEntitlementUnavailable`

Rules:

- Mutating or authorization commands require `TraceContext`.
- Commands must carry package/developer scope and operation name where applicable.
- Capability-call commands should include application id, session id, capability id, quantity, and unit when available.
- DTOs must reuse Phase 08 commerce value objects instead of duplicating license/state semantics.
- Detailed English comments must explain provider-neutral boundaries and redaction rules.

### Slice S9.3: Service Admission Specifications

Files:

- Add: `macaca/crates/macaca-runtime-host/src/store_entitlement_admission.rs`

Behavior:

- Add small Specification objects:
  - `StoreTraceSpec`
  - `PackageScopeSpec`
  - `EntitlementOperationSpec`
  - `MeteringScopeSpec`
  - `CommerceRedactionSpec`
  - `FreeOpenFastPathSpec`
- Validate command payloads before provider dispatch reaches Phase 08 facade.
- Return structured unavailable/invalid request errors, not panic.

Rules:

- No provider/vendor/app/workflow hardcode.
- Logs include service id, command, trace id, package id, developer id, operation, state, and reason code.
- Logs must not include raw manifest body, package bytes, encrypted payload, credentials, API keys, private keys, or license secrets.

### Slice S9.4: Runtime-Host Entitlement Service Provider

Files:

- Add: `macaca/crates/macaca-runtime-host/src/entitlement_service_provider.rs`
- Update: `macaca/crates/macaca-runtime-host/src/lib.rs`

Behavior:

- Add `EntitlementSystemServiceProvider`.
- Translate `ServiceCommand` payloads into typed entitlement commands.
- Delegate:
  - query/upsert/revoke to `EntitlementStore`
  - authorize install/start/call to `EntitlementRuntimeFacade`
  - audit query to `EntitlementStore`
  - metering record to `EntitlementRuntimeFacade` or event/audit bridge
  - snapshot to sanitized repository/facade state
- Return structured unavailable when store/facade/event log is absent.
- Emit structured logs for provider start, stop, query, upsert, revoke, authorize, audit, metering, snapshot, failures.

Rules:

- Runtime-host provider owns service lifecycle orchestration, not commerce business semantics.
- Provider must not expose repository concrete type or `EntitlementRuntimeFacade` in DTOs.
- Preserve existing Phase 08 facade behavior behind the adapter.

### Slice S9.5: Runtime-Host Store Service Provider

Files:

- Add: `macaca/crates/macaca-runtime-host/src/store_service_provider.rs`
- Update: `macaca/crates/macaca-runtime-host/src/lib.rs`

Behavior:

- Add `StoreSystemServiceProvider`.
- Support package inspect/resolve/install/status/snapshot with provider-neutral metadata.
- Use existing package descriptors and compatibility guard where possible.
- For paid install/start, delegate to Entitlement Service or `EntitlementRuntimeFacade` adapter.
- For free/open install/status, return structured allow/status without requiring store entitlement.
- Model unknown or missing package source as structured unavailable/not found.

Rules:

- Store Service does not execute payment, driver, skill, MCP, application workflow, or Web3/EVM logic.
- Store Service must not read or return raw package body by default; it returns metadata and resource handles only.
- Existing YAML application behavior must not regress.

### Slice S9.6: SDK Focused Clients

Files:

- Add: `macaca/crates/macaca-sdk/src/store_client.rs`
- Add: `macaca/crates/macaca-sdk/src/entitlement_client.rs`
- Update: `macaca/crates/macaca-sdk/src/package_client.rs`
- Update: `macaca/crates/macaca-sdk/src/system_facade.rs`
- Update: `macaca/crates/macaca-sdk/src/lib.rs`

Behavior:

- Add `SystemStoreClient` and `SystemEntitlementClient` traits.
- Add service-backed clients over `SystemServiceClient`.
- Add unavailable/null-object clients.
- Upgrade `SystemPackageClient` so package inspection/install/status can delegate to Store Service instead of returning empty results where runtime-backed service exists.
- Add `SystemFacade::store_client()` and `SystemFacade::entitlement_client()` accessors.

Rules:

- SDK must not depend on `macaca-runtime-host`, `macaca-app`, `macaca-skill`, or provider concrete implementations for Store/Entitlement behavior.
- Missing service returns structured unavailable, not panic or hidden success.
- Client logs include command start/completion/failure with trace id, package id, developer id, and operation.

### Slice S9.7: App And Skill Guard Migration

Files:

- Update: `macaca/crates/macaca-app/src/commercial_package.rs`
- Update: `macaca/crates/macaca-app/src/runtime_guard.rs`
- Update: `macaca/crates/macaca-skill/src/encrypted_package.rs`

Behavior:

- Add service-backed authorizer adapters that use Entitlement Service client/command shape.
- Keep `ApplicationEntitlementAuthorizer` and `EncryptedPackageAuthorizer` traits as dependency-inversion seams.
- Mark direct `EntitlementRuntimeFacade` guard path as deprecated for new production use where a service-backed authorizer is available.
- Ensure encrypted package decrypt hook never runs when entitlement service denies or is unavailable for paid/encrypted packages.

Rules:

- Do not remove existing Phase 08 behavior.
- Free/open package behavior must remain unchanged.
- Paid/subscription/metered paths must be auditable and trace-backed.

### Slice S9.8: Web / CLI Consumer Migration

Files:

- Update: `macaca/crates/macaca-web/src/lib.rs`
- Update: `macaca/crates/macaca-web/src/state.rs`
- Update: `macaca/crates/macaca-web/src/routes.rs`
- Update: `macaca/crates/macaca-cli/src/commands.rs`

Behavior:

- Web startup registers and starts Store/Entitlement services when service runtime is available.
- Web package inspection/status/install routes, if present, prefer `SystemStoreClient`.
- Web entitlement/audit surfaces, if present, prefer `SystemEntitlementClient`.
- CLI package inspect/install/status and entitlement inspection commands prefer `SystemFacade` clients.
- Existing direct fallback paths remain deprecated compatibility anchors until S12 thin shell removes them.

Rules:

- Web/CLI remain shell adapters and must not define package/entitlement semantics.
- No application-specific package names or business workflow special cases.
- UI/API responses return structured unavailable/denied states for missing service or invalid entitlement.

### Slice S9.9: Governance And Dependency Gate Updates

Files:

- Update: `macaca/docs/route-c-architecture-governance.md`
- Update: `macaca/docs/route-c-serviceization-allowlist.md`
- Update: `macaca/crates/macaca-integration-tests/tests/route_c_dependency_boundaries/allowlist.rs` if dependency edges change.

Behavior:

- Add S9 Store / Entitlement Service Ownership section.
- Document remaining direct dependency debt and expiry conditions.
- Ensure no new presentation-shell provider-construction hub dependency is introduced.

Rules:

- Prefer deleting allowlist debt over extending it.
- Any new allowlist row must include replacement service/facade path and target migration phase.

### Slice S9.10: Verification

Commands:

```bash
openspec validate add-store-entitlement-services-v1 --strict
cargo fmt --all --check
cargo test -p macaca-proto store_service entitlement_service commerce
cargo test -p macaca-persist entitlement
cargo test -p macaca-runtime-host entitlement store_service entitlement_service service_runtime
cargo test -p macaca-app commercial_package package_manifest
cargo test -p macaca-skill encrypted_package
cargo test -p macaca-sdk package_client store_client entitlement_client
cargo test -p macaca-integration-tests route_c_dependency_boundaries
cargo test -p macaca-integration-tests --test route_c_baseline
cargo check --workspace
npx gitnexus detect-changes -r agent --scope unstaged
```

Expected regression coverage:

- `RC-APP-001`: YAML application loading remains available.
- `RC-SKILL-001`: skill/MCP smoke path remains available; encrypted skill entitlement hook is additive.
- `RC-TRACE-001`: entitlement allow/deny/metering events are trace/audit compatible.

## Rollback Plan

- Keep Phase 08 `EntitlementRuntimeFacade`, `CommercialPackageGuard`, `EncryptedPackageLoader`, and `EntitlementStore` behavior intact.
- If service-backed clients fail, disable Store/Entitlement service registration and fall back to deprecated Phase 08 guard paths for local compatibility.
- Revert Web/CLI service-first route changes without deleting DTOs/provider code if contract validation remains useful.
- Do not remove persisted entitlement/audit data during rollback.

## Completion Criteria

- OpenSpec proposal/design/tasks/spec for `add-store-entitlement-services-v1` validates strictly.
- Store and Entitlement services can be registered, started, called, stopped, and snapshotted through `ServiceRuntime`.
- SDK exposes service-backed and unavailable/null Store/Entitlement clients.
- Upper consumers use service-first Store/Entitlement paths, with deprecated direct paths retained only as compatibility anchors.
- Free/open package paths remain runnable without Store requirement.
- Paid/subscription/metered install/start/call paths are traceable, auditable, and deny/unavailable in structured form when entitlement is invalid or service is missing.
- No new provider/app/workflow/business hardcode is introduced.
- Route C dependency gate, targeted tests, workspace check, and GitNexus detect changes pass.
