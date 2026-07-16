## ADDED Requirements

### Requirement: Industrial catalog implementation SHALL be decomposed into child OpenSpec proposals

The industrial catalog umbrella SHALL track one dedicated child OpenSpec proposal for every required sub-pack before that sub-pack is implemented as an available capability.

#### Scenario: Sub-pack implementation starts
- **WHEN** maintainers start implementing an individual sub-pack such as filesystem, email, repository, PDF, market data, maps, LLM, or task
- **THEN** there SHALL be a child OpenSpec proposal for that sub-pack
- **AND** the child proposal SHALL define service ownership, typed commands, permissions, policy, SDK metadata, trace/audit evidence, unavailable behavior, tests, and boundary gates

#### Scenario: Umbrella tracks sub-pack progress
- **WHEN** the industrial catalog umbrella tasks are reviewed
- **THEN** every required sub-pack SHALL appear as a child proposal task
- **AND** family-level tasks SHALL group child proposals but SHALL NOT replace the child proposal requirement
- **AND** the umbrella SHALL NOT mark the catalog complete by adding shallow descriptor entries without child proposal coverage

#### Scenario: Child proposal is implementation-grade
- **WHEN** a child proposal is written for a sub-pack
- **THEN** its tasks SHALL be detailed enough that completing them produces a genuinely usable industrial pack or an explicitly justified preview/unavailable pack
- **AND** it SHALL specify service contracts, provider boundaries, admission behavior, SDK usage, trace/audit evidence, diagnostics, tests, and boundary gates for that sub-pack

### Requirement: Macaca SHALL expose an industrial developer pack catalog

Macaca SHALL expose a provider-neutral industrial developer pack catalog that lets developers discover and declare broad production-oriented pack families while concrete execution remains owned by system services, optional packages, plugins, or provider adapters.

#### Scenario: Developer discovers industrial pack families
- **WHEN** a developer or shell lists pack families
- **THEN** Macaca SHALL return descriptor-derived pack family and sub-pack metadata
- **AND** each entry SHALL include lifecycle, availability, permissions, service mappings, diagnostics, and SDK metadata
- **AND** the response SHALL NOT expose provider secrets, raw package bytes, raw manifests, raw prompts, or unbounded provider payloads

#### Scenario: Catalog growth avoids OS business branches
- **WHEN** maintainers add a new pack family or sub-pack
- **THEN** they SHALL add descriptor data and optional provider/plugin registrations
- **AND** they SHALL NOT add application-specific, provider-specific, or business-domain routing branches to the kernel, SDK, shells, or base runtime-host

### Requirement: Industrial pack entries SHALL distinguish callable and non-callable capabilities

Industrial pack entries SHALL explicitly distinguish callable capabilities from preview, unavailable, unsupported, deprecated, and retired capabilities.

#### Scenario: Callable pack maps to actual service commands
- **WHEN** a pack entry is marked callable or available
- **THEN** it SHALL map to at least one admitted service descriptor and typed command schema
- **AND** invocation SHALL build a canonical traced service command through the SDK/facade path

#### Scenario: Planned pack is not implemented
- **WHEN** a catalog entry describes a planned or preview capability without an installed provider
- **THEN** Macaca SHALL report the entry as preview or unavailable with structured diagnostics
- **AND** it SHALL NOT allow invocation, silently fall back, or fake success

### Requirement: Industrial pack taxonomy SHALL be broad and extensible

Macaca SHALL define an extensible industrial pack taxonomy covering foundational and common application-development capability families without making those families base OS dependencies.

#### Scenario: Initial industrial taxonomy is queried
- **WHEN** the active catalog is queried
- **THEN** it SHALL be able to describe foundation, communication, knowledge, developer, office, media, finance, commerce, identity, location, device, AI, and workflow families
- **AND** each family SHALL be represented as descriptor data with explicit availability

#### Scenario: Required initial sub-pack set is present
- **WHEN** the industrial catalog is inspected
- **THEN** the `foundation` family SHALL describe filesystem, key-value state, time, random, config, secrets reference, and session state sub-packs
- **AND** the `communication` family SHALL describe email, messaging, notification, inbox, and calendar sub-packs
- **AND** the `knowledge` family SHALL describe search, retrieval, document parsing, citations, graph, and summarization sub-packs
- **AND** the `developer` family SHALL describe code, repository, CI, issue tracker, terminal, browser automation, and design tools sub-packs
- **AND** the `office` family SHALL describe document, spreadsheet, presentation, PDF, and forms sub-packs
- **AND** the `media` family SHALL describe image, audio, video, transcription, and rendering sub-packs
- **AND** the `finance` family SHALL describe market data, stock, crypto, accounting, portfolio, and invoice sub-packs
- **AND** the `commerce` family SHALL describe catalog, cart, order, payment intent, receipt, and entitlement sub-packs
- **AND** the `identity` family SHALL describe account, profile, auth handoff, organization, and tenant sub-packs
- **AND** the `location` family SHALL describe maps, geocode, route, place search, and timezone sub-packs
- **AND** the `device` family SHALL describe sensors, camera, local files, notifications, and foreground/background host capabilities sub-packs
- **AND** the `ai` family SHALL describe LLM, embedding, rerank, vision, speech, and model evaluation sub-packs
- **AND** the `workflow` family SHALL describe task, schedule, approval, delegation, review, and recovery sub-packs

#### Scenario: Future vertical family is added
- **WHEN** maintainers add a future vertical family such as health, education, manufacturing, or legal
- **THEN** the addition SHALL use the same descriptor, admission, discovery, trace, and optional provider mechanisms
- **AND** no microkernel or shell semantic ownership change SHALL be required

### Requirement: Pack declarations SHALL expand to effective callable capabilities

Application pack declarations SHALL resolve into deterministic effective capability reports that separate callable services from unavailable, degraded, unsupported, deprecated, and retired pack entries.

#### Scenario: Required industrial pack is unavailable
- **WHEN** an application declares an unavailable industrial pack as required
- **THEN** admission or readiness SHALL return a structured blocking diagnostic
- **AND** the application SHALL NOT execute that capability through an undeclared fallback provider

#### Scenario: Optional industrial pack is unavailable
- **WHEN** an application declares an unavailable industrial pack as optional
- **THEN** admission SHALL record a degraded diagnostic
- **AND** execution MAY continue only for remaining declared, callable, and policy-allowed capabilities

### Requirement: SDK discovery SHALL explain service-backed pack usage

SDK pack discovery SHALL explain how declared packs map to service-backed commands, policy bounds, examples, availability, and replay-safe diagnostics.

#### Scenario: Developer inspects a callable pack
- **WHEN** a developer inspects an available pack
- **THEN** the SDK SHALL return service ids, command names, command schema refs, result schema refs, permission scopes, policy template, examples, lifecycle state, availability, diagnostics, and docs references
- **AND** the SDK SHALL NOT construct or expose concrete providers

#### Scenario: Developer inspects unavailable pack
- **WHEN** a developer inspects an unavailable pack
- **THEN** the SDK SHALL return bounded unavailable reasons and remediation hints
- **AND** it SHALL NOT expose raw provider payloads or raw package manifests

### Requirement: Industrial pack operations SHALL remain traceable and auditable

Catalog composition, declaration validation, pack resolution, provider snapshots, policy decisions, service-call requests, service-call outcomes, and unavailable states SHALL emit sanitized trace and audit evidence.

#### Scenario: Operator replays industrial pack invocation
- **WHEN** an operator replays a session containing an industrial pack-backed call
- **THEN** replay SHALL show pack id, family id, lifecycle state, service id, command name, application id, session id, trace id, policy decision, capability hash, provider class, bounded status metadata, and latency
- **AND** replay SHALL NOT include raw secrets, prompts, manifests, WASM bytes, package bytes, private keys, credentials, raw signatures, raw provider payloads, or unbounded user content

### Requirement: Industrial pack composition SHALL preserve serviceization boundaries

Industrial pack composition SHALL merge base descriptor data, optional package descriptor data, and plugin descriptor data through generic composition hooks while preserving serviceization boundaries.

#### Scenario: Optional pack package is absent
- **WHEN** an optional package that could provide industrial pack services is absent
- **THEN** the base OS SHALL still start and expose structured unavailable diagnostics
- **AND** kernel, SDK, shells, and base runtime-host SHALL NOT import that optional package

#### Scenario: Optional pack package is installed
- **WHEN** an optional package registers industrial pack services
- **THEN** it SHALL register through descriptor-owned service provider factories
- **AND** calls SHALL traverse the canonical service runtime path with trace, policy, health, snapshot, and structured error behavior
