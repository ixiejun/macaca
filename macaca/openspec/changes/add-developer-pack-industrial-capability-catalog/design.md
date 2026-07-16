# Developer Pack Industrial Capability Catalog Design

## Context

Macaca OS is positioning developer packs as the application developer surface for reusable Agent OS capabilities. The first platform work created descriptor contracts, resolution, unavailable diagnostics, SDK discovery, and a minimal reference catalog. This change turns that seed into an industrial catalog: broad enough for real application development, explicit about what is actually available, and structured so future pack families can be added without architectural rewrites.

## External Platform Research Baseline

The catalog model deliberately borrows stable ideas from mature application
platforms and translates them into Macaca's microkernel/service-runtime model:

- Apple developer documentation for entitlements and App Sandbox establishes the
  pattern that privileged host capabilities are declared separately from
  application code and checked before sensitive use.
- Microsoft Windows app capability declarations establish the pattern that
  package identity, manifest-visible capabilities, privacy-sensitive resources,
  device access, and restricted capabilities are explicit admission metadata.
- Android permission documentation establishes the pattern that dangerous or
  sensitive capabilities are declared and requested in context, with graceful
  degradation when the user denies or revokes access.
- HarmonyOS permission documentation establishes the pattern that application
  capability use is bound to declared permission metadata and host-side
  authorization.

Macaca adopts these as provider-neutral pack descriptors, permission scopes,
policy/approval gates, service command contracts, unavailable diagnostics, and
replayable trace/audit evidence. It does not copy platform-specific APIs or move
domain behavior into the microkernel.

## Goals

- Provide a production-oriented catalog of pack families and sub-packs that developers can discover and declare.
- Ensure a declared pack expands to actual service descriptors and typed service commands when implemented.
- Ensure absent or future packs return structured unavailable or preview diagnostics instead of fake success.
- Keep catalog growth data-driven and provider-neutral.
- Preserve the canonical execution path for every pack-backed call.
- Make pack metadata traceable, auditable, policy-aware, and SDK-readable.

## Non-Goals

- Do not implement application-specific product flows.
- Do not add concrete business provider logic to the kernel, SDK, shells, or base runtime-host.
- Do not introduce a second execution path for pack calls.
- Do not make optional package absence a startup failure.
- Do not promise that every listed future pack is immediately available.
- Do not implement all industrial sub-packs directly inside this umbrella change.

## Architectural Position

```text
Applications
  declare required/optional packs, permissions, policy bounds, and service usage

Application Framework / SDK
  validates declarations, resolves catalog entries, exposes discovery and invocation helpers

Developer Pack Industrial Catalog
  owns descriptor data, taxonomy, availability state, command schema references,
  examples, diagnostics, governance labels, compatibility and lifecycle metadata

System Services / Optional Packages / Plugins
  own concrete service descriptors, command handlers, provider adapters, health, snapshots

Service Runtime / Runtime Host
  composes descriptor-owned registrations, applies decorators, emits trace/audit evidence

Microkernel
  owns identity, policy facade, service-call evidence, trace/audit primitives only
```

## Design Patterns

- **Facade**: SDK pack catalog clients expose list, inspect, resolve, explain, and command-building APIs without provider construction.
- **Command**: each callable pack capability resolves to typed service command metadata and ultimately a canonical `ServiceCallCommand`.
- **Strategy**: catalog composition, availability resolution, version selection, and provider selection remain replaceable strategies.
- **Specification**: executable validators enforce pack id syntax, family hierarchy, lifecycle state, service-command mapping, permission scopes, and policy bounds.
- **Observer**: catalog load, resolution, provider health, unavailable states, policy decisions, and service calls emit sanitized events.
- **Memento**: effective capability reports, catalog snapshots, and provider snapshots are replayable audit records.
- **Abstract Factory**: optional packages and plugins register service providers through descriptor-owned factories; runtime-host consumes registrations generically.

## Catalog Model

Each pack entry is immutable descriptor data:

- `pack_id`: stable identifier such as `pack.developer.repository.v1`.
- `family_id`: broad family such as `developer`, `knowledge`, `workflow`, `office`, or `commerce`.
- `parent_pack_id`: optional parent for inheritance and discovery grouping.
- `lifecycle`: `available`, `preview`, `unavailable`, `deprecated`, or `retired`.
- `stability`: `experimental`, `preview`, `stable`, `deprecated`, or `retired`.
- `services`: service ids that already exist in the service registry or optional package descriptors.
- `commands`: command schema references, result schema references, examples, and policy hints.
- `permissions`: app-scoped permission scopes required before side effects.
- `policy_template`: timeout, retry, budget, resource, network, entitlement, and approval defaults.
- `data_governance`: data classification, retention, redaction, replay, and snapshot policy.
- `sdk`: client namespace, generated helper metadata, examples, docs pointer, and version lane.
- `diagnostics`: health probes, unavailable reasons, provider snapshot shape, and replay schema refs.
- `compatibility`: pack version ranges, parent ranges, service version ranges, and migration notes.

The catalog can contain planned or future packs only when their lifecycle is explicit and discovery surfaces report that they are not callable. A callable pack MUST map to at least one admitted service command.

## Umbrella And Child Proposal Model

This change is the umbrella proposal for catalog governance, shared contracts,
SDK discovery, admission, composition, and executable gates. Every concrete
sub-pack is implemented through a dedicated child OpenSpec proposal listed in
`child-proposals.md` and tracked in `tasks.md`.

Each child proposal must define:

- the pack id and family/sub-pack relationship;
- the service descriptors and typed commands that make the pack callable;
- permission scopes, policy defaults, entitlement/resource behavior, and approval rules;
- SDK discovery metadata and examples;
- provider ownership, optional package/plugin boundaries, and unavailable behavior;
- trace/audit event names, replay evidence, health checks, and snapshots;
- tests proving canonical service-path invocation and no OS-layer business branches.

The umbrella proposal is complete only when the shared catalog machinery is implemented and every required child proposal has either landed as an available implementation or is explicitly scoped as preview/unavailable with its own approved OpenSpec rationale.

## Industrial Pack Families

The initial industrial taxonomy is broad but declarative. It does not require all providers to ship in base OS.

| Family | Representative sub-packs | Ownership rule |
| --- | --- | --- |
| `foundation` | filesystem, key-value state, time, random, config, secrets reference, session state | Only generic host/service primitives; no app workflow semantics |
| `communication` | email, messaging, notification, inbox, calendar | Gateway/communication services own transport |
| `knowledge` | search, retrieval, document parsing, citations, graph, summarization | Memory/context/LLM/task/search services own execution |
| `developer` | code, repository, CI, issue tracker, terminal, browser automation, design tools | Existing tooling services or optional providers; SDK does not construct tools |
| `office` | document, spreadsheet, presentation, PDF, forms | Optional providers or document services own formats |
| `media` | image, audio, video, transcription, rendering | Optional media services own codecs/providers |
| `finance` | market data, stock, crypto, accounting, portfolio, invoice | Optional finance package owns provider logic |
| `commerce` | catalog, cart, order, payment intent, receipt, entitlement | Store/payment/entitlement services own commerce semantics |
| `identity` | account, profile, auth handoff, organization, tenant | Identity/entitlement services own authorization |
| `location` | maps, geocode, route, place search, timezone | Optional location providers own external APIs |
| `device` | sensors, camera, local files, notifications, foreground/background host capabilities | Host/device providers own privileged access |
| `ai` | LLM, embedding, rerank, vision, speech, model evaluation | Intelligence services own model routing and policy |
| `workflow` | task, schedule, approval, delegation, review, recovery | Autonomy/task services own state machines |

Adding later families such as `health`, `education`, `manufacturing`, or `legal` must add catalog descriptors and optional providers only. It must not add hardcoded OS-layer branches.

## Availability Semantics

Pack discovery returns one of these states:

- `available`: services and commands are registered and policy-admissible.
- `degraded`: optional services are missing but required commands remain callable.
- `preview`: descriptor exists but compatibility or provider coverage is not stable.
- `unavailable`: descriptor exists but providers or entitlements are absent.
- `unsupported`: descriptor is not recognized by the active catalog.
- `retired`: descriptor exists only for migration diagnostics.

Required unavailable packs block readiness. Optional unavailable packs produce degraded diagnostics and cannot be invoked unless a concrete service command remains declared and policy-allowed.

## Invocation Flow

1. Application manifest declares `required_packs`, `optional_packs`, direct services, permissions, and policy overrides.
2. Admission validates pack syntax, lifecycle, version constraints, permissions, and policy bounds.
3. Catalog resolver expands packs to service ids, command schemas, source attribution, and unavailable diagnostics.
4. SDK returns an effective capability memento with a stable hash and replay references.
5. Runtime invocation builds a typed service command only for commands present in the effective capability set.
6. The service runtime applies policy, entitlement, resource, trace, audit, and data-governance decorators before side effects.
7. Provider results return through structured service results and sanitized events.

## Trace And Audit

Every catalog and invocation step emits bounded evidence:

- `pack_catalog_composed`
- `pack_catalog_entry_loaded`
- `pack_catalog_entry_unavailable`
- `pack_declaration_validated`
- `pack_declaration_rejected`
- `pack_resolved`
- `pack_resolution_degraded`
- `pack_provider_snapshot_recorded`
- `pack_policy_decision`
- `pack_service_call_requested`
- `pack_service_call_succeeded`
- `pack_service_call_failed`

Events include pack id, family id, lifecycle state, service id, command name, application id, session id, trace id, policy decision, capability hash, provider class, bounded error code, and latency. Events must not include raw secrets, prompts, manifests, WASM bytes, package bytes, private keys, credentials, raw signatures, raw provider payloads, or unbounded user content.

## Implementation Approach

1. Extend catalog descriptor data first; do not add concrete providers in the same step.
2. Create or approve one child OpenSpec proposal per sub-pack before implementing that sub-pack.
3. Add industrial family/sub-pack builders as data-only descriptor modules split by family to stay below file-size limits.
4. Add mapping validators that require callable entries to point at known service descriptors and command schemas.
5. Extend SDK discovery results with availability, examples, command schemas, and diagnostics.
6. Add application admission checks for required versus optional industrial packs.
7. Add runtime-host composition hooks that merge base descriptors, optional package descriptors, and plugin descriptors without business branches.
8. Add tests and gates proving no optional domain package imports leak into kernel, SDK, shells, or base runtime-host.

## Risks And Mitigations

- Risk: catalog becomes a hardcoded business registry.
  Mitigation: descriptor-only data in proto/application catalog layers; providers remain optional packages/plugins.
- Risk: broad taxonomy implies unavailable capabilities are working.
  Mitigation: explicit lifecycle and availability states; callable entries must map to admitted service commands.
- Risk: command schemas drift from actual service implementations.
  Mitigation: executable descriptor compatibility tests and service-command mapping gates.
- Risk: SDK helper APIs bypass service runtime.
  Mitigation: helpers only build canonical service commands and no-direct-provider-call gates remain mandatory.
- Risk: catalog files become oversized.
  Mitigation: split family descriptors into focused modules and keep executable gates for source-size limits.

## Open Questions

- Which industrial families should be promoted to stable documentation first after foundation, developer, and knowledge?
- Should preview/unavailable planned packs live in the same catalog as available packs or in a separate marketplace-style discovery lane?
- Should generated SDK helper names be persisted in catalog descriptors or derived from pack ids during SDK build?
