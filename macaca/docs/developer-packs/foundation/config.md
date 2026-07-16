# Foundation Config Pack

`pack.foundation.config.v1` provides provider-neutral configuration discovery
and resolution for Macaca applications. It covers schema description, typed
configuration references, key listing, effective-value resolution, validation,
provenance explanation, watch/reload, snapshots, redacted export, and
unavailable diagnostics without exposing provider-native config handles.

## Manifest Declaration

Declare the pack in an application service contract:

```yaml
service_contract:
  optional_packs:
    - pack.foundation.config.v1
```

Use `required_packs` only when the application cannot run without a registered
configuration provider. When no provider is installed, admission returns an
explicit `config_provider_not_installed` diagnostic.

## Config And Secret Boundary

Configuration stores non-secret settings and references to secret material. Raw
secret values are not valid app-facing config results. Secret-classified config
must use `pack.foundation.secrets.reference.v1` interoperability so providers
can resolve secrets only inside policy-approved service calls.

## Permissions

The pack defines these provider-neutral scopes:

- `config.read`: read a bounded config value reference.
- `config.list`: list keys within declared namespace and prefix bounds.
- `config.validate`: validate candidate config against a schema.
- `config.watch`: subscribe to bounded change checkpoints.
- `config.reload`: reload admitted sources.
- `config.snapshot`: create metadata snapshots.
- `config.export`: export redacted diagnostics.

## Commands

- `config.describe_schema`: inspect a schema reference and compatibility
  metadata.
- `config.get`: read one key through a selector.
- `config.get_many`: read a bounded key batch.
- `config.list_keys`: list keys by namespace and optional prefix.
- `config.resolve_effective`: resolve the winning layer for a key.
- `config.validate`: validate a candidate artifact against a schema.
- `config.explain_provenance`: explain layer/source precedence.
- `config.watch`: create a bounded watch stream from a cursor.
- `config.reload`: request a dry-run or admitted source reload.
- `config.snapshot`: create a metadata snapshot.
- `config.export_redacted`: export diagnostics without raw values.

## DTO Guidance

Use `ConfigKeyReference` for namespace-scoped keys, `ConfigSelector` for
profile/tenant/environment selection, and `ConfigSourceReference` for redacted
source identity. `ConfigTypedValueRef` carries type and artifact references, not
raw values. `ConfigProvenance` and `ConfigRedactionSummary` are safe for
diagnostics when bounded.

Logs, traces, snapshots, SDK diagnostics, and examples must not contain raw
secret values, raw environment dumps, raw provider payloads, credentials,
private keys, package bytes, manifests, prompts, or unbounded config values.

## Result And Error DTOs

All commands use a bounded result envelope with status, optional data, optional
error, trace id, and descriptor hash. Standard statuses are `success`,
`partial_page`, `watch_checkpoint`, `denied`, `not_found`, `invalid_key`,
`invalid_schema`, `validation_failed`, `secret_value_forbidden`,
`unavailable_source`, `unsupported_selector`, `quota_exceeded`, `unavailable`,
and `provider_failure`.

## Examples

Reading effective config:

```json
{
  "key": {
    "key": "ui.theme",
    "namespace": "app"
  },
  "selector": {
    "profile": "default",
    "tenant_ref": "tenant-ref",
    "environment_ref": "env-ref"
  },
  "include_provenance": true
}
```

Validating candidate config:

```json
{
  "candidate_ref": "artifact:candidate-config",
  "schema": {
    "schema_id": "app.settings",
    "version": "v1"
  },
  "selector": {
    "profile": "staging",
    "tenant_ref": "tenant-ref",
    "environment_ref": "env-ref"
  }
}
```

Explaining override provenance:

```json
{
  "key": {
    "key": "feature.flag",
    "namespace": "app"
  },
  "selector": {
    "profile": "production",
    "tenant_ref": "tenant-ref",
    "environment_ref": "env-ref"
  }
}
```

Watching config changes:

```json
{
  "namespace": "app",
  "selector": {
    "profile": "default",
    "tenant_ref": "tenant-ref",
    "environment_ref": "env-ref"
  },
  "start_cursor": "cursor"
}
```

Denied raw secret value:

```json
{
  "status": "secret_value_forbidden",
  "error": {
    "code": "secret_value_forbidden",
    "message": "secret-classified config must be returned as a secret reference",
    "retryable": false
  }
}
```

Redacted export:

```json
{
  "selector": {
    "profile": "default",
    "tenant_ref": "tenant-ref",
    "environment_ref": "env-ref"
  },
  "redaction_level": "metadata_only"
}
```

Unavailable remote config source:

```json
{
  "status": "unavailable_source",
  "error": {
    "code": "unavailable_source",
    "message": "remote configuration source is unavailable",
    "retryable": true
  }
}
```

## Provider Replacement

Providers are replaceable service implementations. Expected provider classes
include `package-descriptor`, `workspace`, `environment`, `remote`, `mock`, and
`unavailable`. Provider adapters must expose descriptor metadata, health,
snapshots, command support, source hashes, schema hashes, redaction summaries,
unavailable states, and sanitized diagnostics through the service runtime.
SDKs, shells, kernel code, and applications must not instantiate provider
configuration objects directly.
