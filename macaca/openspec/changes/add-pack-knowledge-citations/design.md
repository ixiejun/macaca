# Knowledge Citations Pack Design

## Context

`pack.knowledge.citations.v1` exposes citation capture, source anchoring,
identifier resolution, bibliography formatting, and evidence verification as a
Macaca OS serviceized capability. It lets applications attach verifiable
provenance to claims and outputs without embedding provider-specific Crossref,
DataCite, Zotero, CSL, or annotation logic into generic OS layers.

Citations often point into private documents, web pages, datasets, source code,
messages, or generated evidence bundles. The pack must preserve source anchors
and metadata while avoiding raw source leaks. It therefore uses typed commands,
redaction profiles, policy gates, trace/audit events, and replayable source
selectors.

## Supplier Capability Matrix

| Supplier/standard | Industrial capability | Macaca abstraction |
| --- | --- | --- |
| Citation Style Language | Citation item schema, style files, bibliography rendering, locales | Citation item, bibliography style, locale/style compatibility, formatted citation/bibliography |
| Crossref | DOI metadata, references, funders, relations, licenses, updates | Identifier resolution, bibliographic enrichment, related references, license metadata, stale check |
| DataCite | DOI metadata for datasets/software/texts, related identifiers, versions, rights | Research/data/software citation metadata, related-resource verification, version metadata |
| W3C Web Annotation | Targets, bodies, motivations, text quote/position/fragment selectors, states | Source target, citation body, selector, source span anchor, quote selector, source state |
| Zotero/reference managers | Items, creators, libraries, collections, attachments, tags, relations, export | Citation library, item collection, attachment/source handle, tags, import/export |

## Goals

- Provide stable pack id `pack.knowledge.citations.v1` and command namespace
  `citations.*`.
- Support citation creation, identifier resolution, metadata enrichment, source
  span linking, quote anchoring, citation verification, bibliography formatting,
  import/export, source-anchor inspection, and provider capability inspection.
- Model CSL-compatible metadata, DOI/URL/ISBN/arXiv-like identifiers,
  contributors, source selectors, evidence bundles, quote policy, license
  metadata, freshness/staleness, verification status, and formatted outputs.
- Keep identifier/provider-specific resolution logic behind replaceable
  provider adapters.
- Require developer documentation under
  `docs/developer-packs/knowledge/citations.md`.

## Non-Goals

- Do not implement concrete Crossref, DataCite, Zotero, CSL rendering, browser,
  or document-provider adapters in this proposal.
- Do not perform retrieval, parsing, graph extraction, or summarization; those
  packs provide source material and citations attach provenance to it.
- Do not expose raw source documents, raw provider payloads, raw private quotes,
  credentials, raw bibliography style files, or unbounded source snippets in
  logs, traces, snapshots, SDK diagnostics, or examples.
- Do not let shell UI own verification semantics or source-span repair.

## Ownership And Boundaries

- Pack id: `pack.knowledge.citations.v1`.
- Family: `knowledge`.
- Backing service owner: citation service provider.
- SDK surface: `sdk.packs.knowledge.citations`.
- Command namespace: `citations.*`.
- Microkernel owns identity, policy facade, resource primitives, service-call
  evidence, trace/audit primitives, and registry metadata only.
- Application framework owns manifest declarations, app-scoped permissions, and
  effective capability projection.
- Runtime host owns provider adapter registration, style engine bridge
  composition, identifier resolver bridge composition, and sanitized diagnostics
  through approved composition roots.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `citations.create_citation` | Create citation item from metadata, identifier, or source handle | Requires idempotency, metadata validation, and source policy |
| `citations.resolve_identifier` | Resolve DOI/URL/ISBN/arXiv-like identifiers through provider adapters | Returns typed metadata or unavailable/unsupported diagnostics |
| `citations.link_source_span` | Link citation to source page/text/DOM/range/fragment selector | Requires selector validation and source permission |
| `citations.verify_citation` | Verify identifier reachability, source anchor stability, metadata freshness, and quote match | Returns verification status and bounded evidence |
| `citations.format_citation` | Render one or more citations in a style/locale/output format | Requires style capability and redaction policy |
| `citations.format_bibliography` | Render bibliography from ordered citation items | Requires bibliography permission, style validation, and bounded output |
| `citations.list_citations` | List citations by collection, source, claim, evidence bundle, or tag | Returns bounded pages and redacted metadata |
| `citations.update_citation` | Update metadata, tags, notes, or source links | Requires optimistic concurrency and audit reason |
| `citations.import_citations` | Import CSL JSON, BibTeX/RIS-like metadata, or provider library references | Requires validation and conversion diagnostics |
| `citations.export_citations` | Export citation collection to supported metadata format | Requires export permission and redaction profile |
| `citations.inspect_source_anchor` | Inspect selector validity, quote match, and source state | Must not expose raw private source text beyond policy |
| `citations.inspect_provider` | Inspect resolver/style/import/export capability | Returns bounded metadata only |

## DTO Model

Core DTOs:

- `CitationItem`: citation handle, item type, title, contributors, issued date,
  publisher/container, edition/version, identifiers, URL handle, license, tags,
  notes handle, source anchors, metadata provenance, and version hash.
- `CitationIdentifier`: scheme, normalized value, resolver capability, checksum,
  resolution state, and last verified timestamp.
- `CitationContributor`: name parts, organization name, role, identifier handle,
  and ordering metadata.
- `CitationSourceAnchor`: source handle, target kind, selector set, source state
  hash, quote policy, page/section anchors, offsets, and replay pointer.
- `CitationSelector`: text quote, text position, fragment, page, DOM, byte
  range, media timestamp, or custom bounded selector.
- `CitationEvidence`: citation handle, claim/evidence handle, quote handle,
  source span, confidence, verification status, freshness, and redaction class.
- `BibliographyStyle`: style handle, style family, locale, output format,
  supported item types, and capability hash.
- `FormattedCitation`: citation handle, formatted output handle, style handle,
  locale, output format, ordering metadata, and warnings.
- `CitationVerificationResult`: identifier status, source anchor status, quote
  match status, metadata freshness, license status, confidence, and diagnostics.
- `CitationProviderCapability`: identifier schemes, metadata enrichment,
  verification depth, style rendering, import/export formats, max items, rate
  limits, and health.

## Permission, Policy, Resource, Entitlement, And Approval Model

Permission scopes:

- `citation.create`
- `citation.read`
- `citation.update`
- `citation.source.link`
- `citation.resolve`
- `citation.verify`
- `citation.format`
- `citation.import_export`
- `citation.evidence.read`

Policy defaults:

- Every command is scoped to application id, tenant id, session id, task id, and
  trace id when available.
- Source-span linking requires source access permission and selector validation.
- Identifier resolution may use network/provider adapters and requires rate
  limits, quota, and redacted provider payload handling.
- Formatting output must obey quote/snippet redaction and output-size limits.
- Verification may disclose source availability or metadata freshness, so it
  requires separate verification permission.
- Raw source documents, raw private quotes, raw provider payloads, credentials,
  and unbounded bibliography output are forbidden in observability.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, family, version, lifecycle, command schemas,
identifier schemes, style capabilities, import/export formats, permission
scopes, policy templates, source-anchor selector support, verification depth,
examples, unavailable diagnostics, health, compatibility, redaction profiles,
and documentation links.

The developer guide at `docs/developer-packs/knowledge/citations.md` must cover
manifest declarations, permissions, citation metadata, identifiers, source
anchors, W3C-style selectors, CSL-compatible data, bibliography styles,
verification states, quote policies, import/export, provider replacement,
unavailable diagnostics, trace/audit interpretation, and conformance tests.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `citations_pack_declared`
- `citations_pack_admission_validated`
- `citation_created`
- `citation_identifier_resolved`
- `citation_source_span_linked`
- `citation_verified`
- `citation_formatted`
- `citation_bibliography_formatted`
- `citation_imported`
- `citation_exported`
- `citation_anchor_inspected`
- `citations_pack_policy_decision`
- `citations_pack_service_call_requested`
- `citations_pack_service_call_succeeded`
- `citations_pack_service_call_failed`
- `citations_pack_unavailable`
- `citations_pack_snapshot_recorded`

Snapshots include descriptor version, provider capability hashes, identifier
scheme support, style capability hashes, import/export support, verification
status aggregates, source-anchor counts, provider health, command availability,
policy template hash, resource counters, and sanitized replay pointers.
Snapshots must exclude raw source documents, raw private quotes, raw provider
payloads, credentials, raw style files, and unbounded formatted output.

## Design Patterns

- **Facade**: SDK clients expose discovery and command builders only.
- **Command**: every operation is a typed command/result DTO.
- **Strategy**: identifier resolvers, style renderers, import/export adapters,
  selector validators, and unavailable behavior are replaceable.
- **Decorator**: trace, policy, entitlement, resource, approval, metering,
  source-permission checks, and redaction wrap service calls.
- **Specification**: admission validates citation metadata, identifiers,
  selectors, style support, permissions, provider capability, and compatibility.
- **Observer**: citation changes, verification events, source-anchor events,
  health, trace, and audit events are subscribable.
- **Memento**: citation version hashes, source anchors, verification snapshots,
  formatted-output handles, and replay pointers preserve recovery state.
- **Abstract Factory**: provider adapters are created only by approved runtime
  host composition roots.

## Risks And Mitigations

- Risk: source anchors leak private text. Mitigation: anchors use handles,
  selectors, hashes, and redacted quote handles.
- Risk: citation formatting becomes provider-specific. Mitigation: use
  provider-neutral style descriptors and bounded renderer adapters.
- Risk: verification falsely implies source truth. Mitigation: status distinguishes
  identifier reachability, anchor match, quote match, metadata freshness, and
  license state.
- Risk: provider metadata drifts over time. Mitigation: model freshness,
  version hash, stale diagnostics, and replayable verification results.
