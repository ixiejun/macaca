## 1. Supplier API Research And Scope

- [x] 1.1 Re-read architecture governance, microkernel boundaries,
  serviceization allowlist, design-pattern guidance, and the industrial catalog
  umbrella proposal before implementation.
- [x] 1.2 Record API/standard notes for CSL data/styles/locales, Crossref DOI
  metadata/references/relations/licenses, DataCite DOI metadata/related
  identifiers/versions/rights, W3C Web Annotation targets/selectors/states, and
  Zotero-style reference library items/collections/import/export.
- [x] 1.3 Map supplier concepts to provider-neutral citation item, identifier,
  contributor, source anchor, selector, evidence, bibliography style, formatted
  citation, verification result, import/export, and provider capability DTOs.
- [x] 1.4 Inventory existing service descriptors, SDK clients, admission paths,
  trace/audit schemas, optional providers, mock providers, unavailable providers,
  document/retrieval/evidence handles, and policy/resource gates that can back
  citations.
- [x] 1.5 Record GitNexus CRITICAL/HIGH findings as memo only before
  implementation commits.

## 2. Contract, Descriptor, And DTOs

- [x] 2.1 Define provider-neutral DTOs for `CitationItem`,
  `CitationIdentifier`, `CitationContributor`, `CitationSourceAnchor`,
  `CitationSelector`, `CitationEvidence`, `BibliographyStyle`,
  `FormattedCitation`, `CitationVerificationResult`, `CitationImportResult`,
  `CitationExportResult`, and `CitationProviderCapability`.
- [x] 2.2 Define typed command DTOs for `citations.create_citation`,
  `citations.resolve_identifier`, `citations.link_source_span`,
  `citations.verify_citation`, `citations.format_citation`,
  `citations.format_bibliography`, `citations.list_citations`,
  `citations.update_citation`, `citations.import_citations`,
  `citations.export_citations`, `citations.inspect_source_anchor`, and
  `citations.inspect_provider`.
- [x] 2.3 Define typed success, page, formatted-output, verification,
  import/export, denied, unavailable, unsupported, conflict, quota, timeout,
  validation, and provider-failure result DTOs.
- [x] 2.4 Define descriptor metadata for pack id, identifier schemes, style
  support, selector support, import/export formats, command schemas,
  permissions, policy templates, verification depth, redaction profile, SDK
  metadata, compatibility, diagnostics, and documentation links.
- [x] 2.5 Add descriptor hash, identifier normalization, selector validation,
  CSL metadata compatibility, style rendering, redaction-profile, and provider
  capability tests.

## 3. Admission, Permission, Policy, Resource, And Approval

- [x] 3.1 Implement declaration validation for scopes: `citation.create`,
  `citation.read`, `citation.update`, `citation.source.link`,
  `citation.resolve`, `citation.verify`, `citation.format`,
  `citation.import_export`, and `citation.evidence.read`.
- [x] 3.2 Enforce source access, source-anchor validation, identifier scheme
  support, network/provider resolver policy, style support, import/export limits,
  quote/snippet redaction, output bounds, rate limit, timeout, approval, and
  resource budget checks before provider calls.
- [x] 3.3 Reject raw credentials, raw provider payloads, raw source documents,
  raw private quotes, raw bibliography style files, unbounded formatted output,
  and private corpus content at admission and observability boundaries.
- [x] 3.4 Model required declarations as readiness blockers and optional
  declarations as explicit degraded effective capabilities.
- [x] 3.5 Add tests proving denied, validation, quota, unsupported, and
  unavailable paths do not call concrete citation providers.

## 4. Service Provider And Runtime Integration

- [x] 4.1 Implement or bind citation providers only through the service runtime
  and approved runtime-host composition roots.
- [x] 4.2 Add unavailable and mock providers with deterministic identifier
  resolution, source anchor validation, verification, formatting, import/export,
  and capability behavior.
- [x] 4.3 Add lifecycle, health, snapshot, shutdown, timeout, cancellation,
  bounded pagination, optimistic concurrency, source-anchor inspection,
  metadata freshness checks, and formatted-output handles.
- [x] 4.4 Add provider capability reporting for identifier schemes, metadata
  enrichment, verification depth, style rendering, import/export formats,
  selector support, max items, rate limits, and health.
- [x] 4.5 Add canonical execution-path tests proving every citation command
  traverses SDK/facade, service runtime decorators, and provider dispatch exactly
  once.

## 5. SDK, WASM ABI, Application Framework, And Examples

- [x] 5.1 Extend SDK discovery for `pack.knowledge.citations.v1` with command
  schemas, provider capability reports, examples, availability, diagnostics,
  docs metadata, policy templates, identifier schemes, style support,
  import/export support, verification depth, and compatibility.
- [x] 5.2 Add focused SDK helper builders that only produce canonical traced
  service calls and return Null Object unavailable diagnostics when the pack is
  absent.
- [x] 5.3 Extend WASM/application ABI metadata so applications can declare
  citation access, link source spans, resolve identifiers, verify citations, and
  format bibliographies only through declared permissions.
- [x] 5.4 Add generic examples for create citation, resolve DOI-like identifier,
  link text selector, verify citation, format inline citation, format
  bibliography, import/export citations, inspect source anchor, inspect provider,
  and unavailable provider handling.

## 6. Trace, Audit, Replay, Security, And Gates

- [x] 6.1 Emit sanitized declaration, admission, citation creation, identifier
  resolution, source span linking, verification, formatting, bibliography
  formatting, import/export, anchor inspection, policy, resource, entitlement,
  approval, service-call, provider-call, health, snapshot, and unavailable
  events.
- [x] 6.2 Add replay tests proving citation creation, identifier resolution,
  source anchors, verification, formatting, import/export, and anchor inspection
  are trace-addressable through the canonical service path.
- [x] 6.3 Add sanitization tests proving traces, audits, snapshots, SDK
  diagnostics, and examples do not leak raw credentials, raw provider payloads,
  raw source documents, raw private quotes, raw style files, unbounded formatted
  output, or private corpus content.
- [x] 6.4 Add dependency gates proving kernel, SDK, shells, and generic
  application framework do not import concrete citation providers, style engines,
  or identifier resolver adapters.
- [x] 6.5 Run `openspec validate add-pack-knowledge-citations --strict`,
  targeted cargo tests, boundary gates, file-size gates, canonical execution-path
  tests, and audit replay checks before marking complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/knowledge/citations.md` with pack purpose,
  platform comparison, manifest declaration, permission scopes, citation
  metadata, identifiers, contributors, CSL-compatible data, source anchors,
  W3C-style selectors, quote policies, bibliography styles, verification
  statuses, import/export, provider replacement, unavailable diagnostics,
  trace/audit interpretation, and operational limits.
- [x] 7.2 Include generic app-facing examples for create citation, resolve
  identifier, link source span, verify citation, format citation, format
  bibliography, import/export, inspect source anchor, and handle unavailable
  provider results.
- [x] 7.3 Include provider-author guidance for descriptor metadata, identifier
  resolution, source selector validation, style rendering, freshness checks,
  redaction, snapshots, quota reporting, and conformance tests.
- [x] 7.4 Cross-link the guide from SDK discovery metadata and the industrial
  pack catalog index before marking `add-pack-knowledge-citations` complete.
