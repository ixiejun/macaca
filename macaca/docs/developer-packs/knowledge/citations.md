# Knowledge Citations Pack

`pack.knowledge.citations.v1` describes reference metadata, identifier
resolution, source-span linking, citation verification, formatting,
bibliography generation, import/export, and provider inspection.

## Manifest Declaration

```toml
[service_contract]
optional_packs = ["pack.knowledge.citations.v1"]
```

Use optional declarations when citations improve output quality but are not
required for readiness.

## Permissions

Scopes are `citation.create`, `citation.read`, `citation.update`,
`citation.source.link`, `citation.resolve`, `citation.verify`,
`citation.format`, `citation.import_export`, and `citation.evidence.read`.

## Capability Model

DTOs include citation item, identifier, contributor, source anchor, selector,
evidence, bibliography style, formatted citation, verification result,
import/export results, and provider capability. Source quotes, raw style files,
raw provider metadata, raw source documents, and private corpus content stay
behind handles and redaction policies.

## Platform Comparison

CSL data, styles, and locales map to bibliography style and formatted citation
DTOs. Crossref DOI metadata, references, relations, and licenses map to
identifier, citation item, contributor, and verification metadata. DataCite DOI
metadata, related identifiers, versions, and rights map to identifier and
citation metadata. W3C Web Annotation targets, selectors, and states map to
source anchor and selector DTOs. Zotero-style library items, collections, and
import/export map to citation item and import/export result DTOs.

## Commands

Commands cover citation creation, identifier resolution, source-span linking,
verification, citation formatting, bibliography formatting, listing, updating,
import/export, source-anchor inspection, and provider inspection.

## App-Facing Examples

- Create a citation with normalized identifiers and contributor references.
- Resolve a DOI-like identifier through `citations.resolve_identifier`.
- Link a source span with a bounded text-position selector and a source handle.
- Verify citation freshness and anchor validity before publishing.
- Format inline citations and bibliographies with descriptor-supported styles.
- Import or export citation collections through bounded handles.
- Inspect a source anchor when verification fails.
- Inspect provider capability for identifier schemes, style rendering,
  selector support, import/export formats, and verification depth.
- Handle unavailable diagnostics without falling back to raw resolver APIs.

## Trace And Audit

Trace metadata should include citation id, identifier scheme, selector kind,
style id, command name, provider class, capability hash, and result status. Raw
provider payloads, raw source documents, private quotes, raw style files, and
private corpus content must remain outside trace and audit records.

## Provider Authors

Descriptors must report identifier schemes, metadata enrichment, CSL-compatible
style support, selector support, import/export formats, verification depth,
freshness, redaction, quota, snapshots, and conformance tests. Unavailable
providers must return trace-safe diagnostics without faking resolver success.

Conformance tests should cover identifier normalization, selector validation,
CSL-style compatibility, source-anchor redaction, import/export bounds,
verification statuses, unavailable behavior, and provider capability reporting.
