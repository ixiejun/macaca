# Change: Add Developer Pack Industrial Capability Catalog

## Why

The developer pack platform now has a provider-neutral contract, SDK discovery surface, and a small reference catalog, but the catalog is not yet broad enough for developers to build production-grade applications by declaring packs and then invoking the actual service capabilities behind those declarations. Macaca needs an industrial capability catalog that expands foundational, developer, knowledge, workflow, data, media, office, communication, commerce, identity, location, device, AI, and vertical pack families while preserving microkernel and serviceization boundaries.

## What Changes

- Define `developer-pack-industrial-capability-catalog` as an umbrella catalog governance and implementation track layered on top of the existing `developer-pack-platform`.
- Require every sub-pack to have its own OpenSpec child proposal before implementation so each capability receives a dedicated design, service boundary analysis, task plan, trace/audit contract, unavailable behavior, and verification gates.
- Cover the required initial industrial pack set:
  `foundation` (filesystem, key-value state, time, random, config, secrets reference, session state),
  `communication` (email, messaging, notification, inbox, calendar),
  `knowledge` (search, retrieval, document parsing, citations, graph, summarization),
  `developer` (code, repository, CI, issue tracker, terminal, browser automation, design tools),
  `office` (document, spreadsheet, presentation, PDF, forms),
  `media` (image, audio, video, transcription, rendering),
  `finance` (market data, stock, crypto, accounting, portfolio, invoice),
  `commerce` (catalog, cart, order, payment intent, receipt, entitlement),
  `identity` (account, profile, auth handoff, organization, tenant),
  `location` (maps, geocode, route, place search, timezone),
  `device` (sensors, camera, local files, notifications, foreground/background host capabilities),
  `ai` (LLM, embedding, rerank, vision, speech, model evaluation), and
  `workflow` (task, schedule, approval, delegation, review, recovery).
- Expand pack taxonomy from a minimal seed into production-oriented pack families and sub-packs with stable identifiers, lifecycle state, permissions, command schemas, SDK metadata, diagnostics, and data-governance rules.
- Track child proposal IDs in the umbrella tasks so implementation cannot collapse many rich packs into one shallow catalog update.
- Require each catalog entry to map to a real service descriptor, optional package/plugin provider, or explicit unavailable/preview diagnostic; catalog metadata MUST NOT fake implemented capability.
- Add a data-driven catalog composition model so new packs can be added without kernel, SDK, shell, or base runtime-host business branches.
- Add SDK-facing discovery requirements that explain available commands, unavailable reasons, policy bounds, examples, and trace/audit replay handles for every pack.
- Add executable gates proving pack declarations expand only to declared service calls through the canonical service path.

## Impact

- Affected specs: `developer-pack-industrial-capability-catalog`, `developer-pack-platform`, `sdk-system-facade`, `service-runtime`, `serviceization-dependency-gate`, `unified-execution-path`.
- Affected code later: `macaca-proto` domain-pack catalog descriptors, `macaca-sdk` pack discovery DTOs/clients, `macaca-app` manifest admission and effective capability reports, `macaca-runtime-host` generic catalog composition, optional package/plugin descriptor crates, integration gates.
- Non-goal: adding application-specific behavior, hardcoding workflow/business rules in OS layers, or marking unavailable domain capabilities as working services.

## Governance Notes

- The microkernel owns only pack and service identity, trace/audit primitives, policy facade decisions, and registry invariants.
- System services, optional packages, and plugins own concrete capabilities.
- SDK and shells expose catalog discovery and invocation helpers through provider-neutral facades.
- Applications own product behavior and choose packs through manifest declarations.
- GitNexus CRITICAL/HIGH findings during implementation are recorded as memo only per user instruction; they do not block this proposal, but boundary tests remain mandatory.
