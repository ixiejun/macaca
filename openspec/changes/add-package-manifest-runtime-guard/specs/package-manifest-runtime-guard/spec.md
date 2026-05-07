## ADDED Requirements

### Requirement: Macaca SHALL define Package Manifest v0 contracts

Macaca SHALL define provider-neutral Package Manifest v0 contracts that describe package id, package type, version, developer id, signature metadata, runtime kind, runtime ABI version, entry, permissions, required services, optional services, provided capabilities, inert commerce metadata, compatibility constraints, and metadata.

#### Scenario: Package manifest round trips through serde

- **WHEN** a package manifest is serialized and deserialized
- **THEN** the decoded manifest SHALL preserve package id, package type, version, developer id, signature metadata, runtime kind, ABI version, entry, permissions, required services, optional services, provided capabilities, commerce metadata, compatibility constraints, and metadata
- **AND** the package contract SHALL NOT depend on `macaca-web`, frontend code, concrete provider crates, concrete driver implementations, concrete gateway implementations, chain implementations, Store implementation, payment implementation, or business workflows

### Requirement: Macaca SHALL model package type and runtime kind as extensible contracts

Macaca SHALL support package types for application, skill, plugin, mcp, driver, system module, and UI component pack, and runtime kinds for yaml, wasm component, native adapter, remote service, and encrypted text bundle, while still allowing unknown future package or runtime kinds to be represented as structured data.

#### Scenario: Unknown future type does not crash

- **WHEN** a package manifest declares a future package type or future runtime kind unknown to the current host
- **THEN** parsing SHALL preserve the unknown value as structured data
- **AND** execution or loading that requires unsupported runtime behavior SHALL return a structured unsupported or runtime-unavailable error instead of panicking, hanging, or silently accepting unsafe execution

### Requirement: Macaca SHALL represent existing YAML applications as first-class packages

Macaca SHALL provide a compatibility adapter that maps existing YAML application manifests into package descriptors without treating YAML applications as second-class legacy inputs.

#### Scenario: YAML application maps to package descriptor

- **WHEN** an existing YAML `app.yaml` manifest is parsed
- **THEN** the compatibility adapter SHALL produce a package descriptor containing application id, application name, version, YAML runtime kind, entry agent or entrypoint when declared, workflow references when declared, agent capabilities, allowed tools, required service declarations when inferable, optional service declarations when present, and provided capabilities
- **AND** the adapter SHALL NOT hardcode demo application names or application-specific workflow routing

### Requirement: Macaca SHALL validate package load attempts through an ordered runtime guard chain

Macaca SHALL validate package load attempts through an ordered runtime guard chain that runs schema validation, signature metadata validation, compatibility validation, permission validation, required service validation, optional service availability marking, and inert commerce precheck.

#### Scenario: Guard steps run in deterministic order

- **WHEN** a package load attempt reaches the runtime guard
- **THEN** the guard SHALL evaluate schema, signature metadata, compatibility, permissions, required services, optional services, and commerce metadata in deterministic order
- **AND** each guard step SHALL return structured pass, warning, unavailable, or rejection data
- **AND** callers SHALL NOT need provider-specific string parsing to understand the decision

### Requirement: Macaca SHALL reject invalid package runtime metadata with structured errors

Macaca SHALL reject packages that lack required runtime metadata or declare incompatible runtime metadata with structured errors before execution or service activation.

#### Scenario: Missing runtime kind is rejected

- **WHEN** a package manifest lacks runtime kind
- **THEN** the runtime guard SHALL reject the package with a structured missing-runtime-kind error
- **AND** the package SHALL NOT be executed or activated

#### Scenario: Incompatible ABI is rejected

- **WHEN** a package manifest requires a runtime ABI version unsupported by the current host
- **THEN** the runtime guard SHALL reject the package with a structured ABI-incompatible error
- **AND** the rejection SHALL include required and supported ABI version data

### Requirement: Macaca SHALL distinguish required and optional service availability

Macaca SHALL reject packages whose required services are unavailable, and SHALL mark missing optional services unavailable without rejecting the package.

#### Scenario: Required service missing rejects package

- **WHEN** a package declares a required service that is not registered or not available
- **THEN** the runtime guard SHALL reject the package with a structured missing-required-service error
- **AND** the package SHALL NOT be executed or activated

#### Scenario: Optional service missing degrades package descriptor

- **WHEN** a package declares an optional service that is not registered or not available
- **THEN** the runtime guard SHALL preserve the package descriptor
- **AND** the optional service SHALL be marked unavailable with structured reason data
- **AND** the package SHALL remain eligible for metadata load when all required checks pass

### Requirement: Macaca SHALL keep commerce metadata inert in Phase 04

Macaca SHALL parse and preserve commerce metadata such as license, subscription, price, and distribution hints without enforcing payment, subscription, entitlement, or encrypted paid package rules in Phase 04.

#### Scenario: Commerce metadata is preserved but not enforced

- **WHEN** a package manifest includes commerce metadata
- **THEN** the package descriptor SHALL preserve that metadata
- **AND** the runtime guard SHALL record an inert commerce precheck outcome
- **AND** Phase 04 SHALL NOT perform payment, subscription, entitlement, license server, Store, or decryption enforcement

### Requirement: Macaca SHALL select package loaders by runtime kind

Macaca SHALL provide a package loader factory that selects package loaders by runtime kind and package type without hardcoded application, provider, driver, gateway, model, chain, or workflow names.

#### Scenario: YAML loader loads metadata through compatibility path

- **WHEN** a package uses YAML runtime kind and application package type
- **THEN** the loader factory SHALL select the YAML package loader
- **AND** the loader SHALL use the compatibility adapter to load package metadata for existing YAML applications
- **AND** existing YAML application loading behavior SHALL remain compatible

#### Scenario: WASM execution without runtime returns unavailable

- **WHEN** a package uses WASM component runtime kind and execution is requested before a WASM runtime is installed
- **THEN** the loader factory SHALL return a structured `RuntimeUnavailable` or equivalent error
- **AND** it SHALL NOT attempt to execute WASM code

### Requirement: Macaca SHALL emit trace and audit records for package guard decisions

Macaca SHALL emit presentation-neutral trace/audit records and structured logs for package parse, descriptor build, guard step start, guard step pass, guard step rejection, optional service unavailable, loader selection, runtime unavailable, and package metadata load outcomes.

#### Scenario: Rejected package produces traceable guard decision

- **WHEN** the runtime guard rejects a package
- **THEN** trace/audit records SHALL include package id, package type, runtime kind, ABI version when present, guard step, decision, structured error code, and correlation ids when available
- **AND** logs SHALL NOT include secrets, private keys, raw encrypted package contents, or provider credentials

#### Scenario: Accepted metadata load produces traceable outcome

- **WHEN** package metadata load succeeds
- **THEN** trace/audit records SHALL include package id, package type, runtime kind, loader kind, final guard decision, optional service availability summary, and correlation ids when available

### Requirement: Macaca SHALL provide additive descriptor hooks for skills, drivers, and runtime-host packages

Macaca SHALL provide additive descriptor conversion hooks for existing skill metadata, driver manifests, and runtime-host package requirements where those hooks can be added without changing runtime behavior.

#### Scenario: Existing package-like metadata maps without migration

- **WHEN** a supported existing skill or driver manifest is converted through the package descriptor hook
- **THEN** the result SHALL include package identity, package type, runtime kind, version, capabilities, permissions or service requirements when available, and trace/audit metadata
- **AND** existing skill, driver, and runtime-host loaders SHALL continue to compile and behave as before until explicitly migrated by later changes

### Requirement: Macaca SHALL document package runtime guard code with detailed English comments

Macaca SHALL include detailed English comments in new Phase 04 Rust code explaining package contracts, compatibility adapters, guard chain steps, trace/audit behavior, loader factory selection, optional service handling, commerce inert behavior, and compatibility limitations.

#### Scenario: Maintainer can understand package invariants from comments

- **WHEN** a maintainer reads the new package manifest and runtime guard modules
- **THEN** comments SHALL explain what each public type, trait, and guard step represents
- **AND** comments SHALL explain how trace, audit, permissions, compatibility, optional services, and loader selection invariants are protected
- **AND** comments SHALL explain which future capabilities are intentionally not implemented in Phase 04

### Requirement: Macaca SHALL preserve Route C Phase 04 regression baselines

Macaca SHALL implement Phase 04 additively without regressing YAML application loading or `/api/chat/v2` session creation behavior.

#### Scenario: Phase 04 baseline checks pass

- **WHEN** Phase 04 verification runs
- **THEN** the implementation SHALL preserve regression matrix scenarios `RC-APP-001` and `RC-CHAT-001`
- **AND** existing YAML application, trace, task board, resume, driver, skill/MCP, and current Web/CLI behavior SHALL continue to compile and run through existing paths until explicitly migrated by later changes
