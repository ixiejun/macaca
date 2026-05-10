# Design: Store / Entitlement Services v1

## Context

Phase 08 established a Store / Entitlement runtime baseline:

- `macaca-proto::commerce` owns provider-neutral license, entitlement, subscription, metering, and commerce error contracts.
- `macaca-persist::EntitlementStore` owns entitlement persistence and deterministic precedence.
- `macaca-runtime-host::EntitlementRuntimeFacade` owns install/start/call authorization and metering emission.
- `macaca-app::CommercialPackageGuard` and `macaca-skill::EncryptedPackageLoader` provide application and skill guard seams.

S9 turns that baseline into system services that participate in `ServiceRuntime` lifecycle, trace-required dispatch, SDK focused clients, and Web/CLI shell migration.

## Goals

- Provide Store Service and Entitlement Service command/result DTOs in `macaca-proto`.
- Register Store and Entitlement service providers through `macaca-runtime-host`.
- Preserve Phase 08 behavior while marking direct helper paths deprecated for new production use.
- Expose service-backed SDK clients through `SystemFacade`.
- Move upper consumers to service-first paths without breaking free/open local development packages.
- Ensure every paid install/start/call path is traceable, auditable, and policy-checkable.

## Non-Goals

- No real payment settlement or A2A payment lifecycle.
- No concrete Store vendor integration.
- No raw package download/extraction transport.
- No Web3/EVM entitlement verification.
- No deletion of compatibility APIs.
- No application-specific or business-specific routing.

## Decisions

### Decision: Split Store Service and Entitlement Service

Store Service owns package source metadata and package lifecycle commands: inspect, resolve, install, status, snapshot.

Entitlement Service owns entitlement records and decisions: query, upsert, revoke, authorize install/start/call, audit query, metering record, snapshot.

This avoids a commerce macro-service while allowing one runtime-host factory or composition root to register both services.

### Decision: Use existing Phase 08 components behind adapters

Runtime-host providers adapt:

- `EntitlementStore` for entitlement query/upsert/revoke/audit.
- `EntitlementRuntimeFacade` for authorization and metering.
- `CommercialPackageGuard` and `EncryptedPackageAuthorizer` seams for app/skill guard migration.
- `PackageRuntimeGuard` and package descriptors for package install/status validation.

The service DTOs do not expose repository concrete types or runtime facade concrete types.

### Decision: Keep DTOs in `macaca-proto`

SDK and runtime-host both need command/result contracts without depending on each other. `macaca-proto` is the existing provider-neutral contract crate and already owns commerce value objects.

### Decision: SDK clients are the upper-consumer boundary

Web and CLI must call Store/Entitlement through `SystemFacade` focused clients, not runtime-host concrete providers. Missing service uses Null Object clients that return structured unavailable/empty status rather than panic or fake success.

### Decision: Trace and redaction are mandatory

Mutating and authorization commands require `TraceContext`. Logs and snapshots only contain bounded identifiers, state, operation, counts, trace id, reason code, and sanitized diagnostics.

Prohibited payloads include raw package bytes, encrypted package bytes, license secrets, API keys, private keys, credentials, prompt bodies, raw manifest bodies, and raw tool payloads.

## Patterns

- Facade: Store/Entitlement services and SDK clients hide lower-level repository/runtime details.
- Adapter / Bridge: Phase 08 helper APIs are adapted behind service commands.
- Chain of Responsibility: authorization flows through signature metadata, compatibility, entitlement state, license/subscription policy, metering, and audit.
- Strategy: source resolution, license policy, offline grace, metering, and entitlement lookup remain replaceable.
- Command: all operations use typed command DTOs before `ServiceCommand` dispatch.
- Specification: trace, scope, operation kind, metering, and redaction are validated centrally.
- Observer: allow/deny/metering/install/status/audit nodes emit structured logs.
- Null Object: unavailable clients/providers return structured unavailable.
- Memento: snapshots expose sanitized service state for Web/CLI and recovery.

## Risks / Trade-offs

- Risk: free/open packages are accidentally blocked by Store service unavailability.
  Mitigation: free/open fast-path is explicit and covered by tests.
- Risk: paid packages bypass service authorization through old helpers.
  Mitigation: mark direct paths deprecated, migrate known consumers, scan for deprecated use, and keep service-backed authorizers as default.
- Risk: Store Service grows into marketplace business logic.
  Mitigation: S9 only handles package metadata/source/status/install command and entitlement checks; payment, recommendation, billing, and marketplace operations are out of scope.
- Risk: service payloads leak secrets.
  Mitigation: DTOs and logs are sanitized by design; raw payload transfer is out of scope and must use future resource handles.
- Risk: runtime-host provider starts owning app/skill semantics.
  Mitigation: runtime-host owns service lifecycle and adapter orchestration only; app and skill crates keep their domain semantics.

## Migration Plan

1. Add OpenSpec and validate strict deltas.
2. Add proto DTOs and tests.
3. Add runtime-host admission specs and service providers.
4. Add SDK focused clients and SystemFacade accessors.
5. Migrate app/skill guard seams to service-backed authorizer adapters while retaining old traits.
6. Migrate Web/CLI surfaces to service-first clients.
7. Update Route C governance and dependency allowlist.
8. Run targeted tests, workspace check, dependency gate, and GitNexus detect changes.

## Rollback

- Disable Store/Entitlement service registration while keeping DTOs and SDK unavailable clients.
- Revert Web/CLI service-first call sites to deprecated Phase 08 compatibility paths.
- Keep persisted entitlement/audit records intact.
- Do not delete Phase 08 contracts or guard code during rollback.

## Open Questions

- Whether Store Service should expose package install as metadata-only in v1 or also support resource-handle based package acquisition. S9 should prefer metadata-only plus future resource handle seam.
- Whether audit query needs pagination in v1. The spec requires an audit page shape; implementation can start with deterministic in-memory paging.
