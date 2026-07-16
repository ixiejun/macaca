# Change: Add Foundation Config Pack

## Why

Developers need `pack.foundation.config.v1` as a layered, typed, auditable
configuration capability. Applications need to declare configuration schemas,
read effective values, resolve layered overrides, validate configuration, watch
changes, inspect provenance, and receive unavailable diagnostics without
hardcoding environment-specific values or reading secrets directly.

Configuration is foundational because every application and pack needs portable
runtime settings. If configuration is scattered across ad hoc files, environment
variables, prompts, provider-specific stores, or shell code, Macaca cannot audit
what configuration was used, replay decisions, enforce secret boundaries, or
support multi-tenant provider replacement.

## Supplier And Platform API Research

The proposal is derived from a capability-by-capability comparison of mature
configuration systems:

- Kubernetes ConfigMap: non-confidential key-value/file configuration separated
  from container images and consumed as environment variables, command arguments,
  or mounted files.
- Spring Boot Externalized Configuration: multiple sources, properties/YAML,
  environment variables, command-line arguments, profiles, binding, and
  precedence.
- Twelve-Factor App config: strict separation of config from code and portable
  deploy-time configuration.
- Android resources and preferences: resource qualifiers for device/runtime
  configuration and user/application preference settings.
- Apple bundle/property-list/defaults patterns: bundled defaults, runtime
  overrides, typed preference values, and platform-managed settings.

Macaca borrows the stable concepts, not provider APIs:

- configuration is declarative descriptor data, not application code;
- secrets are references, not raw config values;
- all values have schema, source, precedence, and provenance;
- effective config snapshots are replayable;
- watch/update behavior is bounded and policy-governed;
- provider absence returns structured unavailable diagnostics.

## What Changes

- Define `pack.foundation.config.v1` as the canonical app-facing configuration
  pack.
- Add an industrial command surface covering schema registration, get, list,
  resolve, validate, explain provenance, watch, snapshot, reload, and export
  redacted effective config.
- Define provider-neutral DTO requirements for config keys, typed values,
  schemas, layers, profiles, precedence, source refs, secret refs, provenance,
  validation reports, watch events, and snapshot refs.
- Define permission scopes for read, list, validate, watch, reload, snapshot,
  and export.
- Require a detailed developer guide under `docs/developer-packs/foundation/config.md`
  before this proposal can be marked complete.
- Keep implementation ownership in a config system service; kernel, SDK, shells,
  and application framework remain provider-neutral.

## Impact

- Affected specs: `pack-foundation-config`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral DTOs, descriptor validators, application
  admission, SDK discovery, SDK command helpers, config service provider,
  mock/unavailable providers, trace/audit event schema, replay tests, and
  dependency-boundary gates.
- Non-goals: secret storage, provider-specific config APIs in SDK, shell-owned
  config semantics, app-specific config keys in OS code, or raw environment dump
  exposure in traces.
