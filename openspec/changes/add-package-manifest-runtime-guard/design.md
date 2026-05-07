# Design: Package manifest runtime guard

## Context

Route C Phase 01 introduced microkernel primitives, Phase 02 introduced system service contracts, and Phase 03 introduced a local-first service bus. Phase 04 introduces the software distribution contract that every application, plugin, skill, MCP package, driver, system module, and UI component pack can eventually share.

The current repository already has application manifests (`app.yaml`), driver manifests (`driver.toml` / ABI JSON), and skill frontmatter metadata. Those shapes are useful but isolated. Phase 04 does not erase them; it creates a canonical package descriptor and guard chain that adapters can map into incrementally.

## Goals

- Define Package Manifest v0 as data-only protocol contracts in `macaca-proto`.
- Represent existing YAML applications as first-class package descriptors.
- Validate package load attempts through an ordered runtime guard chain.
- Preserve current YAML application loading and `/api/chat/v2` behavior.
- Keep commerce metadata inert until Store / entitlement phases.
- Keep WASM metadata loadable but non-executable until Application ABI/runtime phases.
- Make guard decisions traceable, auditable, structured, and policy-ready.
- Avoid application-specific, provider-specific, driver-specific, gateway-specific, model-specific, chain-specific, or workflow-specific branching.

## Non-Goals

- Do not build the Store.
- Do not implement payment, subscription, entitlement, encryption, or license enforcement.
- Do not execute WASM.
- Do not migrate every existing package-like loader.
- Do not introduce a package manager UI.
- Do not make `macaca-web` define package semantics.

## Superpowers Brainstorm Summary

### Current Problem

Macaca needs to evolve from YAML/demo-oriented application loading into an OS ecosystem where software can be packaged, validated, hot-installed, traced, and eventually distributed through a Store. Without a shared manifest and guard chain, each subsystem will continue inventing loader-specific rules and bypassing common compatibility, permission, optional service, and trace/audit behavior.

### Options Considered

1. **Canonical package contract with compatibility adapters.**
   - Pros: keeps YAML apps first-class, supports future package types, minimizes migration risk, gives all loaders one guard vocabulary.
   - Cons: requires adapter code before full migration is complete.
   - Verdict: recommended for Phase 04.

2. **Extend existing `AppManifest` only.**
   - Pros: small change for current applications.
   - Cons: keeps skill/driver/plugin/MCP outside the model, makes Store and optional modules harder later, reinforces application-specific semantics.
   - Verdict: reject as too narrow.

3. **Implement full Store/package manager now.**
   - Pros: end-to-end ecosystem story sooner.
   - Cons: combines manifest, entitlement, payments, encryption, distribution, runtime installation, and UI into one high-risk phase.
   - Verdict: reject as over-scoped.

### Recommended Plan

Implement the canonical package contract and guard chain additively. YAML applications are adapted into package descriptors now. Skill, driver, and runtime-host gain descriptor conversion hooks or skeletons where useful. Store, entitlement, encrypted package enforcement, plugin installation, and WASM execution remain later phases.

## Design Patterns

- **Builder**: package descriptors should be built from raw manifest data through explicit builders or conversion functions that normalize defaults and keep construction readable.
- **Specification**: schema, compatibility, permission, service requirement, optional module, signature metadata, and commerce precheck rules are composable specifications instead of hardcoded `if app == ...` branches.
- **Chain of Responsibility**: runtime guard steps run in order: parse/schema, signature metadata, compatibility, permission, required services, optional service availability, commerce inert precheck.
- **Factory Method**: package loader selection is based on runtime kind and package type, allowing YAML, future WASM component, native adapter, remote service, and encrypted text bundle loaders to evolve independently.
- **Adapter**: existing YAML app manifests, driver manifests, and skill metadata map into canonical package descriptors without rewriting their current loaders.
- **Null Object**: missing optional modules/services become structured unavailable descriptors rather than panics, hangs, or silent skips.
- **Observer**: validation and load decisions emit trace/audit events through presentation-neutral sinks.
- **Value Object**: package id, developer id, package type, runtime kind, ABI version, service requirement, and capability ids remain typed data rather than raw strings at runtime boundaries.

## Contract Shape

### `macaca-proto`

Add a focused package protocol module, likely `macaca-proto/src/package.rs`, containing data-only types:

- `PackageId`
- `DeveloperId`
- `PackageType`
- `PackageVersion`
- `PackageSignature`
- `PackageRuntime`
- `PackageRuntimeKind`
- `PackageEntry`
- `PackagePermission`
- `PackageServiceRequirement`
- `PackageCapability`
- `PackageCommerceMetadata`
- `PackageCompatibility`
- `PackageManifest`
- `PackageDescriptor`
- `PackageGuardError`
- `PackageGuardDecision`

Package type and runtime kind should be extensible value objects or enums with `Custom(String)` variants so third-party package categories do not require kernel source edits.

### `macaca-app`

Add package-facing Application Framework modules:

- `package.rs`: package descriptor builder and YAML app compatibility conversion.
- `runtime_guard.rs`: guard chain traits, default guard, rule results, trace/audit hooks.
- `package_loader.rs`: loader factory and runtime-kind loader interfaces.
- `tests/package_manifest.rs`: YAML compatibility, guard rejection, optional service unavailable, and loader factory tests.

Existing `AppLoader` remains available. Phase 04 may add an adapter that calls existing parsing logic and returns a package descriptor, but it must not replace all runtime paths at once.

### Skill / Driver / Runtime Host Hooks

Phase 04 may add small conversion hooks:

- skill metadata to package descriptor skeleton;
- driver manifest to package descriptor skeleton;
- runtime-host package requirement structures or guard integration points.

These hooks should be additive and testable. Full loader migration belongs to later package/plugin/runtime phases.

## Runtime Guard Chain

The default guard chain should be explicit and ordered:

```text
parse/schema
  -> signature metadata validation
  -> compatibility validation
  -> permission validation
  -> required service validation
  -> optional service availability marking
  -> commerce inert precheck
```

Each step returns structured data. Required service absence rejects the package. Optional service absence does not reject the package; it marks the service requirement as unavailable and records that outcome for trace/audit. Commerce metadata is parsed and preserved but must not enforce payment or entitlement in this phase.

## Trace, Audit, And Logging

Package operations must log and trace:

- manifest parsed;
- descriptor built;
- guard step started;
- guard step passed;
- guard step rejected;
- optional service marked unavailable;
- loader selected;
- runtime unavailable;
- package accepted for metadata load.

Logs and trace payloads should include package id, package type, runtime kind, ABI version, guard step, decision, structured error code, and correlation ids when available. They must not log secrets, signatures as raw private material, tokens, or encrypted package contents.

## Compatibility Rules

Phase 04 must preserve YAML applications as first-class packages. A YAML application descriptor should include app id, app name, version, entry agent or entrypoint, workflow references, required services inferred from its declared capabilities/tools where possible, optional service declarations where available, and provided capabilities.

WASM component packages may be parsed as metadata, but execution must fail with structured `RuntimeUnavailable` until the Application ABI phase. Unknown future package types or runtime kinds must not crash; unsupported execution attempts must return structured unsupported/unavailable errors.

## Risks / Trade-offs

- **Risk: Package model becomes too Store-heavy too early.** Mitigation: commerce metadata is inert and entitlement enforcement is explicitly deferred.
- **Risk: YAML apps are treated as legacy.** Mitigation: YAML adapter is a required acceptance path and regression target.
- **Risk: Guard chain becomes hardcoded.** Mitigation: rules are Specifications and Chain of Responsibility steps with traceable decisions.
- **Risk: WASM metadata suggests executable WASM support.** Mitigation: loader factory must return structured runtime unavailable for execution.
- **Risk: File size grows beyond project limits.** Mitigation: split proto, app package builder, runtime guard, and loader factory into focused files under 500 lines each.

## Open Questions

- None for Phase 04. Store, entitlement, encrypted paid package enforcement, package installation UI, full plugin runtime, and WASM execution belong to later phases.
