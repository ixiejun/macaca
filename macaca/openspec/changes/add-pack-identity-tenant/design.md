# Identity Tenant Pack Design

## Context

`pack.identity.tenant.v1` is a child proposal of the developer-pack industrial
capability catalog. It exposes a serviceized tenancy surface for developers:
tenant records, tenant identifiers, lifecycle, isolation policy references,
quota envelopes, usage snapshots, residency hints, config references,
relationship references, and tenant audit artifacts.

Tenant concepts are overloaded across identity providers, SaaS platforms, cloud
providers, and infrastructure control planes. A tenant may be a directory,
customer, organization boundary, cloud account, subscription, namespace,
workspace, or internal isolation partition. Macaca must offer a provider-neutral
contract that applications can declare and invoke without hardcoding provider
names, business workflows, or cloud-specific semantics into OS layers.

## Supplier Capability Matrix

| Supplier or platform | Relevant capability | Macaca interpretation |
| --- | --- | --- |
| Microsoft Entra ID / Graph | Organization resource, tenant id, verified domains, tenant-aware app issuers, directory roles | Tenant record, identifier, verified-domain reference, issuer/audience reference; roles stay as policy/organization references |
| Auth0 | Tenant settings, logs, custom domains, connections, organizations | Tenant configuration references, domain references, audit references; login and organization membership stay adjacent |
| Okta | Org settings, domains, authorization servers, groups, roles, policies, logs | Tenant administrative boundary, policy references, config references, audit references; group membership stays organization-side |
| Google Workspace / Cloud Identity | Customers, organizational units, groups, directory metadata | Tenant/customer record, hierarchy references, directory references; group membership stays organization-side |
| AWS Organizations | Accounts, OUs, service control policies, delegated admin, tags | Tenant/account records, hierarchy references, policy attachments, quota/budget references; cloud provisioning is non-goal |
| Azure management groups / subscriptions | Management hierarchy, subscriptions, policy assignments, resource groups | Tenant/resource-scope references, policy attachments, budget/quota references; Azure-specific resource management is non-goal |
| Kubernetes namespaces / ResourceQuota | Namespaces, scoped resources, resource quotas, admission constraints | Compact model for isolation scope, quota envelope, resource reservation, admission diagnostics |
| SCIM / OIDC | External IDs, issuer/audience boundaries, schema metadata | Interoperable identifiers, schema versioning, tenant issuer references, and provider-neutral metadata |

## Goals

- Provide stable pack id `pack.identity.tenant.v1` and command namespace
  `tenant.*`.
- Normalize tenant records, identifiers, lifecycle, policy references, quota
  envelopes, usage snapshots, residency hints, config references, relationship
  references, audit references, and artifact handles.
- Support provider inspection, schema discovery, planning commands, mutating
  commands, read/search commands, quota reservation, policy attachment, usage
  snapshots, and artifact retrieval through typed command/result DTOs.
- Preserve a single canonical execution path through SDK/facade clients,
  service runtime decorators, and replaceable tenant service providers.
- Return structured `success`, `partial`, `approval_required`, `denied`,
  `unavailable`, `unsupported`, `conflict`, `stale_version`,
  `quota_exceeded`, `rate_limited`, `timeout`, `cancelled`, and `failure`
  results.
- Emit sanitized trace, audit, health, snapshot, and replay evidence for every
  declaration, admission, policy decision, service call, provider decision, and
  unavailable state.
- Require detailed developer documentation at
  `docs/developer-packs/identity/tenant.md`.

## Non-Goals

- No account lifecycle or profile management.
- No login, token exchange, hosted auth, callback validation, or session store.
- No organization membership, invitation, or role-binding management.
- No billing entitlement, payment, subscription, invoice, receipt, or commerce
  policy.
- No cloud resource provisioning, Kubernetes controller behavior, workspace
  product workflow, HRIS workflow, or application-specific multitenancy logic.
- No raw secrets or credentials; tenant configuration stores only references to
  secrets, config, and policy handles.
- No provider-name routing or concrete provider construction in kernel, SDK,
  shells, or the generic application framework.

## Ownership And Boundaries

- Pack id: `pack.identity.tenant.v1`.
- Family: `identity`.
- Backing service owner: replaceable tenant service provider.
- SDK surface: `sdk.packs.identity.tenant`.
- Command namespace: `tenant.*`.
- Microkernel ownership: abstract tenant identity handles, policy facade,
  resource facade, service-call evidence, trace/audit primitives, and scheduler
  primitives only.
- Application framework ownership: manifest declaration, app-scoped permission
  declarations, lifecycle projection, and effective capability mementos.
- Runtime-host ownership: provider registration, service runtime decorators,
  transport adapters, health/snapshot bridge, and unavailable/mock provider
  composition through approved composition roots.

## Command Surface

All commands carry trace context, application/session/task/tenant identifiers
when available, policy context, idempotency key for side effects, redaction
profile, resource budget, and replay metadata.

| Command | Purpose | Notes |
| --- | --- | --- |
| `tenant.inspect_provider` | Return provider capability metadata | Reports tenant lifecycle, policy support, quota support, residency support, config support, audit support, rate limits, and unavailable reasons |
| `tenant.discover_schema` | Return tenant schema and command metadata | Exposes field descriptors, lifecycle states, quota dimensions, policy attachment shape, filters, pagination, and redaction profile |
| `tenant.plan_create` | Validate tenant creation without side effects | Checks identifiers, display labels, parent/relationship references, policy, quota, residency, entitlement, and provider support |
| `tenant.create` | Create a tenant record or isolation boundary | Requires idempotency, approval when high impact, conflict handling, and audit evidence |
| `tenant.get` | Read one tenant record | Returns minimized tenant fields with version/freshness metadata |
| `tenant.search` | Search/list tenants | Requires bounded pagination, filters, field masks, tenant/app scoping, and redaction |
| `tenant.plan_update` | Validate tenant metadata/config update | Checks version preconditions, immutable fields, identifier changes, residency changes, and policy |
| `tenant.update` | Patch tenant metadata or references | Excludes raw secrets and provider-specific business settings |
| `tenant.plan_lifecycle_transition` | Validate suspend, reactivate, archive, restore, or delete | Checks state machine, approval, dependent resources, and provider support |
| `tenant.request_lifecycle_transition` | Request a lifecycle transition | Requires idempotency, approval, stale-version handling, and audit evidence |
| `tenant.inspect_isolation_policy` | Inspect isolation policy references | Returns policy handles, decision freshness, data boundary hints, and effective state |
| `tenant.plan_policy_attachment` | Validate policy attach/detach without side effects | Checks privilege, separation constraints, residency, resource impact, and approval |
| `tenant.request_policy_attachment` | Attach or detach tenant policy references | Requires idempotency and policy/audit evidence |
| `tenant.inspect_quota` | Inspect quota envelopes and limits | Returns dimensions, hard/soft limits, usage class, provider quota state, and budget references |
| `tenant.plan_quota_reservation` | Validate quota reservation before side effects | Checks resource budget, entitlement, policy, fairness, and provider support |
| `tenant.request_quota_reservation` | Reserve or release quota envelope capacity | Requires idempotency, timeout/cancellation behavior, and replay metadata |
| `tenant.snapshot_usage` | Return bounded usage snapshot | Reports usage counters, measurement window, freshness, and redaction state |
| `tenant.inspect_residency` | Inspect region/residency hints | Returns allowed regions, data boundary references, provider limitations, and policy decisions |
| `tenant.inspect_config` | Inspect tenant config references | Returns config handles, custom domain references, connection references, issuer references, and secret references without raw secret values |
| `tenant.update_config_reference` | Update tenant config references | Requires approval when sensitive or externally visible |
| `tenant.inspect_relationships` | Inspect parent/child/peer references | Returns hierarchy references such as org unit, account, subscription, namespace, or workspace relation without owning those systems |
| `tenant.export_audit` | Request bounded tenant audit export | Returns artifact handle with retention/redaction metadata |
| `tenant.get_artifact` | Retrieve audit/export artifact metadata | Does not expose raw provider payloads or unbounded logs |

## Provider-Neutral DTO Model

- `TenantScope`: application id, tenant id, caller subject, provider reference,
  parent relationship reference, and trace context.
- `TenantRecord`: stable tenant handle, display label, identifiers, lifecycle
  state, isolation policy references, quota envelope references, residency
  hints, config references, relationship references, version, freshness, and
  audit references.
- `TenantIdentifier`: provider id, directory id, customer id, account id,
  subscription id, namespace, issuer id, verified domain, alias, slug, external
  id, or display label with uniqueness scope and verification metadata.
- `TenantLifecycleState`: planned, active, suspended, locked, disabled,
  archived, pending_delete, deleted, degraded, unavailable, provider_unknown.
- `TenantIsolationPolicyReference`: opaque policy handle, policy type,
  decision freshness, attachment state, data boundary hints, separation
  constraints, and audit reference.
- `TenantQuotaEnvelope`: quota handle, dimension, hard limit, soft limit,
  burst limit, reservation state, budget reference, provider quota class, and
  enforcement mode.
- `TenantUsageSnapshot`: measured counters, measurement window, freshness,
  source, redaction profile, and confidence.
- `TenantResidencyHint`: allowed regions, preferred regions, restricted
  regions, provider limitation references, data boundary policy references, and
  freshness.
- `TenantConfigReference`: custom domain reference, connection reference,
  issuer reference, authorization-server reference, config handle, secret
  reference, feature-flag reference, and redaction class.
- `TenantRelationshipReference`: parent, child, peer, organization unit,
  account, subscription, namespace, workspace, or directory relationship with
  provider class and version metadata.
- `TenantAuditReference`: bounded event reference, provider event cursor,
  export artifact handle, redaction profile, and retention metadata.
- `TenantArtifactHandle`: artifact id, content class, redaction state,
  retention deadline, size class, checksum/hash, and retrieval permissions.

## Permission, Policy, Resource, Entitlement, And Approval Model

Initial permission scopes:

- `identity.tenant.read`
- `identity.tenant.search`
- `identity.tenant.write`
- `identity.tenant.lifecycle`
- `identity.tenant.policy.read`
- `identity.tenant.policy.write`
- `identity.tenant.quota.read`
- `identity.tenant.quota.reserve`
- `identity.tenant.usage.read`
- `identity.tenant.residency.read`
- `identity.tenant.config.read`
- `identity.tenant.config.write`
- `identity.tenant.relationship.read`
- `identity.tenant.audit.export`
- `identity.tenant.artifact.read`

Policy checks run before side effects and before provider calls that could
reveal sensitive tenant metadata. Policy inputs include caller subject,
application id, current tenant id, target tenant scope, command, requested
fields, lifecycle transition, policy attachment class, quota dimension,
residency hint, config sensitivity, approval state, resource budget, and
entitlement state.

Approval is required for high-impact operations such as tenant creation,
tenant deletion/archive/restore, policy attachment changes, residency boundary
changes, external custom-domain changes, quota limit changes, large usage
exports, audit exports, and config references that affect authentication or
external connectivity.

Resource checks cover tenant count, policy attachment count, quota dimensions,
reserved capacity, usage snapshot window, audit export size, pagination window,
provider quota, network budget, timeout, retained artifacts, retained
snapshots, and event volume.

Entitlement checks determine whether the calling application/tenant may use the
pack, requested commands, policy attachment features, quota reservation,
residency inspection, config-reference mutation, audit export, and artifact
retrieval. Missing entitlement returns structured `unavailable` or `denied`
diagnostics rather than provider fallback.

## Service Runtime And Provider Strategy

The tenant service provider is a Strategy behind the service runtime. The
runtime composes provider adapters, unavailable providers, mock providers,
policy decorators, resource decorators, entitlement decorators, metering,
redaction, trace, audit, timeout/cancellation, and health/snapshot behavior.

Provider adapters may target Microsoft Entra/Microsoft Graph, Auth0, Okta,
Google Workspace/Cloud Identity, AWS Organizations, Azure management scopes,
Kubernetes namespace/quota providers, SCIM/OIDC-backed directories, built-in
local providers, remote providers, plugin providers, or mock providers.
Provider-specific capabilities are descriptor data, not OS routing branches.

The unavailable provider is first-class. It exposes descriptor metadata, health
state, unsupported command diagnostics, and stable error DTOs without crashing,
hanging, silently falling back, contacting undeclared providers, or faking
success.

## State, Consistency, And Idempotency

Tenants, policy attachments, quota reservations, usage snapshots, config
references, relationship references, and audit exports have explicit lifecycle
or freshness states. Mutating commands require idempotency keys and version
preconditions when provider support exists. When a provider has eventual
consistency, the result must include freshness, provider_state, replay cursor,
and partial/async status rather than pretending the state is immediately final.

Quota reservations are separate from billing entitlement. A quota reservation
proves resource availability inside Macaca policy/resource constraints; it does
not grant a license, subscription, payment right, or product feature.

## SDK Discovery And Developer Documentation

SDK discovery must return pack metadata, command schemas, permission scopes,
field masks, filter support, pagination support, lifecycle support, policy
attachment support, quota dimensions, residency support, config-reference
support, audit-export support, examples, availability, diagnostics, provider
class, compatibility hash, redaction profile, and documentation link.

SDK helper builders only build canonical traced service calls. They must never
construct providers, hold credentials, call provider APIs directly, evaluate
product authorization, mutate account/profile/organization state, provision
cloud resources, create billing entitlements, or bypass policy.

Developer documentation at `docs/developer-packs/identity/tenant.md` must cover:

- Capability purpose and non-goals.
- Manifest declaration examples for required and optional usage.
- Permission scopes and approval behavior.
- Command DTOs and result DTOs with field-level explanations.
- Tenant record, identifier, lifecycle, policy reference, quota envelope,
  usage snapshot, residency, config reference, relationship, audit, artifact,
  version, and freshness models.
- Supplier/API mapping and provider replacement guidance.
- Unavailable/denied/conflict/stale-version/quota diagnostics.
- Trace/audit events, redaction rules, snapshot/replay behavior, and
  conformance checklist for provider authors.

## Trace, Audit, Health, Snapshot, And Replay

Required event families:

- `tenant_pack_declared`
- `tenant_pack_admission_validated`
- `tenant_pack_discovery_requested`
- `tenant_pack_policy_decision`
- `tenant_pack_resource_reserved`
- `tenant_pack_approval_required`
- `tenant_pack_service_call_requested`
- `tenant_pack_service_call_succeeded`
- `tenant_pack_service_call_failed`
- `tenant_pack_unavailable`
- `tenant_pack_conflict_detected`
- `tenant_pack_snapshot_recorded`
- `tenant_pack_audit_export_requested`

Events include pack id, descriptor version, command name, trace id,
application/session/task/tenant identifiers when available, target tenant handle
hash, policy decision, approval state, provider class, latency, bounded
resource counters, capability hash, quota dimension, and bounded error code.

Events, snapshots, SDK diagnostics, and examples must exclude raw credentials,
client secrets, access tokens, refresh tokens, private keys, signatures, raw
provider payloads, raw manifests, package bytes, raw audit exports, full usage
exports, unbounded tenant lists, and unbounded output.

Snapshots include descriptor version, provider capability hash, command
availability, provider health, policy template hash, quota envelope hash,
resource counters, bounded tenant/policy/quota/config summary counts, artifact
summaries, event cursors, and sanitized replay pointers.

## Design Patterns

- **Facade**: `SystemFacade` and focused SDK clients expose discovery and typed
  command builders while hiding service runtime and provider composition.
- **Command**: every operation is represented as a typed command/result DTO
  with explicit success, partial, denied, unavailable, unsupported, conflict,
  stale-version, quota, approval-required, timeout, cancelled, and failure
  variants.
- **Adapter/Bridge**: Microsoft, Auth0, Okta, Google, AWS, Azure, Kubernetes,
  SCIM/OIDC, built-in, plugin, remote, mock, and unavailable providers adapt
  into the same provider-neutral contract.
- **Strategy**: provider selection, tenant hierarchy mapping, quota behavior,
  policy attachment behavior, residency support, config-reference behavior,
  audit-export behavior, and unavailable behavior are replaceable.
- **Decorator**: trace, audit, policy, resource, entitlement, approval,
  metering, timeout, cancellation, and redaction wrap every service call.
- **State**: tenant lifecycle, policy attachment, quota reservation, usage
  snapshot, config reference, audit export, and provider lifecycle states are
  explicit and replayable.
- **Observer**: trace, audit, health, and service events are subscribable by
  shells without giving shells semantic ownership.
- **Memento**: effective capability reports, snapshots, provider capability
  hashes, quota hashes, and audit cursors preserve bounded recovery state.
- **Specification**: admission validates pack id, command availability,
  permission scopes, provider health, entitlement, resource budgets, quota
  constraints, and policy templates.
- **Abstract Factory**: concrete provider adapters are constructed only in
  approved composition roots.

## Risks And Mitigations

- Risk: tenant pack becomes an application-specific multitenancy framework.
  Mitigation: keep tenant semantics generic; application data models, feature
  routing, and product authorization remain outside OS layers.
- Risk: tenant quotas become billing or entitlement logic. Mitigation: quotas
  represent resource/resource-policy envelopes only; commerce entitlement
  remains in commerce packs.
- Risk: tenant relationships become cloud provisioning. Mitigation: AWS, Azure,
  Kubernetes, and directory concepts are represented as references and provider
  capabilities, not direct OS-owned resource provisioning.
- Risk: organization membership leaks into tenant. Mitigation: tenant returns
  relationship and policy references; membership management stays in
  `pack.identity.organization.v1`.
- Risk: SDK helpers become a second execution path. Mitigation: helpers only
  build canonical service commands and are covered by no-direct-provider-call
  gates.
- Risk: tenant configuration leaks secrets. Mitigation: config values are
  references; secrets belong to `pack.foundation.secrets-reference.v1`.
