# Developer Pack Capability Platform Design

## Context

Macaca OS is an Agent OS for third-party applications. The operating system must give developers enough reusable capability packs to build rich applications, but it must not hardcode application logic, provider names, or business workflows into the kernel, SDK, web shell, or base runtime host. The current code already has a provider-neutral `DomainPackDefinition`, `DomainPackCatalog`, manifest `service_contract.use_packs`, and optional `macaca-domain-pack-finance` package. This change turns that seed into a broader pack platform.

## Goals

- Make packs a stable developer-facing abstraction for grouped service capabilities.
- Support families and sub-packs such as `pack.finance.v1`, `pack.finance.stock.v1`, and `pack.finance.crypto.v1` without changing OS source for each new domain.
- Preserve the canonical execution path: `SystemFacade`/SDK client -> `ServiceRouter` -> `ServiceRuntime` -> `ServiceBus` -> `SystemService`.
- Keep all pack implementations optional, serviceized, traceable, auditable, replaceable, and policy-gated.
- Let the initial catalog be incomplete while making future additions data-driven.
- Provide SDK and tooling discovery so application developers can inspect available packs, required permissions, command schemas, examples, health, and unavailable reasons.

## Non-Goals

- Do not add app-specific flows, stock-specific logic, payment-provider logic, chain-specific logic, or UI-specific branches to OS layers.
- Do not compile business-domain providers into `macaca-runtime-host`.
- Do not introduce a second execution path for packs.
- Do not require all future pack families to exist in the first release.

## Operating-System Lessons

MacOS, Windows, Android, and HarmonyOS expose developer power through kits/frameworks/APIs backed by declarations, permissions, lifecycle management, distribution metadata, diagnostics, and compatibility rules. Macaca should express the same idea in Agent OS terms:

- Kits become packs and sub-packs.
- Entitlements and permissions become declared capability scopes plus policy decisions.
- App lifecycle hooks become application/session/task/service lifecycle commands.
- Platform services become `SystemService` descriptors and command schemas.
- Store/distribution metadata becomes package, entitlement, version, and compatibility metadata.
- Diagnostics become trace, audit, health, snapshot, and replay surfaces.

## Architecture

```text
Applications
  declare packs, sub-packs, direct services, permissions, and policy bounds

Application Framework / SDK
  resolves pack declarations, validates admission, exposes typed discovery clients

Developer Pack Platform
  owns catalog contracts, family taxonomy, version ranges, capability scopes,
  policy templates, command schemas, compatibility metadata, diagnostics

System Services / Optional Packages
  implement replaceable service providers for installed packs

Service Runtime / Runtime Host
  registers descriptor-owned providers, applies decorators, emits audit evidence

Microkernel
  owns service/capability identity, policy facade, trace/audit primitives only
```

## Pack Model

The pack platform should evolve `DomainPackDefinition` into a richer immutable value object:

- `pack_id`: stable id, for example `pack.finance.stock.v1`.
- `family_id`: broad family such as `finance`, `office`, `media`, `developer`, `data`, `device`, `identity`, `commerce`, `communication`, `location`, `ai`, `knowledge`, or `workflow`.
- `parent_pack_id`: optional parent for sub-pack trees.
- `version`: semantic version or declared compatibility lane.
- `stability`: `experimental`, `preview`, `stable`, `deprecated`, `retired`.
- `services`: service ids with version ranges and command schemas.
- `permissions`: capability scopes required before side effects.
- `policy_template`: default timeout, retry, resource, budget, data, network, and entitlement bounds.
- `data_governance`: classification, retention, redaction, and audit snapshot strategy.
- `sdk_surface`: generated client namespace, examples, and docs pointer.
- `diagnostics`: health probes, unavailable reasons, and replay schema refs.

## Pack Family Taxonomy

Initial taxonomy should be broad but not exhaustive. Families are identifiers and governance categories, not mandatory built-in implementations.

- `foundation`: filesystem, key-value state, time, random, config, secrets reference, session state.
- `communication`: email, messaging, notification, inbox, calendar.
- `knowledge`: search, retrieval, document parsing, citations, graph, summarization.
- `developer`: code, repository, CI, issue tracker, terminal, browser automation, design tools.
- `office`: document, spreadsheet, presentation, PDF, forms.
- `media`: image, audio, video, transcription, rendering.
- `finance`: market data, stock, crypto, accounting, portfolio, invoice.
- `commerce`: catalog, cart, order, payment intent, receipt, entitlement.
- `identity`: account, profile, auth handoff, organization, tenant.
- `location`: maps, geocode, route, place search, timezone.
- `device`: sensors, camera, local files, notifications, foreground/background host capabilities.
- `ai`: LLM, embedding, rerank, vision, speech, model evaluation.
- `workflow`: task, schedule, approval, delegation, review, recovery.

The taxonomy is open. Adding `pack.health.*`, `pack.education.*`, or `pack.manufacturing.*` later should add catalog data and optional providers, not modify core routing.

## Design Patterns

- **Facade**: SDK exposes focused pack discovery and service clients; shells and applications do not read runtime internals.
- **Command**: pack operations resolve to typed `ServiceCommand` and structured `ServiceCallResult`.
- **Adapter / Bridge**: optional package providers translate external APIs into canonical service contracts.
- **Strategy**: provider selection, version resolution, policy merge, and unavailable behavior are replaceable strategies.
- **Decorator**: trace, policy, resource, entitlement, metering, and data-governance checks wrap service calls.
- **State**: pack lifecycle models installed, enabled, disabled, deprecated, unavailable, degraded, and retired states.
- **Observer**: pack catalog changes, provider health, service calls, and admission decisions emit events.
- **Memento**: capability expansion reports and audit records are replayable.
- **Specification**: admission gates validate pack id syntax, version constraints, permissions, policy bounds, and declared service schemas.
- **Abstract Factory**: package bootstrap factories create provider registrations; runtime-host only consumes descriptor-owned registrations.

## Resolution Flow

1. Application manifest declares `use_packs`, `required_services`, `optional_services`, permissions, and policy bounds.
2. Application admission resolves packs through the installed pack catalog.
3. The resolver expands parent/sub-pack services, applies version constraints, and records unresolved or incompatible packs.
4. Policy evaluates platform, tenant, application, entitlement, and pack defaults.
5. The application receives an effective capability set with a stable hash.
6. Runtime calls are allowed only if the service id and command are present in the effective set and policy allows the operation.
7. Missing optional packs return structured unavailable/degraded diagnostics; missing required packs block execution.

## Trace And Audit

Every pack-related critical node must emit sanitized evidence:

- `pack_catalog_loaded`
- `pack_resolved`
- `pack_resolution_failed`
- `pack_provider_registered`
- `pack_provider_started`
- `pack_policy_decision`
- `pack_service_call_requested`
- `pack_service_call_succeeded`
- `pack_service_call_failed`
- `pack_unavailable`

Events include pack id, service id, application id, session id, tenant id when available, trace id, decision code, provider class, version, capabilities hash, latency, retry count, and bounded error code. Events must not include raw secrets, prompts, manifests, package bytes, provider payloads, private keys, raw signatures, or unbounded user content.

## Initial Pack Roadmap

The first implementation should prioritize the platform skeleton before broad pack coverage:

1. Foundation pack metadata and examples for safe host primitives that already exist.
2. Developer pack metadata for repository, terminal, browser, issue, and design tool capabilities routed through existing service boundaries.
3. Knowledge pack metadata for search, context, memory, document parsing, and citation services.
4. Finance pack normalization as the first domain-pack reference, with sub-pack metadata split from provider implementation.

Later waves can add office, media, communication, commerce, identity, location, device, AI, and workflow packs through the same catalog/provider interfaces.

## Risks And Mitigations

- Risk: pack taxonomy becomes a hardcoded business registry.
  Mitigation: store taxonomy as catalog metadata and governance docs; OS code validates shape only.
- Risk: developers depend on unstable preview packs.
  Mitigation: explicit stability states, compatibility reports, and admission warnings.
- Risk: sub-pack inheritance creates confusing permissions.
  Mitigation: flattened effective capability reports with source attribution and policy explain output.
- Risk: pack APIs bypass canonical service path.
  Mitigation: no-direct-provider-call gate and unified-execution-path tests.
- Risk: catalog grows faster than documentation.
  Mitigation: SDK discovery surfaces generated from descriptors and examples.

## Open Questions

- Should stable pack ids use `pack.<family>.<subpack>.vN` only, or allow semantic versions inside manifest version ranges?
- Which pack families should be first-class documentation sections versus optional marketplace categories?
- Should generated SDK clients be built during package installation or generated lazily from descriptors?
