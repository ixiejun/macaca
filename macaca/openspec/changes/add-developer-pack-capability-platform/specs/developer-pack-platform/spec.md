## ADDED Requirements

### Requirement: Macaca SHALL expose a developer pack platform

Macaca SHALL expose a provider-neutral developer pack platform that lets applications declare reusable capability packs and sub-packs while all concrete execution remains owned by system services, optional packages, plugins, or provider adapters.

#### Scenario: Application declares packs without OS business branches
- **WHEN** an application manifest declares one or more pack ids
- **THEN** Macaca SHALL resolve those declarations through the installed pack catalog
- **AND** kernel, SDK, presentation shells, and base runtime-host SHALL NOT branch on application-specific, provider-specific, or business-domain-specific names

#### Scenario: Pack execution uses the canonical service path
- **WHEN** an application invokes a service expanded from a pack
- **THEN** the invocation SHALL be represented as a typed service command
- **AND** it SHALL traverse the canonical service path defined by `unified-execution-path`
- **AND** it SHALL carry trace context and policy scope before side effects

### Requirement: Pack definitions SHALL support families and sub-packs

Pack definitions SHALL support a versioned family/sub-pack hierarchy so broad packs and narrower sub-packs can be added incrementally without OS architecture changes.

#### Scenario: Sub-pack is added after the parent family
- **WHEN** maintainers add a new sub-pack such as `pack.finance.stock.v1` under an existing family
- **THEN** they SHALL add catalog metadata and optional provider registrations
- **AND** they SHALL NOT modify microkernel routing, shell routing, or base runtime-host business branches

#### Scenario: Initial catalog is incomplete
- **WHEN** an application declares a pack that is not installed or not yet supported
- **THEN** Macaca SHALL return a structured unresolved, incompatible, or unavailable diagnostic
- **AND** it SHALL NOT crash, hang, silently fall back, or fake success

### Requirement: Pack catalog entries SHALL be descriptor-driven

Pack catalog entries SHALL be immutable descriptor data carrying pack id, family id, parent id when present, version, stability, service contracts, permission scopes, policy template, data governance, SDK metadata, diagnostics metadata, and compatibility metadata.

#### Scenario: Developer tooling inspects an installed pack
- **WHEN** a developer or shell asks to inspect installed packs
- **THEN** the SDK SHALL return descriptor-derived metadata including services, commands, permissions, stability, version, health, and documentation references
- **AND** the SDK SHALL NOT expose provider secrets, raw package bytes, raw manifests, raw prompts, or unbounded provider payloads

### Requirement: Pack admission SHALL use executable specifications

Application admission SHALL validate pack ids, version constraints, required versus optional packs, service command schemas, permission scopes, and policy bounds using executable specifications before runtime execution.

#### Scenario: Required pack cannot be resolved
- **WHEN** an application declares a required pack that is absent or incompatible
- **THEN** admission or execution readiness SHALL return a structured blocking diagnostic
- **AND** the application SHALL NOT execute through an undeclared fallback provider

#### Scenario: Optional pack cannot be resolved
- **WHEN** an application declares an optional pack that is absent or incompatible
- **THEN** admission SHALL record a degraded diagnostic
- **AND** execution MAY continue only for capabilities that remain explicitly declared and policy-allowed

### Requirement: Pack providers SHALL remain optional service providers

Concrete pack providers SHALL live in optional package or plugin crates and SHALL register through descriptor-owned service provider registrations. Base `macaca-runtime-host` SHALL keep only generic registration, lifecycle, decorator, health, snapshot, and unavailable mechanics.

#### Scenario: Base runtime host starts without optional packs
- **WHEN** Macaca starts with no optional pack providers installed
- **THEN** base services SHALL start normally
- **AND** pack-backed service calls SHALL return structured unavailable diagnostics
- **AND** no optional pack absence SHALL change base OS semantics

### Requirement: Pack operations SHALL emit sanitized trace and audit evidence

Pack catalog load, resolution, admission, provider registration, policy decisions, service calls, failures, and unavailable states SHALL emit sanitized trace and audit evidence.

#### Scenario: Pack service call is replayed
- **WHEN** operators replay a session containing a pack-backed service call
- **THEN** the replay SHALL show pack id, service id, application id, session id, trace id, policy decision, provider class, version, capabilities hash, and bounded status metadata
- **AND** it SHALL NOT include raw secrets, prompts, manifests, package bytes, private keys, raw signatures, raw provider payloads, or unbounded user content

### Requirement: Pack SDK surface SHALL be facade-owned

SDK pack discovery and invocation helpers SHALL be facade-owned clients over provider-neutral descriptors and service commands. SDK clients SHALL NOT construct concrete pack providers.

#### Scenario: Shell lists available packs
- **WHEN** Web, CLI, or another shell lists available packs
- **THEN** it SHALL call SDK/facade pack discovery
- **AND** it SHALL render descriptor data and diagnostics only
- **AND** it SHALL NOT import optional package crates or runtime-host internals
