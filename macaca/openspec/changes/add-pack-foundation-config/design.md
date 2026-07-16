# Foundation Config Pack Design

## Context

`pack.foundation.config.v1` provides typed configuration discovery and resolution
for Macaca applications. It must support layered config, schemas, profiles,
source precedence, validation, provenance, watch events, snapshots, and redacted
exports while keeping secrets and provider-specific storage outside the generic
OS contract.

This pack is distinct from `foundation.key-value-state`: config is declarative,
versioned, policy-validated setup data with provenance and schema; state is
mutable runtime data. This pack is also distinct from `foundation.secrets-reference`:
raw secret values never live in config; only secret references may appear.

## Supplier API Comparison

| Source API family | Relevant concepts | Macaca abstraction |
| --- | --- | --- |
| Kubernetes ConfigMap | non-confidential key-value/file config, image/code separation, env/arg/file consumption | config layers, non-secret classification, effective config projection, provider source refs |
| Spring Boot external config | properties/YAML/env/CLI sources, profiles, precedence, binding, validation | source precedence, profile selectors, typed schema binding, validation reports |
| Twelve-Factor config | config/code separation, deploy-time environment, portability | manifest-declared config requirements, runtime source injection, no hardcoded environment branching |
| Android resources/preferences | resource qualifiers, alternative resources, user settings | selector/profile dimensions, typed values, app/user override layer |
| Apple bundle/defaults/plist | bundled defaults, property lists, defaults database | default layer, typed plist-like values, runtime override layer, provenance |

Design conclusion: Macaca should expose a typed config resolver with explicit
layers and provenance. It should not expose raw provider config stores or let
OS code branch on application-specific config keys.

## Goals

- Provide schema declaration, get, list, resolve, validate, explain provenance,
  watch, reload, snapshot, and redacted export operations.
- Support typed config values: bool, integer, float, string, enum, list, object,
  duration, size, URL, path ref, secret reference, and artifact reference.
- Support layers: package default, app manifest, workspace, tenant, environment,
  session, task, user override, remote provider, and test override.
- Support profiles/selectors without hardcoding environment names into OS code.
- Support effective config snapshots for audit and replay.
- Support unavailable diagnostics when a config provider, source, profile,
  schema, or secret-reference integration is absent.

## Non-Goals

- No raw secret storage or secret value resolution.
- No application-specific config key semantics in OS layers.
- No feature flag business rules; feature flag services may build on config but
  remain separate.
- No unbounded environment variable dumps or file content dumps in diagnostics.
- No direct reads from shell process env as the only config authority.
- No mutation of application business state; use key-value state for runtime
  mutable state.

## Ownership And Boundaries

- Pack id: `pack.foundation.config.v1`.
- Family: `foundation`.
- Service owner: config system service.
- Provider examples: package descriptor provider, workspace config provider,
  environment adapter, tenant config provider, remote config provider, mock
  provider, unavailable provider.
- SDK surface: `sdk.packs.foundation.config`.
- Command namespace: `config.*`.
- Microkernel ownership: identity, policy facade, service-call evidence,
  trace/audit primitives only.
- Application framework ownership: manifest config declarations, schema refs,
  app-scoped permission declarations, effective capability projection.
- Runtime-host ownership: provider registration, source adapters, decorators,
  snapshots, and unavailable provider composition.

## Command Surface

| Command | Supplier analogs | DTO notes | Side effects |
| --- | --- | --- | --- |
| `config.describe_schema` | Spring binding metadata, resource schemas | schema id, version, fields, defaults, redaction | No |
| `config.get` | property lookup, env lookup | key, selector/profile, expected type, redaction mode | No |
| `config.get_many` | grouped properties | key list, selector, type projection | No |
| `config.list_keys` | ConfigMap keys, property sources | prefix, layer filter, page token, redaction | No |
| `config.resolve_effective` | Spring precedence, layered config | selector/profile, layer order, schema validation | No |
| `config.validate` | config binding validation | schema id, candidate config, strictness | No |
| `config.explain_provenance` | property source origin | key, selected value, source chain, override reasons | No |
| `config.watch` | ConfigMap mounted updates / remote config watch | key/prefix/schema, selector, stream budget | Starts stream |
| `config.reload` | config refresh | source refs, validation mode, idempotency key | May update cache |
| `config.snapshot` | effective config record | schema/profile/filter, redaction policy | Records snapshot |
| `config.export_redacted` | diagnostics/export | effective config ref, redaction policy, format | No |

## DTO Model

Core DTOs:

- `ConfigKeyRef`: normalized key, namespace, schema field ref, prefix policy,
  redaction label.
- `ConfigValue`: typed value or secret/artifact/path reference. Raw secrets are
  forbidden.
- `ConfigSchemaRef`: schema id, version, compatibility lane, validation rules.
- `ConfigLayerRef`: package default, manifest, workspace, tenant, environment,
  session, task, user, remote, test.
- `ConfigSelector`: profile, tenant, app, session, task, locale, platform,
  provider capability, and custom declarative selectors.
- `ConfigSourceRef`: provider id, source id, version/hash, loaded timestamp,
  availability, and trust level.
- `ConfigProvenance`: selected layer, overridden layers, source refs, validation
  result, redaction summary.
- `ConfigWatchEvent`: changed, removed, unavailable, validation_failed,
  source_reloaded, stream_checkpoint.
- `ConfigError`: denied, not_found, invalid_key, invalid_schema,
  validation_failed, secret_value_forbidden, unavailable_source,
  unsupported_selector, quota_exceeded, unavailable, provider_failure.

## Permission And Policy Model

Permission scopes:

- `config.read`
- `config.list`
- `config.validate`
- `config.watch`
- `config.reload`
- `config.snapshot`
- `config.export`

Policy rules:

- Every command is scoped to tenant id, application id, session id, task id,
  schema id, key/prefix, selector, and trace id when available.
- Read/list/export commands must apply redaction before returning diagnostics.
- Config values classified as secret must be rejected unless represented as a
  secret reference and permitted by the secret-reference pack policy.
- Reload/watch commands require resource budgets, source availability checks,
  and validation before effective config changes become visible.
- Test override layers require test/replay policy context.
- Unknown selectors and profile names are data, not OS branches; they are
  validated against descriptors and policy.

## SDK And Developer Documentation

SDK discovery returns command schemas, config value types, layer model,
selector/profile support, permission scopes, policy templates, provider
availability, health, examples, docs link, and unavailable diagnostics.

Required developer guide:

- Path: `docs/developer-packs/foundation/config.md`.
- Content: config/code separation, manifest declarations, schema model, key
  model, value types, layers, selectors/profiles, precedence, validation,
  provenance, watch/reload, snapshots, redaction, secret references, unavailable
  diagnostics, provider replacement, trace/audit fields, and examples.
- Examples: read effective config, validate candidate config, explain override
  provenance, watch config changes, denied raw secret value, redacted export, and
  unavailable remote config source.

## Trace, Audit, Health, Snapshot, And Replay

Required event names:

- `config_pack_declared`
- `config_pack_admission_validated`
- `config_pack_policy_decision`
- `config_pack_service_call_requested`
- `config_pack_service_call_succeeded`
- `config_pack_service_call_failed`
- `config_pack_source_reloaded`
- `config_pack_watch_started`
- `config_pack_watch_stopped`
- `config_pack_snapshot_recorded`
- `config_pack_validation_failed`
- `config_pack_unavailable`

Events include pack id, service id, command name, trace id, app/session/task
identifiers, schema id, selector hash, key/prefix hash, layer names, source
hashes, validation result, redaction summary, provider class, latency, bounded
resource counters, and bounded error code. Events do not include raw secret
values, unbounded config values, raw provider payloads, or raw environment dumps.

Health checks include provider registered state, source availability, schema
registry health, watcher support, reload support, max key/value size, max source
count, redaction support, secret-reference integration, and unavailable reasons.

Snapshots include descriptor version, provider class, schema ids, source hashes,
effective config hash, policy template hash, redaction summary, validation
result, and replay references.

## Implementation Slices

1. Contract slice: descriptor, command schemas, config DTOs, result/error DTOs,
   health/snapshot DTOs, provider capability report, stable hashes.
2. Admission slice: config declarations, schema refs, required/optional behavior,
   permission validation, selector/profile validation, service mapping.
3. Service slice: config service trait/provider interface, unavailable provider,
   mock provider, package/workspace/environment providers, remote adapter bridge.
4. SDK slice: discovery, typed command builders, effective config helper,
   provenance helper, watch helper, redacted export helper, docs link.
5. WASM/app-runtime slice: expose only declared callable config imports through
   service runtime; no raw env/config provider handles.
6. Observability slice: trace/audit events, redaction, replay tests, health
   snapshots, watch cancellation.
7. Developer-docs slice: complete `docs/developer-packs/foundation/config.md`
   and link it from catalog metadata.

## Design Patterns

- **Facade**: SDK exposes config helpers and command builders only.
- **Command**: every operation is a typed command/result.
- **Adapter/Bridge**: package, workspace, environment, remote, mock, and
  unavailable providers adapt to one contract.
- **Strategy**: layer precedence, selector resolution, provider selection,
  redaction behavior, and unavailable behavior are replaceable.
- **Decorator**: trace, policy, resource, redaction, validation, and audit wrap
  calls.
- **Specification**: schemas, selectors, layer order, permission scopes, and
  redaction rules are executable validators.
- **Observer**: watch streams, source reloads, validation failures, and audit
  events are subscribable.
- **Memento**: effective config snapshots preserve replay state.

## Risks And Mitigations

- Risk: config becomes a secret store.
  Mitigation: raw secret values are rejected; only secret references are allowed.
- Risk: OS code hardcodes environment/profile names.
  Mitigation: selectors are descriptor data validated by policy, not branches.
- Risk: config changes are not replayable.
  Mitigation: effective config snapshots include source hashes, layer order,
  schema ids, validation result, and redaction summary.
- Risk: watch/reload creates unbounded streams.
  Mitigation: stream budgets, page bounds, cancellation, and source health
  diagnostics.
- Risk: provider precedence becomes inconsistent.
  Mitigation: precedence strategy is explicit, versioned, and included in
  effective config metadata.
