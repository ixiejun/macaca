# Knowledge Citations Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
existing platform inventory, and GitNexus memo evidence for
`pack.knowledge.citations.v1`. Citations must attach provenance to sources and
claims through typed service commands without leaking raw source documents,
private quotes, provider metadata payloads, or bibliography style files.

## Source Baseline

- Citation Style Language schemas, styles, and locales:
  <https://citationstyles.org/>
- Crossref REST API:
  <https://www.crossref.org/documentation/retrieve-metadata/rest-api/>
- DataCite REST API:
  <https://support.datacite.org/reference/introduction>
- W3C Web Annotation Data Model:
  <https://www.w3.org/TR/annotation-model/>
- Zotero Web API:
  <https://www.zotero.org/support/dev/web_api/v3/basics>
  and Zotero Style Repository: <https://www.zotero.org/styles>

## Supplier API Notes

- CSL contributes citation item schema, styles, locales, bibliography rendering,
  and output formats. Macaca should expose style handles and formatted output
  references, not raw style files.
- Crossref contributes DOI metadata, references, relations, funder, license,
  ORCID/ROR, and update metadata. Macaca should model identifier resolution,
  related references, license/freshness, and provider capability.
- DataCite contributes DOI metadata for datasets/software/texts, related
  identifiers, versions, and rights. Macaca should support research/data/software
  citation metadata without hardcoding a publication-domain workflow.
- W3C Web Annotation contributes targets, bodies, motivations, selectors, and
  source states. Macaca should model source anchors and quote/position/fragment
  selectors with redaction.
- Zotero contributes library items, collections, attachments, tags, relations,
  and import/export. Macaca should expose library/collection handles and
  import/export diagnostics, not Zotero-native item JSON as SDK contract.

## Macaca-Owned Abstractions

`pack.knowledge.citations.v1` should define `CitationItem`,
`CitationIdentifier`, `CitationContributor`, `CitationSourceAnchor`,
`CitationSelector`, `CitationEvidence`, `BibliographyStyle`,
`FormattedCitation`, `CitationVerificationResult`, `CitationImportResult`,
`CitationExportResult`, and `CitationProviderCapability`.

The DTOs must carry identifier schemes, normalized metadata, contributors,
source selectors, quote policy, evidence links, style capability, formatted
output handles, verification freshness, import/export results, provider health,
and replay pointers. Raw source documents, private quotes, provider JSON,
credentials, style files, and unbounded formatted output are rejected.

## Existing Macaca Platform Inventory

- Document parsing, retrieval, graph, and summarization packs will provide
  source/evidence handles through declared capabilities; citations must not
  parse or retrieve documents directly.
- Generic service descriptors, SDK facade, trace-required service calls,
  unavailable providers, persistence snapshots, and permission command objects
  provide the reusable substrate for citation providers.
- No current evidence proves citation-specific DTOs, provider traits, source
  anchor validators, style rendering, import/export, SDK helpers, or docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
