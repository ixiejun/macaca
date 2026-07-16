## 1. Research, Governance, And Scope

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design-pattern guidance, and the industrial catalog umbrella proposal before implementation.
- [x] 1.2 Record supplier/API findings for Microsoft Entra/Graph tenants, Auth0 tenant settings, Okta org settings, Google Workspace customers/org units, AWS Organizations, Azure management groups/subscriptions, Kubernetes namespaces/resource quotas, SCIM, and OIDC tenant/issuer semantics.
- [x] 1.3 Confirm boundary decisions with adjacent packs: account owns account lifecycle, profile owns profile fields, auth handoff owns login/token exchange, organization owns membership/invitations/role bindings, entitlement owns licensing, commerce owns billing, workflow owns approvals/reviews, and foundation owns secrets/config references.
- [x] 1.4 Inventory existing descriptors, SDK clients, identity services, policy/resource services, service-runtime decorators, artifact services, mock providers, and unavailable providers that can back tenant service implementation.
- [x] 1.5 Record GitNexus CRITICAL/HIGH findings as memo only before implementation commits, without letting advisory severity block this proposal track.

## 2. Contract, Descriptor, And Schema

- [x] 2.1 Define `pack.identity.tenant.v1` descriptor metadata for pack id, family, lifecycle, stability, command schemas, permissions, policy template, resource budget, approval rules, data governance, SDK metadata, docs link, compatibility, health, and diagnostics.
- [x] 2.2 Define provider-neutral DTOs for `TenantScope`, `TenantRecord`, `TenantIdentifier`, `TenantLifecycleState`, `TenantIsolationPolicyReference`, `TenantQuotaEnvelope`, `TenantUsageSnapshot`, `TenantResidencyHint`, `TenantConfigReference`, `TenantRelationshipReference`, `TenantAuditReference`, and `TenantArtifactHandle`.
- [x] 2.3 Define command DTOs for `tenant.inspect_provider`, `tenant.discover_schema`, `tenant.plan_create`, `tenant.create`, `tenant.get`, `tenant.search`, `tenant.plan_update`, `tenant.update`, `tenant.plan_lifecycle_transition`, `tenant.request_lifecycle_transition`, `tenant.inspect_isolation_policy`, `tenant.plan_policy_attachment`, `tenant.request_policy_attachment`, `tenant.inspect_quota`, `tenant.plan_quota_reservation`, `tenant.request_quota_reservation`, `tenant.snapshot_usage`, `tenant.inspect_residency`, `tenant.inspect_config`, `tenant.update_config_reference`, `tenant.inspect_relationships`, `tenant.export_audit`, and `tenant.get_artifact`.
- [x] 2.4 Define typed success, partial, approval-required, denied, unavailable, unsupported, conflict, stale-version, quota, rate-limited, timeout, cancelled, and failure result DTOs.
- [x] 2.5 Add descriptor hashing, schema-version compatibility, command-availability hashing, policy-template hashing, quota-envelope hashing, config-reference hashing, and redaction-profile hashing.
- [x] 2.6 Add unit tests for valid descriptors, rejected descriptors, missing command schemas, invalid permission scopes, unstable hashes, incompatible versions, quota schema mismatch, and redaction metadata.

## 3. Permission, Policy, Resource, Entitlement, And Approval

- [x] 3.1 Implement declaration validation for scopes: `identity.tenant.read`, `identity.tenant.search`, `identity.tenant.write`, `identity.tenant.lifecycle`, `identity.tenant.policy.read`, `identity.tenant.policy.write`, `identity.tenant.quota.read`, `identity.tenant.quota.reserve`, `identity.tenant.usage.read`, `identity.tenant.residency.read`, `identity.tenant.config.read`, `identity.tenant.config.write`, `identity.tenant.relationship.read`, `identity.tenant.audit.export`, and `identity.tenant.artifact.read`.
- [ ] 3.2 Implement policy checks for caller subject, application id, current tenant id, target tenant scope, command, requested fields, lifecycle transition, policy attachment class, quota dimension, residency hint, config sensitivity, approval state, resource budget, and entitlement state before provider calls.
- [ ] 3.3 Implement resource reservation for tenant count, policy attachment count, quota dimensions, reserved capacity, usage snapshot window, audit export size, pagination window, provider quota, network budget, timeout, retained artifacts, retained snapshots, and event volume.
- [ ] 3.4 Implement entitlement checks and structured unavailable/denied diagnostics for missing provider, disabled pack, missing permission, missing entitlement, missing policy feature, missing quota feature, missing residency feature, missing config feature, missing audit-export feature, and disabled host capability.
- [ ] 3.5 Implement approval behavior for tenant creation, tenant deletion/archive/restore, policy attachment changes, residency boundary changes, external custom-domain changes, quota limit changes, large usage exports, audit exports, and config references affecting authentication or external connectivity.
- [ ] 3.6 Add tests proving denied, unavailable, unsupported, quota, approval-required, conflict, stale-version, missing-entitlement, and config-secret-denied paths do not call concrete providers or emit side effects.

## 4. Service Runtime Provider Implementation

- [x] 4.1 Implement or bind tenant service provider behind the service runtime; do not construct providers from SDK, shell, kernel, or application code.
- [x] 4.2 Add a deterministic unavailable provider that returns complete descriptor metadata, health state, command availability, and typed unavailable/unsupported diagnostics.
- [x] 4.3 Add mock provider support for provider inspection, schema discovery, tenant lifecycle, policy attachment, quota inspection/reservation, usage snapshots, residency inspection, config references, relationship references, audit export, and artifact handle metadata.
- [ ] 4.4 Add lifecycle, health, snapshot, shutdown, timeout, cancellation, pagination, async audit export, idempotency, version precondition, stale-version diagnostics, conflict diagnostics, quota diagnostics, and rate-limit diagnostics.
- [ ] 4.5 Add Strategy implementations for provider adapters, tenant hierarchy mapping, quota behavior, policy attachment behavior, residency support, config-reference behavior, audit-export behavior, artifact behavior, redaction, and unavailable behavior.
- [ ] 4.6 Add explicit state machines for tenant lifecycle, policy attachment, quota reservation, usage snapshot freshness, config reference updates, audit exports, and provider lifecycle states.
- [ ] 4.7 Add side-effect safety support for idempotency keys, provider state validation, policy attachment preconditions, quota reservation rollback/release, residency-change approval, config secret-reference validation, and non-mutating plan commands.
- [ ] 4.8 Add provider capability reporting for available, degraded, preview, unavailable, unsupported, retired, lifecycle-limited, policy-limited, quota-limited, residency-limited, config-limited, audit-limited, and rate-limited states.

## 5. SDK, Admission, ABI, And Examples

- [x] 5.1 Extend SDK discovery for `pack.identity.tenant.v1` with command schemas, permission scopes, field masks, filter support, pagination support, lifecycle support, policy attachment support, quota dimensions, residency support, config-reference support, audit-export support, examples, availability, diagnostics, documentation link, provider class, compatibility hash, quota hash, and redaction profile.
- [x] 5.2 Extend application admission so required declarations block readiness when unavailable and optional declarations degrade explicitly with effective capability mementos.
- [x] 5.3 Add SDK command helper builders for all `tenant.*` commands; helpers must only build canonical traced service calls and must never construct providers, hold credentials, call provider APIs directly, evaluate product authorization, mutate account/profile/organization state, provision cloud resources, create billing entitlements, or bypass policy.
- [ ] 5.4 Extend WASM/app ABI descriptors so applications can discover tenant commands, declare permissions, receive unavailable diagnostics, and submit typed service calls through the canonical execution path.
- [x] 5.5 Add generic app-facing examples for tenant creation, tenant read/search, lifecycle transition, policy inspection/attachment, quota inspection/reservation, usage snapshot, residency inspection, config reference update, relationship inspection, audit export, and unavailable diagnostics.
- [x] 5.6 Add provider-unavailable, missing-permission, missing-entitlement, policy-unsupported, quota-unsupported, residency-unsupported, stale-version, approval-required, quota-exceeded, config-secret-denied, audit-export-denied, and artifact-denied examples that avoid provider names, credentials, raw config values, raw provider payloads, raw audit logs, and application business workflows.

## 6. Trace, Audit, Replay, And Boundary Gates

- [ ] 6.1 Emit sanitized declaration, admission, discovery, policy, resource, entitlement, approval, service-call, tenant lifecycle, policy attachment, quota reservation, usage snapshot, residency inspection, config reference, relationship inspection, audit-export, artifact, health, snapshot, unavailable, conflict, and failure events.
- [ ] 6.2 Ensure traces, audits, snapshots, SDK diagnostics, and examples exclude raw credentials, client secrets, access tokens, refresh tokens, private keys, signatures, raw provider payloads, raw manifests, package bytes, raw audit exports, full usage exports, unbounded tenant lists, and unbounded output.
- [x] 6.3 Add replay tests proving every `tenant.*` command is trace-addressable through the canonical service path and snapshots contain enough bounded metadata for recovery diagnostics.
- [x] 6.4 Add dependency gates proving kernel, SDK, shells, and generic application framework do not import concrete Microsoft, Auth0, Okta, Google, AWS, Azure, Kubernetes, SCIM, OIDC, quota, policy, credential, or tenant provider adapters.
- [ ] 6.5 Add no-direct-provider-call gates proving SDK helpers, WASM ABI handlers, app admission, web, CLI, and frontend paths route through descriptor-owned service commands.
- [x] 6.6 Add boundary tests proving optional provider absence returns structured unavailable diagnostics and never crashes, hangs, silently falls back, creates tenants, changes policy, reserves quota, mutates config, exports audits, provisions cloud resources, contacts providers, or fakes success.
- [ ] 6.7 Run `openspec validate add-pack-identity-tenant --strict`, targeted cargo tests, boundary gates, file-size gates, and audit replay checks before marking implementation complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/identity/tenant.md` with purpose, capability model, manifest declaration, required versus optional behavior, permissions, tenant records, identifiers, lifecycle states, policy references, quota envelopes, usage snapshots, residency hints, config references, relationship references, audit exports, artifacts, unavailable diagnostics, provider replacement, and operational limits.
- [x] 7.2 Document all command DTOs and result DTOs with field-level explanations, idempotency semantics, pagination behavior, version preconditions, freshness metadata, quota semantics, redaction behavior, approval behavior, artifact retention behavior, and structured error codes.
- [x] 7.3 Document supplier/API mapping: Microsoft Entra/Graph tenants, Auth0 tenant settings, Okta org settings, Google Workspace customers/org units, AWS Organizations, Azure management groups/subscriptions, Kubernetes namespaces/resource quotas, SCIM, and OIDC concepts mapped to Macaca abstractions, including what is intentionally not exposed as OS semantics.
- [x] 7.4 Add generic examples for required declaration, optional declaration, tenant creation, lifecycle transition, policy attachment, quota reservation, usage snapshot, residency inspection, config reference update, audit export, artifact inspection, unavailable provider, denied permission, conflict, and stale-version handling.
- [x] 7.5 Add conformance checklist and test guidance for provider authors: descriptor completeness, tenant/policy/quota/config scope validation, idempotency, version handling, quota enforcement, policy attachment validation, residency validation, config secret-reference handling, audit redaction, resource bounds, policy hooks, trace/audit events, unavailable behavior, snapshot/replay, and no raw payload leakage.
- [x] 7.6 Cross-link the guide from SDK discovery metadata and the industrial pack catalog index before marking `add-pack-identity-tenant` complete.
