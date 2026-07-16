## 1. Supplier API Research And Scope

- [x] 1.1 Read and summarize Kubernetes ConfigMap concepts for non-confidential
  config, key-value/file data, source injection, and config/code separation.
- [x] 1.2 Read and summarize Spring Boot externalized configuration for property
  sources, YAML/properties, environment variables, command-line args, profiles,
  precedence, binding, and validation.
- [x] 1.3 Read and summarize Twelve-Factor config principles for deploy-time
  config, portability, and separation from code.
- [x] 1.4 Read and summarize Android resources/preferences concepts for
  resource qualifiers, alternative resources, and user/application settings.
- [x] 1.5 Read and summarize Apple bundle/defaults/plist concepts for bundled
  defaults, typed property values, and runtime overrides.
- [x] 1.6 Convert the supplier comparison into Macaca-owned abstractions and
  explicitly reject raw secret values, app-specific keys in OS code, and
  provider-native config handles.
- [x] 1.7 Record GitNexus CRITICAL/HIGH findings as memo only before
  implementation commits.

## 2. Contract, Descriptor, And DTOs

- [x] 2.1 Define `pack.foundation.config.v1` descriptor metadata: lifecycle,
  stability, service ids, command namespace, command schemas, permission scopes,
  policy template, resource template, SDK metadata, docs link, health, snapshot,
  and unavailable diagnostics.
- [x] 2.2 Define command DTOs for `config.describe_schema`, `config.get`,
  `config.get_many`, `config.list_keys`, `config.resolve_effective`,
  `config.validate`, `config.explain_provenance`, `config.watch`,
  `config.reload`, `config.snapshot`, and `config.export_redacted`.
- [x] 2.3 Define shared DTOs for config key refs, typed values, schema refs,
  layer refs, selectors/profiles, source refs, provenance, watch events,
  validation reports, redaction summaries, provider capability reports, and
  stable descriptor hashes.
- [x] 2.4 Define result/error DTOs for success, partial page, watch checkpoint,
  denied, not_found, invalid_key, invalid_schema, validation_failed,
  secret_value_forbidden, unavailable_source, unsupported_selector,
  quota_exceeded, unavailable, and provider_failure.
- [x] 2.5 Add schema compatibility tests and stable hash tests for command,
  result, health, snapshot, provider capability, and unavailable DTOs.

## 3. Admission, Permission, Policy, Resource, And Approval

- [ ] 3.1 Implement manifest declaration validation for required/optional
  `pack.foundation.config.v1`, schema refs, and app-scoped config selectors.
- [x] 3.2 Validate scopes: `config.read`, `config.list`, `config.validate`,
  `config.watch`, `config.reload`, `config.snapshot`, and `config.export`.
- [x] 3.3 Add policy checks for schema id, key/prefix bounds, layer access,
  selector/profile access, max value size, max source count, redaction mode,
  watch budget, reload budget, and provider capability.
- [x] 3.4 Reject raw secret values and require secret-reference interoperability
  for secret-classified config.
- [ ] 3.5 Add approval behavior for reload from external sources, broad export,
  test override activation, and tenant-wide config changes when providers support
  mutation.
- [ ] 3.6 Add tests proving denied, unavailable, validation_failed, quota, and
  secret_value_forbidden paths do not invoke concrete providers where they should
  be rejected before side effects.

## 4. Service Provider And Runtime Integration

- [ ] 4.1 Define the config service trait/provider interface behind the service
  runtime.
- [x] 4.2 Implement unavailable provider behavior for absent config service,
  missing source, unsupported watch/reload, unsupported selector, missing schema,
  and missing secret-reference integration.
- [ ] 4.3 Implement deterministic mock provider for contract and replay tests.
- [ ] 4.4 Implement or bind package descriptor, workspace config, environment
  adapter, tenant config, and remote config bridge providers without leaking
  provider-native APIs to SDK callers.
- [ ] 4.5 Add lifecycle, health, snapshot, shutdown, reload, watch cancellation,
  validation, redaction, and provider capability reports.

## 5. SDK, WASM ABI, And Application Framework

- [x] 5.1 Extend SDK discovery with pack metadata, command schemas, value types,
  layer model, selectors/profiles, permissions, policy templates, provider
  availability, health, diagnostics, and docs link.
- [x] 5.2 Add SDK command builders for every `config.*` command; builders must
  only produce canonical traced service calls.
- [ ] 5.3 Add SDK helpers for effective config, typed get, validation, provenance
  explanation, watch cancellation, redacted export, and unavailable diagnostics.
- [ ] 5.4 Extend effective capability projection so applications can inspect
  callable commands, denied commands, unavailable sources, provider capability
  flags, schema availability, and replay references.
- [ ] 5.5 Expose WASM host imports only for declared callable config commands and
  route every import through the service runtime path.
- [ ] 5.6 Add app-framework tests proving YAML, WASM, GenUI, and headless apps all
  use the same config execution path.

## 6. Trace, Audit, Replay, And Gates

- [ ] 6.1 Emit sanitized events for declaration, admission, policy, resource,
  service calls, source reloads, watch lifecycle, snapshots, validation failures,
  success, failure, denied, and unavailable states.
- [ ] 6.2 Add audit redaction tests proving raw secret values, unbounded config
  values, raw environment dumps, prompts, manifests, package bytes, credentials,
  private keys, and provider payloads do not enter observability surfaces.
- [ ] 6.3 Add replay tests proving config commands are trace-addressable and can
  reconstruct effective config with source hashes, layer order, schema ids,
  validation result, and redaction summary.
- [ ] 6.4 Add dependency-boundary tests proving kernel, SDK, shells, and
  application framework do not import concrete config providers.
- [ ] 6.5 Add no-direct-provider-call gates proving SDK helpers and WASM host
  imports cannot bypass service runtime.
- [ ] 6.6 Run `openspec validate add-pack-foundation-config --strict`, targeted
  cargo tests, dependency-boundary gates, file-size gates, and audit replay
  checks before marking complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/foundation/config.md`.
- [x] 7.2 Document purpose, manifest declaration, config/code separation, schema
  model, key model, value types, layers, selectors/profiles, precedence,
  validation, provenance, watch/reload, snapshots, redaction, secret references,
  permissions, policy defaults, command DTOs, result DTOs, error DTOs,
  unavailable diagnostics, and provider replacement.
- [x] 7.3 Add minimal examples for reading effective config, validating candidate
  config, explaining override provenance, watching config changes, denied raw
  secret values, redacted export, and unavailable remote config source.
- [x] 7.4 Cross-link the guide from SDK discovery metadata and the industrial pack
  catalog index before marking this proposal complete.
