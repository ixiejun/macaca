# Foundation Config Pack Research

## Purpose

This note records supplier/API research for `pack.foundation.config.v1`. The
pack must provide layered, typed, traceable configuration resolution while
keeping raw secrets, provider-native config handles, and application-specific
configuration keys out of OS-layer code.

## Source Baseline

- Kubernetes ConfigMap:
  <https://kubernetes.io/docs/concepts/configuration/configmap/>
- Kubernetes ConfigMap pod usage:
  <https://kubernetes.io/docs/tasks/configure-pod-container/configure-pod-configmap/>
- Spring Boot externalized configuration:
  <https://docs.spring.io/spring-boot/reference/features/external-config.html>
- Twelve-Factor App config:
  <https://12factor.net/config>
- Android app resources:
  <https://developer.android.com/guide/topics/resources/providing-resources>
- Android SharedPreferences guide:
  <https://developer.android.com/training/data-storage/shared-preferences>
- Apple `UserDefaults`:
  <https://developer.apple.com/documentation/foundation/userdefaults>
- Apple Preferences and Settings Programming Guide:
  <https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/UserDefaults/Introduction/Introduction.html>
- Apple User Defaults value access:
  <https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/UserDefaults/AccessingPreferenceValues/AccessingPreferenceValues.html>
- Apple Information Property List:
  <https://developer.apple.com/documentation/bundleresources/information-property-list>

## Kubernetes ConfigMap Summary

Kubernetes ConfigMaps establish a clean non-secret configuration object model:

- ConfigMaps store non-confidential key-value or file-like configuration.
- Workloads can consume config as environment variables, command-line
  arguments, or mounted files.
- ConfigMaps decouple environment-specific configuration from immutable images,
  supporting portability.
- ConfigMaps are explicitly not secret stores. Macaca should reject raw secret
  config values and require `foundation.secrets-reference` interoperability.
- Config sources may be reloaded or projected differently by providers. Macaca
  should expose source refs, layer refs, provenance, and unavailable diagnostics
  rather than Kubernetes object shapes.

## Spring Boot Externalized Configuration Summary

Spring Boot contributes a mature layered configuration model:

- Configuration can come from properties files, YAML, environment variables,
  command-line arguments, and other sources.
- Source precedence is explicit; later/higher-priority sources can override
  earlier defaults.
- Profiles/selectors allow environment-specific activation without changing
  code.
- Binding and validation show that configuration should have schema and typed
  value validation before use.
- Macaca should borrow layer/precedence/profile/schema ideas while rejecting
  Spring-specific annotations, property source classes, and binding APIs.

## Twelve-Factor Config Summary

Twelve-Factor config establishes the deployment boundary:

- Config varies between deploys; code should not.
- Configuration should be externalized so the same code can run in many
  environments.
- Orthogonal controls are better than hardcoded environment group branching.
- Macaca should represent deployment variation as selectors/profiles and
  declarative source refs, not OS code branches on environment names.
- Secrets require separate handling. Even when supplied through environment-like
  mechanisms, raw secret values must not enter config observability.

## Android Resources / Preferences Summary

Android contributes resource-qualifier and preference concepts:

- Resources are external files/static content referenced by code, with
  alternative resources selected based on device/runtime qualifiers.
- Resource qualifiers map to Macaca `ConfigSelector` dimensions such as locale,
  platform, tenant, session, or provider capability.
- SharedPreferences contributes app/user setting concepts, but mutable runtime
  settings overlap with key-value state. Config should remain declarative,
  schema-backed setup data with provenance.
- Android-specific resource directories, generated IDs, and preference APIs must
  not leak into the Macaca SDK/ABI.

## Apple Bundle / Defaults / Plist Summary

Apple APIs establish default-value and property-list value principles:

- Bundle property lists contain key-value metadata used by the system to
  interpret an app bundle.
- UserDefaults provides persistent app-specific and system-wide settings.
- Apple preferences are built around property-list data types such as strings,
  numbers, dates, arrays, and dictionaries.
- Apps can register default values used when app-specific values are absent.
- Macaca should borrow default layer, typed plist-like value categories, runtime
  override, and provenance concepts, while rejecting bundle-specific keys,
  defaults domains, and Apple provider handles.

## Macaca-Owned Abstractions

`pack.foundation.config.v1` should define these provider-neutral concepts:

- `ConfigKeyRef`: normalized key, namespace, schema field reference, prefix
  policy, and redaction label.
- `ConfigValue`: typed value or reference-only value; raw secret values are
  forbidden and secret-classified values must be secret references.
- `ConfigSchemaRef`: schema id, version, compatibility lane, default values,
  validation rules, and migration metadata.
- `ConfigLayerRef`: package default, manifest, workspace, tenant, environment,
  session, task, user override, remote provider, and test override.
- `ConfigSelector`: declarative profile/tenant/app/session/task/locale/platform
  dimensions without hardcoded OS branch names.
- `ConfigSourceRef`: provider id, source id, source version/hash, load time,
  trust level, and availability.
- `ConfigProvenance`: selected value, selected layer, overridden layers, source
  refs, validation result, and redaction summary.
- `ConfigWatchEvent`: changed, removed, unavailable, validation_failed,
  source_reloaded, and stream_checkpoint.
- `ConfigValidationReport`: schema id, candidate hash, field results, missing
  required values, type mismatches, secret-value rejection, and bounded errors.
- `ConfigProviderCapability`: supported sources, layers, selectors, max value
  size, watch/reload support, schema support, redaction support, secret-reference
  integration, health, and unavailable reasons.

## Rejected API Leakage

Macaca must not expose these provider-native shapes as stable SDK/ABI contracts:

- Kubernetes ConfigMap object schemas, pod env/volume projection mechanics,
  namespace/object names, or Kubernetes watch events.
- Spring `Environment`, `PropertySource`, `@ConfigurationProperties`, profile
  activation internals, or Spring binding exceptions.
- Twelve-Factor environment-variable-only assumptions as a mandatory Macaca
  runtime implementation detail.
- Android resource directories, generated resource identifiers, qualifier order,
  SharedPreferences editor APIs, or Android setting screens.
- Apple bundle keys, plist file paths, UserDefaults domains/suites, defaults
  command behavior, or Cocoa preference notification objects.
- Raw secret values, raw environment dumps, unbounded config exports, or
  application-specific config-key semantics in OS-layer code.

All operations must enter through typed Macaca service commands with trace
context, policy checks, resource limits, schema validation, structured result
envelopes, sanitized audit events, unavailable provider behavior, and provider
replacement support.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
