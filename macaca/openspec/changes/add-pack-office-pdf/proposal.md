# Change: Add Office PDF Pack

## Why

Developers need `pack.office.pdf.v1` as an industrial PDF capability for
document inspection, page rendering, text extraction, structure extraction,
image/table extraction, forms, annotations, redaction, merge, split, optimize,
OCR handoff, accessibility/tagging metadata, signature reference workflows,
export/conversion requests, artifact handles, and replay diagnostics. It must
not be a thin wrapper around Adobe Acrobat Services, Mozilla PDF.js, PDFium,
iText, Poppler, PDFBox, a cloud conversion vendor, or one PDF library.

PDFs frequently carry contracts, invoices, identity documents, financial
reports, medical or legal records, signatures, embedded files, form fields,
comments, and scan images. Reading or mutating them can leak regulated data,
invalidate signatures, change legal evidence, or publish private artifacts.
Macaca must therefore expose PDF operations only through provider-neutral typed
service commands with permission, policy, entitlement, resource, approval,
version preconditions, redaction, artifact retention, trace, audit, health,
snapshot, replay, and structured unavailable behavior.

## Research And Supplier/API Baseline

Official and supplier references considered for this pack:

- Adobe Acrobat Services PDF Services API exposes cloud operations for PDF
  creation, export, OCR, compression, protection, split, merge, accessibility
  auto-tagging, and structured extraction. References:
  https://www.adobe.io/document-services/apis/pdf-services/ and
  https://developer.adobe.com/document-services/docs/overview/pdf-extract-api/
- Mozilla PDF.js exposes browser/Node rendering, page loading, canvas rendering,
  text content extraction, annotation layers, viewport handling, and progressive
  loading patterns. Reference: https://mozilla.github.io/pdf.js/examples/
- PDFium exposes native rendering, parsing/rasterization, text extraction, form
  handling, annotation support, signature verification, and document
  modification/creation through embeddable provider wrappers. References:
  https://pdfium.googlesource.com/pdfium/+/master/README.md and
  https://www.embedpdf.com/docs/pdfium/introduction
- iText exposes industrial PDF creation/manipulation, page merge/split patterns,
  PDF/A support, digital signature/PAdES workflows, annotation flattening, and
  compliance-oriented tooling. References:
  https://kb.itextpdf.com/itext/how-to-merge-documents-correctly and
  https://itextpdf.com/blog/itext-news-technical-notes/itext-suite-803-advanced-pades-api-two-step-signing
- Microsoft Adobe PDF Services connector documentation provides an enterprise
  integration baseline for convert/export/OCR/compress/linearize/protect/edit
  and structured extraction operations. Reference:
  https://learn.microsoft.com/en-us/connectors/adobepdftools/

Macaca maps these supplier concepts into provider-neutral PDF scope, document
handle, page handle, render plan, extraction plan, text span, structure element,
table, image, annotation, form field, embedded file, signature reference,
redaction plan, edit plan, merge/split plan, export plan, artifact handle,
provider capability, version/freshness metadata, and diagnostics DTOs. Concrete
Adobe, PDF.js, PDFium, iText, Poppler, PDFBox, OCR, rendering, signing, storage,
and conversion providers stay behind replaceable providers.

## What Changes

- Add provider-neutral `pack.office.pdf.v1` under the `office` family.
- Define command namespace `pdf.*` for:
  - provider and document capability inspection
  - PDF open/import and metadata inspection
  - page listing and page rendering
  - text, structure, table, image, form, annotation, embedded-file, and
    signature-reference inspection
  - extraction planning and extraction requests
  - annotation/redaction/edit planning and edit requests
  - merge, split, optimize, protect, linearize, and export/conversion planning
  - artifact handle resolution and event/snapshot inspection
- Define DTOs for PDF scope, provider capability, document handle, page handle,
  document metadata, page geometry, render plan, extraction plan, text span,
  structure element, table, image, form field, annotation, embedded file,
  signature reference, redaction operation, edit operation, edit plan,
  merge/split plan, export plan, artifact handle, event cursor, and diagnostics.
- Define permission scopes, policy defaults, document/page/artifact scopes,
  encryption/password-reference behavior, signature-preservation behavior,
  approval rules, resource/entitlement behavior, SDK discovery, developer
  documentation, trace/audit events, snapshots, replay, and boundary gates.
- Require detailed developer documentation at
  `docs/developer-packs/office/pdf.md` before implementation completion.

## Impact

- Affected specs: `pack-office-pdf`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral protocol DTOs, pack descriptors,
  admission validators, SDK discovery and command builders, PDF service
  provider or unavailable provider, runtime-host provider adapters,
  render/extract/artifact/redaction support, trace/audit schemas, replay tests,
  dependency-boundary gates, and developer documentation.
- Non-goals: no concrete Adobe/PDF.js/PDFium/iText/Poppler/PDFBox/OCR/signing/
  conversion/storage provider implementation in this proposal; no legal,
  finance, invoice, identity, medical, HR, form-workflow, or document-template
  business logic; no provider-name, document-name, form-name, signature-name, or
  workflow-name routing in OS layers; no raw credentials, passwords, private
  keys, certificates, signatures, provider payloads, full document text, raw
  PDF bytes, raw image bytes, raw exports, prompts, manifests, package bytes, or
  unbounded page trees in observability; no SDK/shell/kernel provider
  construction; no fake success when provider, format support, permission,
  entitlement, approval, resource, password reference, signature policy, or host
  support is absent.
