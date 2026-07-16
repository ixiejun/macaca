# Change: Add Industrial Knowledge Citations Pack

## Why

Applications need citations as a reusable capability for preserving where claims
come from, linking generated or extracted statements to source spans, rendering
bibliographies, validating identifiers, and auditing evidence chains. Citations
are not retrieval, parsing, or summarization; they are the provenance and
reference layer that makes downstream answers verifiable.

Industrial citation support must model source anchors, text/page/DOM selectors,
DOI/URL/ISBN/arXiv-like identifiers, CSL-compatible metadata, bibliography
styles, quote/snippet policies, verification status, evidence bundles, stale
source detection, and replayable audit records.

## Supplier And Platform API Research

This proposal maps mature citation/reference standards and APIs into Macaca
provider-neutral abstractions:

- Citation Style Language (CSL) defines style-independent bibliographic data and
  rendering styles for citations and bibliographies. Macaca maps this to
  `CitationItem`, `BibliographyStyle`, locale/style compatibility, and formatted
  output commands.
- Crossref REST API exposes DOI works, metadata, references, funders, members,
  relations, licenses, and update timestamps. Macaca maps this to identifier
  resolution, metadata enrichment, reference graph metadata, license metadata,
  and stale metadata checks.
- DataCite REST API exposes DOI metadata for datasets, software, texts, related
  identifiers, creators, publishers, rights, and versioning. Macaca maps this to
  research/data/software citation metadata and related-resource verification.
- W3C Web Annotation Data Model defines annotation targets, bodies, selectors,
  text position selectors, text quote selectors, fragment selectors, states, and
  motivations. Macaca maps this to source span anchors, page/DOM/text selectors,
  quote anchoring, evidence target state, and replayable source links.
- Zotero-style library APIs and reference managers model items, creators,
  collections, attachments, tags, relations, and citation export. Macaca maps
  this to citation libraries, item collections, attachment/source handles, tags,
  and import/export compatibility.

The Macaca contract avoids hardcoding any one citation provider or academic
domain. Identifier-specific behavior is expressed through provider capability
metadata and typed resolution results.

## What Changes

- Add provider-neutral `pack.knowledge.citations.v1` under the `knowledge`
  family.
- Define DTOs for citation items, sources, identifiers, contributors, anchors,
  selectors, source spans, quotes, bibliography styles, formatted citations,
  evidence bundles, verification results, license metadata, and provider
  capabilities.
- Define commands for create citation, resolve identifier, link source span,
  verify citation, format citation, format bibliography, list citations, import
  citations, export citations, inspect source anchors, and inspect provider
  capability.
- Define permission scopes for citation create/read/update, source span link,
  identifier resolution, bibliography formatting, verification, import/export,
  and evidence read.
- Require redacted quote/snippet handling, stable source anchors, replayable
  verification, stale metadata diagnostics, provider replacement, and a detailed
  developer guide.

## Impact

- Affected specs: `pack-knowledge-citations`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected future code: provider-neutral proto DTOs, citation descriptors,
  admission validators, SDK discovery metadata, focused SDK clients, citation
  service providers, unavailable/mock providers, style/render adapters,
  identifier-resolution adapters, trace/audit schemas, replay tests, and
  dependency-boundary gates.
- Non-goals: no application-specific citation workflow, no summarization, no
  provider-name routing in OS layers, no raw source document exposure, no
  concrete provider construction in kernel/SDK/shells, and no fake success when
  citation providers or style engines are unavailable.

## References

- Citation Style Language: https://citationstyles.org/
- CSL JSON schema:
  https://github.com/citation-style-language/schema/blob/master/csl-data.json
- Crossref REST API: https://api.crossref.org/swagger-ui/index.html
- DataCite REST API: https://support.datacite.org/docs/api
- W3C Web Annotation Data Model:
  https://www.w3.org/TR/annotation-model/
- Zotero Web API: https://www.zotero.org/support/dev/web_api/v3/start
