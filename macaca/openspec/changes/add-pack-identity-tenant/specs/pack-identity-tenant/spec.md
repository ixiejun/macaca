## ADDED Requirements

### Requirement: Macaca SHALL provide Identity Tenant Pack as a serviceized capability

Macaca SHALL provide `pack.identity.tenant.v1` as a provider-neutral industrial
pack for tenant records, tenant identifiers, lifecycle state, isolation policy
references, quota envelopes, usage snapshots, residency hints, configuration
references, relationship references, tenant audit references, and artifact
handles. The pack SHALL be declared by applications, resolved by
admission/catalog services, and invoked only through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.identity.tenant.v1` as required and a tenant service provider is registered, healthy, entitled, permission-compatible, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, permission scopes, policy template hash, resource limits, approval rules, provider health metadata, compatibility metadata, documentation links, quota metadata, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing provider secrets, raw config values, raw provider payloads, raw manifests, full usage exports, or unbounded tenant data

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.identity.tenant.v1` as required but provider, permission, entitlement, resource, host support, quota support, policy support, or config support is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, contact another undeclared provider, provision resources, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.identity.tenant.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report with unavailable reason codes and command-level availability
- **AND** SDK command helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: Identity Tenant Pack SHALL expose supplier-grade tenant contracts

`pack.identity.tenant.v1` SHALL expose provider-neutral DTOs for tenant records,
tenant identifiers, lifecycle states, isolation policy references, quota
envelopes, usage snapshots, residency hints, configuration references,
relationship references, audit references, artifacts, version metadata,
freshness metadata, redaction metadata, and provider capability metadata.

#### Scenario: Provider schema is discovered
- **WHEN** SDK discovery or `tenant.discover_schema` inspects the pack
- **THEN** Macaca SHALL return field descriptors, command schemas, permission scopes, lifecycle states, identifier types, policy attachment shapes, quota dimensions, residency support, config reference support, relationship support, filter support, pagination support, version support, redaction profile, and compatibility hash
- **AND** the schema SHALL be provider-neutral even when backed by Microsoft, Auth0, Okta, Google, AWS, Azure, Kubernetes, SCIM/OIDC, built-in, plugin, remote, mock, or unavailable providers

#### Scenario: Tenant record is represented
- **WHEN** a provider returns a directory, tenant, customer, org boundary, cloud account, subscription, namespace, workspace, or internal isolation partition
- **THEN** Macaca SHALL map it to `TenantRecord` with stable handle, identifiers, lifecycle state, isolation policy references, quota envelope references, residency hints, config references, relationship references, version/freshness metadata, and bounded audit references
- **AND** Macaca SHALL NOT copy provider-specific cloud governance, billing, org-chart, product authorization, or application multitenancy workflow into OS semantics

#### Scenario: Tenant identifiers are normalized
- **WHEN** provider data includes tenant IDs, directory IDs, customer IDs, account IDs, subscription IDs, namespace names, issuer IDs, verified domains, aliases, slugs, or external IDs
- **THEN** Macaca SHALL represent them as `TenantIdentifier` values with uniqueness scope, verification metadata, source, freshness, and redaction class
- **AND** raw provider payloads and raw secrets SHALL remain excluded from traces, snapshots, SDK diagnostics, and examples

### Requirement: Identity Tenant Pack commands SHALL use canonical typed service calls

Every `tenant.*` operation SHALL be represented as a typed command/result DTO
and SHALL traverse the canonical service runtime path with trace, policy,
resource, entitlement, approval, health, snapshot, timeout, cancellation,
idempotency, redaction, and structured error behavior.

#### Scenario: Provider is inspected
- **WHEN** `tenant.inspect_provider` is invoked for a declared and policy-allowed pack
- **THEN** Macaca SHALL route the command through SDK/facade helpers into the service runtime and tenant service provider
- **AND** the result SHALL report provider class, lifecycle, command availability, tenant lifecycle support, policy support, quota support, residency support, config support, audit-export support, rate-limit state, health, and unavailable diagnostics without raw provider payloads

#### Scenario: Tenant is created
- **WHEN** `tenant.create` is invoked after `tenant.plan_create` validates identifiers, display label, relationship references, policy, quota, residency, entitlement, and provider capability
- **THEN** Macaca SHALL require an idempotency key, route the command through the canonical service path, return a typed tenant result or typed conflict/unavailable/denied result, and emit sanitized trace/audit events
- **AND** the SDK, shell, kernel, and generic application framework SHALL NOT construct concrete providers, provision cloud resources directly, or branch on provider names

#### Scenario: Tenant is searched
- **WHEN** `tenant.search` is invoked with filters and field masks
- **THEN** Macaca SHALL enforce permission, tenant/application scope, resource bounds, pagination limits, redaction, and provider capability before returning a bounded page
- **AND** the result SHALL include freshness and continuation metadata without exposing unbounded tenant lists or raw provider data

#### Scenario: Tenant lifecycle transition is rejected before side effects
- **WHEN** `tenant.request_lifecycle_transition` fails permission, entitlement, approval, version, lifecycle, dependency, resource, or policy validation
- **THEN** Macaca SHALL return a typed denied, approval-required, conflict, stale-version, quota, unavailable, or unsupported result before invoking the concrete provider
- **AND** the audit trail SHALL include only bounded reason codes, hashes, counters, and sanitized references

### Requirement: Identity Tenant Pack SHALL model isolation policy references without owning policy engines

`pack.identity.tenant.v1` SHALL expose tenant isolation policy references,
policy attachment plans, and policy attachment requests while policy evaluation
and enforcement remain behind Macaca policy/resource services and replaceable
providers.

#### Scenario: Isolation policy is inspected
- **WHEN** `tenant.inspect_isolation_policy` is invoked for a declared and policy-allowed tenant
- **THEN** Macaca SHALL return policy handles, policy types, decision freshness, attachment state, data boundary hints, separation constraints, and audit references
- **AND** Macaca SHALL NOT embed provider-specific policy engines, application feature-gating rules, billing rules, or cloud governance workflows in OS layers

#### Scenario: Policy attachment is planned
- **WHEN** `tenant.plan_policy_attachment` validates attaching or detaching a policy reference
- **THEN** Macaca SHALL check privilege, separation constraints, residency impact, resource impact, approval requirements, entitlement, provider support, and version preconditions
- **AND** no provider side effect SHALL occur during the plan command

#### Scenario: Policy attachment requires approval
- **WHEN** `tenant.request_policy_attachment` changes a high-impact policy reference
- **THEN** Macaca SHALL require approval when policy requires it, idempotency, stale-version handling, and audit evidence
- **AND** policy decision records SHALL contain references and bounded reason codes, not raw policy documents or raw provider payloads

### Requirement: Identity Tenant Pack SHALL model quotas and usage without owning billing entitlement

`pack.identity.tenant.v1` SHALL expose quota envelopes, quota reservations, and
usage snapshots as resource-policy evidence. It SHALL NOT grant licenses,
subscriptions, payment rights, billing plans, or product features.

#### Scenario: Quota is inspected
- **WHEN** `tenant.inspect_quota` is invoked
- **THEN** Macaca SHALL return quota dimensions, hard limits, soft limits, burst limits, reservation state, budget references, provider quota class, enforcement mode, usage class, and freshness metadata
- **AND** quota metadata SHALL be bounded and redacted so SDK diagnostics do not expose unbounded usage exports

#### Scenario: Quota reservation is planned
- **WHEN** `tenant.plan_quota_reservation` validates reserving or releasing quota
- **THEN** Macaca SHALL check resource budget, entitlement, policy, fairness, provider capability, current usage snapshot, timeout, and cancellation behavior
- **AND** no provider side effect SHALL occur during the plan command

#### Scenario: Quota reservation exceeds limit
- **WHEN** `tenant.request_quota_reservation` exceeds hard limit, soft limit, entitlement, resource budget, or provider quota
- **THEN** Macaca SHALL return a typed quota, denied, unavailable, or rate-limited result before mutating provider state
- **AND** Macaca SHALL NOT convert quota failure into billing entitlement, payment, subscription, or product feature logic

#### Scenario: Usage snapshot is requested
- **WHEN** `tenant.snapshot_usage` is invoked
- **THEN** Macaca SHALL return measured counters, measurement window, freshness, source, confidence, and redaction profile
- **AND** raw provider usage exports and unbounded metrics SHALL be excluded from traces, snapshots, and SDK diagnostics

### Requirement: Identity Tenant Pack SHALL expose residency and config references safely

`pack.identity.tenant.v1` SHALL expose residency hints and tenant configuration
references without owning data-plane movement, cloud provisioning, auth
provider implementation, or secret storage.

#### Scenario: Residency is inspected
- **WHEN** `tenant.inspect_residency` is invoked
- **THEN** Macaca SHALL return allowed regions, preferred regions, restricted regions, provider limitation references, data boundary policy references, and freshness metadata
- **AND** Macaca SHALL NOT move data, provision cloud resources, or override provider residency behavior inside this pack

#### Scenario: Config references are inspected
- **WHEN** `tenant.inspect_config` is invoked
- **THEN** Macaca SHALL return custom domain references, connection references, issuer references, authorization-server references, config handles, secret references, feature-flag references, redaction classes, version, and freshness metadata
- **AND** raw secrets, raw config payloads, client secrets, access tokens, refresh tokens, private keys, and signatures SHALL NOT be returned

#### Scenario: Config reference update is sensitive
- **WHEN** `tenant.update_config_reference` changes authentication, external connectivity, custom-domain, issuer, or secret-reference metadata
- **THEN** Macaca SHALL require policy approval when configured, idempotency, version preconditions, secret-reference validation, and sanitized audit evidence
- **AND** raw secret values SHALL remain owned by `pack.foundation.secrets-reference.v1`

### Requirement: Identity Tenant Pack SHALL support relationships, audit export, and artifact handles safely

`pack.identity.tenant.v1` SHALL support tenant relationship inspection, bounded
audit export, and artifact handle metadata for tenant, policy, quota, residency,
config, and lifecycle evidence while preventing observability leaks.

#### Scenario: Relationships are inspected
- **WHEN** `tenant.inspect_relationships` is invoked
- **THEN** Macaca SHALL return parent, child, peer, organization unit, account, subscription, namespace, workspace, or directory relationship references with provider class and version metadata
- **AND** Macaca SHALL NOT provision cloud resources, modify organization membership, or infer application business hierarchy from those references

#### Scenario: Audit export is requested
- **WHEN** `tenant.export_audit` is invoked for tenant-scoped evidence
- **THEN** Macaca SHALL enforce permission, entitlement, approval, resource bounds, retention policy, redaction profile, artifact size class, and provider capability
- **AND** the result SHALL return an artifact handle and replay pointers rather than raw unbounded provider audit payloads

#### Scenario: Artifact metadata is retrieved
- **WHEN** `tenant.get_artifact` is invoked for an audit/export artifact
- **THEN** Macaca SHALL return artifact id, content class, redaction state, retention deadline, size class, checksum/hash, and retrieval permissions
- **AND** raw provider payloads, raw audit exports, credentials, config secrets, manifests, package bytes, and unbounded usage data SHALL remain excluded from SDK diagnostics, traces, and snapshots

### Requirement: Identity Tenant Pack SHALL expose health, snapshots, and replayable evidence

`pack.identity.tenant.v1` SHALL expose descriptor metadata, service health,
command availability, provider capability hashes, policy template hashes, quota
hashes, snapshots, replay pointers, and sanitized audit events for all
operations.

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a `pack.identity.tenant.v1` snapshot
- **THEN** the snapshot SHALL include descriptor version, provider capability hash, command availability, provider health, policy template hash, quota envelope hash, resource counters, bounded tenant/policy/quota/config summary counts, artifact summaries, event cursors, and sanitized replay pointers
- **AND** it SHALL exclude raw credentials, client secrets, access tokens, refresh tokens, private keys, signatures, raw provider payloads, raw manifests, package bytes, raw audit exports, full usage exports, unbounded tenant lists, and unbounded output

#### Scenario: Trace replay inspects a command
- **WHEN** trace replay inspects any `tenant.*` command
- **THEN** replay SHALL prove declaration, admission, policy, resource, entitlement, approval when required, service runtime routing, provider class, result variant, and sanitized audit evidence
- **AND** replay SHALL NOT require provider-specific logs, raw provider responses, cloud-control-plane state, or application-specific workflow state

#### Scenario: Provider is unavailable
- **WHEN** the active provider is unavailable, disabled, retired, degraded, command-limited, lifecycle-limited, policy-limited, quota-limited, residency-limited, config-limited, audit-limited, or rate-limited
- **THEN** SDK discovery, health, snapshots, and command results SHALL expose structured diagnostics with stable reason codes
- **AND** Macaca SHALL NOT crash, hang, silently fall back, contact undeclared providers, provision resources, or fake success

### Requirement: Identity Tenant Pack implementation SHALL preserve Macaca boundaries

The `pack.identity.tenant.v1` implementation SHALL remain owned by tenant
service providers and service-runtime contracts. The microkernel, SDK, shells,
and generic application framework SHALL remain provider-neutral and free of
application-specific, supplier-specific, cloud-specific, quota-specific,
policy-specific, config-specific, or workflow-specific routing branches.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete Microsoft, Auth0, Okta, Google, AWS, Azure, Kubernetes, SCIM, OIDC, quota, policy, credential, or tenant provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.identity.tenant.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, capability hashes, quota hashes, and bounded result codes rather than provider-specific business branches

#### Scenario: Adjacent pack boundary is tested
- **WHEN** boundary tests exercise account lifecycle, profile fields, auth handoff, organization membership, billing entitlement, communication delivery, workflow approval, cloud provisioning, and application feature-gating scenarios
- **THEN** `pack.identity.tenant.v1` SHALL expose only references, quotas, policy decisions, or bounded tenant evidence for those concerns
- **AND** it SHALL NOT implement those adjacent pack behaviors internally

### Requirement: Identity Tenant Pack SHALL include detailed developer documentation

The implementation of `pack.identity.tenant.v1` SHALL include detailed
developer documentation under `docs/developer-packs/identity/tenant.md` and
SHALL link that documentation from SDK discovery metadata and the industrial
pack catalog index.

#### Scenario: Developer reads the pack guide
- **WHEN** a developer opens `docs/developer-packs/identity/tenant.md`
- **THEN** the guide SHALL explain purpose, non-goals, manifest declaration, required versus optional behavior, permission scopes, approval behavior, command DTOs, result DTOs, tenant records, identifiers, lifecycle states, policy references, quota envelopes, usage snapshots, residency hints, config references, relationship references, audit exports, artifacts, unavailable diagnostics, provider replacement, and operational limits
- **AND** examples SHALL use synthetic data and generic handles rather than provider names, credentials, raw config values, raw provider payloads, raw audit logs, unbounded usage data, application names, or business workflows

#### Scenario: Provider author reads conformance guidance
- **WHEN** a provider author reads the tenant pack documentation
- **THEN** the guide SHALL include a supplier/API mapping for Microsoft Entra/Graph tenants, Auth0 tenant settings, Okta org settings, Google Workspace customers/org units, AWS Organizations, Azure management groups/subscriptions, Kubernetes namespaces/resource quotas, SCIM, and OIDC concepts
- **AND** it SHALL include conformance checks for descriptor completeness, tenant/policy/quota/config scope validation, idempotency, version handling, quota enforcement, policy attachment validation, residency validation, config secret-reference handling, audit redaction, resource bounds, policy hooks, trace/audit events, unavailable behavior, snapshot/replay, and no raw payload leakage
