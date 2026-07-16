# Office PDF Pack Design

## Context

`pack.office.pdf.v1` exposes PDF operations as a Macaca OS serviceized
capability. It lets applications inspect, render, extract, annotate, redact,
merge, split, optimize, protect, export, and diagnose PDF documents without
embedding Adobe Acrobat Services, PDF.js, PDFium, iText, Poppler, PDFBox, OCR,
signing, storage, conversion, or application-specific document workflows into
generic OS layers.

PDFs combine static page description, images, text extraction ambiguity,
annotations, forms, attachments, encryption, digital signatures, accessibility
tags, and compliance profiles. The pack therefore models reads as bounded
projections and side effects as validated plans plus requests. Raw PDF bytes,
passwords, signature material, certificates, embedded files, extracted content,
and rendered pages remain behind artifact/redaction boundaries.

## Supplier Capability Matrix

| Supplier/platform | Industrial capability | Macaca abstraction |
| --- | --- | --- |
| Adobe Acrobat Services PDF Services / Extract APIs | Create/export/OCR/compress/protect/linearize/split/merge, structured JSON or Markdown extraction, accessibility auto-tagging | PDF provider capability, extraction plan, export plan, OCR handoff, artifact handle, structure/table/image DTOs |
| Mozilla PDF.js | Browser/Node page loading, viewport calculation, canvas rendering, text content, annotation layers, progressive loading | Page handle, render plan, text span extraction, annotation projection, bounded client-compatible rendering |
| PDFium | Native parse/rasterize/render, text extraction/search, forms, annotations, signature verification, modification/creation | Native provider Strategy, render page, inspect forms/annotations/signatures, edit plan, artifact handle |
| iText | Creation/manipulation, page copy/merge/split, PDF/A, PAdES signing, annotation flattening, compliance workflows | Edit/merge/split/export plans, PDF/A profile metadata, signature reference workflow, compliance diagnostics |
| Microsoft Adobe PDF Services connector | Enterprise convert/export/OCR/compress/linearize/protect/edit and structured extraction integration | Remote provider adapter, operation class mapping, enterprise quota/credential/resource diagnostics |

The pack exposes provider-neutral contracts. Provider adapters translate to
cloud APIs, browser renderers, native PDF engines, Java/.NET libraries, local
CLI tools, remote conversion services, or unavailable providers. OS layers must
not branch on provider names, file names, document titles, form names,
signature labels, compliance labels, or business workflows.

## Goals

- Provide stable pack id `pack.office.pdf.v1` and command namespace `pdf.*`.
- Support provider inspection, document import/open, metadata inspection, page
  listing, rendering, text extraction, structure extraction, table extraction,
  image extraction, forms, annotations, embedded files, signature references,
  extraction planning/requests, edit planning/requests, merge/split planning,
  export/conversion planning/requests, artifact handles, snapshots, health, and
  replay diagnostics.
- Preserve safety with document/page/artifact scopes, password references,
  encrypted-document behavior, signature preservation policy, redaction,
  approval, quotas, bounded extraction, and sanitized audit.
- Keep concrete PDF providers behind replaceable service providers.
- Require developer documentation at `docs/developer-packs/office/pdf.md`.

## Non-Goals

- Do not implement concrete Adobe, PDF.js, PDFium, iText, Poppler, PDFBox, OCR,
  signing, certificate, conversion, storage, or rendering providers in this
  proposal.
- Do not define legal, invoice, finance, tax, medical, identity, HR,
  e-signature, review, or form-specific business workflows.
- Do not store or expose raw PDF bytes, passwords, decrypted documents, private
  keys, certificates, signatures, embedded files, rendered images, extracted
  full text, raw provider payloads, prompts, manifests, package bytes, or
  unbounded page trees in observability.
- Do not silently alter signed documents, flatten annotations, redact content,
  export documents, transmit documents, or remove encryption without typed plan,
  policy checks, version preconditions, and approval where required.

## Ownership And Boundaries

- Pack id: `pack.office.pdf.v1`.
- Family: `office`.
- Backing service owner: PDF service provider.
- SDK surface: `sdk.packs.office.pdf`.
- Command namespace: `pdf.*`.
- Microkernel owns identity, policy facade, resource primitives, service-call
  evidence, trace/audit primitives, and registry metadata only.
- Application framework owns manifest declarations, app-scoped permissions, and
  effective capability projection.
- Runtime host owns provider adapter registration, credential/password-reference
  bridges, artifact stores, render/extract/conversion bridges, decorators, and
  sanitized diagnostics through approved composition roots.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `pdf.inspect_provider` | Inspect provider, format, profile, and operation support | Returns sanitized render, extract, edit, form, signature, export, quota, and health metadata |
| `pdf.import_document_request` | Import PDF from file/artifact handle | Requires artifact permission, format validation, malware/size policy, and audit |
| `pdf.open_document` | Resolve PDF document handle and version metadata | Requires document scope, password reference if encrypted, and bounded metadata |
| `pdf.inspect_metadata` | Inspect document metadata, permissions, encryption, signatures, tags, attachments, and page count | Requires metadata permission and redaction |
| `pdf.list_pages` | List page handles and bounded page geometry | Requires page permission and projection limits |
| `pdf.render_page` | Render a page or page region to an artifact | Requires render plan, resource budget, redaction, and artifact retention |
| `pdf.extract_text` | Extract bounded text spans | Requires extraction scope, ordering mode, redaction, paging, and OCR handoff policy |
| `pdf.extract_structure` | Extract logical structure, headings, tags, reading order, and accessibility metadata | Requires bounded tree depth and redaction |
| `pdf.extract_tables` | Extract table handles and bounded cell projections | Requires table extraction support, confidence metadata, and redaction |
| `pdf.extract_images` | Extract image handles and metadata | Requires image permission, size limits, and artifact retention |
| `pdf.inspect_forms` | Inspect AcroForm/XFA-like field metadata where supported | Requires form permission and value redaction |
| `pdf.inspect_annotations` | Inspect comments, markups, links, widgets, and annotation metadata | Requires annotation permission, paging, and redaction |
| `pdf.inspect_embedded_files` | Inspect embedded file metadata | Requires attachment permission, retention, and redaction |
| `pdf.inspect_signatures` | Inspect signature references and validation metadata | Requires signature permission and no private-key access |
| `pdf.plan_edit` | Plan annotations, redactions, metadata edits, form fills, flattening, page edits, protection, and optimization | Validates operations, versions, signatures, permissions, resources, and approvals |
| `pdf.edit_request` | Execute a validated edit plan | Requires plan handle, idempotency key, version preconditions, approval, and audit |
| `pdf.plan_merge_split` | Plan document merge or split operations | Validates page ranges, source compatibility, signatures, resources, and output policy |
| `pdf.merge_split_request` | Execute a validated merge/split plan | Returns bounded artifact handles and audit metadata |
| `pdf.plan_export` | Plan conversion/export to PDF profile or external format | Validates format, sensitivity, retention, OCR/conversion policy, and approvals |
| `pdf.export_request` | Execute a validated export plan | Returns artifact handle and bounded diagnostics |
| `pdf.get_artifact_handle` | Resolve render/extract/export artifact metadata | Requires artifact permission, retention, and redaction |

Every command must define typed command DTOs, typed success results, typed
paged/partial/asynchronous results, typed denied/unavailable/unsupported/
conflict/stale-version/schema-mismatch/format-unsupported/encrypted-document/
password-required/signature-invalid/signature-policy-denied/redaction-denied/
export-denied/write-denied/attachment-denied/quota/timeout/cancellation/
approval-required/failure results, redaction profile, idempotency semantics for
side effects, and replay metadata.

## DTO Model

Core DTOs:

- `PdfScope`: provider scope handle, document handle, source artifact handle,
  credential/password reference, network policy, artifact policy, permission
  state, rate-limit profile, and health.
- `PdfProviderCapability`: provider class, import/open support, render support,
  extraction support, structure/table/image support, form support, annotation
  support, embedded-file support, signature support, encryption support,
  redaction/edit support, merge/split support, export/conversion support, OCR
  handoff support, auth modes, rate limits, lifecycle, and health.
- `PdfDocumentHandle`: document handle, provider scope, source artifact handle,
  format/profile, version hash, permission state, encryption state, signature
  summary, sensitivity class, redaction class, and freshness.
- `PdfMetadata`: title/author/producer handles, page count class, PDF version,
  profile hints, permissions summary, tag/accessibility summary, attachment
  summary, encryption summary, and redaction class.
- `PdfPageHandle`: document handle, page index, page label handle, geometry
  class, rotation, version hash, thumbnail/render artifact handle, and redaction
  class.
- `PdfRenderPlan`: page or region scope, render profile, resolution class,
  color mode, redaction profile, retention, resource estimate, and validation
  diagnostics.
- `PdfExtractionPlan`: page range, extraction mode, reading-order strategy,
  OCR handoff policy, table/image extraction flags, redaction profile, paging,
  resource estimate, and validation diagnostics.
- `PdfTextSpan`: span handle, page handle, text handle, bounds class, style
  references, reading-order class, confidence class, and sensitivity class.
- `PdfStructureElement`: element handle, parent handle, role, page anchors,
  child count class, reading-order class, tag metadata, and redaction class.
- `PdfTable`: table handle, page handles, row/column count class, cell handles,
  confidence class, and redaction class.
- `PdfImage`: image handle, page handle, content type, size class, color space,
  checksum handle, retention, and redaction class.
- `PdfFormField`: field handle, field kind, page handle, value handle,
  required/read-only state, validation metadata, and redaction class.
- `PdfAnnotation`: annotation handle, annotation kind, page handle, author
  handle, timestamp, target bounds class, payload handle, and redaction class.
- `PdfEmbeddedFile`: embedded file handle, file-name handle, content type, size
  class, checksum handle, retention, and redaction class.
- `PdfSignatureReference`: signature handle, document/page scope, signer handle,
  certificate reference handle, validation class, timestamp, coverage summary,
  and policy state.
- `PdfRedactionOperation`: operation handle, target text/page/region/annotation
  handle, reason code, preview artifact handle, and validation metadata.
- `PdfEditOperation`: operation handle, operation kind, target document/page/
  form/annotation/metadata/protection handle, payload handle, and validation
  metadata.
- `PdfEditPlan`, `PdfMergeSplitPlan`, and `PdfExportPlan`: plan handle, input
  handles, operation list hash, version preconditions, signature preservation
  policy, required approvals, idempotency key, retention, redaction, and
  validation diagnostics.
- `PdfArtifactHandle`: artifact handle, source document/page/extraction handle,
  artifact kind, content type, size class, checksum handle, retention, redaction
  class, and replay pointer.

Provider-specific extensions may appear only as bounded `adapter_metadata`
behind capability hashes and must not drive OS-layer routing.

## Permission, Policy, Resource, Entitlement, And Approval Model

Permission scopes:

- `pdf.provider.inspect`
- `pdf.document.import`
- `pdf.document.open`
- `pdf.metadata.read`
- `pdf.page.read`
- `pdf.render`
- `pdf.text.extract`
- `pdf.structure.extract`
- `pdf.table.extract`
- `pdf.image.extract`
- `pdf.form.read`
- `pdf.form.write`
- `pdf.annotation.read`
- `pdf.annotation.write`
- `pdf.embedded_file.read`
- `pdf.signature.read`
- `pdf.document.write`
- `pdf.redaction.write`
- `pdf.merge_split`
- `pdf.export`
- `pdf.artifact.read`

Policy defaults:

- Every command is scoped to application id, tenant id, session id, task id,
  trace id, provider scope, document handle, page/artifact handle when
  applicable, actor handle when available, and credential/password reference
  when required.
- Encrypted documents require password references, never raw passwords.
- Signature-sensitive operations require explicit signature preservation or
  invalidation policy. Macaca must not claim a signature remains valid after an
  operation unless the provider returned typed validation evidence.
- Redaction, annotation flattening, form filling, protection changes,
  attachment extraction, merge/split, and export require plan/request separation,
  idempotency key, version preconditions, approval where policy requires, and
  sanitized audit reason.
- Raw document text, rendered pages, attachments, extracted images, tables,
  forms, comments, signatures, and exported artifacts require redaction and
  bounded output. Raw PDF bytes must not enter observability.
- Remote operations require network permission, provider quota, rate limits,
  timeout, cancellation, and structured unavailable behavior.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, family, version, lifecycle, command schemas,
format/profile support, render support, extraction support, form support,
annotation support, embedded-file support, signature support, encryption
support, redaction/edit support, merge/split support, export/conversion support,
OCR handoff support, permission scopes, policy templates, resource limits,
approval rules, provider capability hashes, health, compatibility,
diagnostics, examples, redaction profiles, and documentation links.

The developer guide at `docs/developer-packs/office/pdf.md` must cover:

- manifest declaration and optional/required behavior
- provider scopes, document handles, encrypted-document behavior, password
  references, page handles, metadata, render plans, extraction plans, text spans,
  structure trees, tables, images, forms, annotations, embedded files, signature
  references, redaction operations, edit plans, merge/split plans, export plans,
  artifacts, provider capabilities, and unavailable states
- plan/request lifecycle, version conflicts, signature preservation policy,
  encrypted/password-required diagnostics, OCR/conversion handoff, redaction,
  approvals, quotas, provider replacement, trace/audit interpretation, and
  conformance tests

Examples must use synthetic PDFs, pages, fields, signatures, attachments, and
artifacts. They must not include provider names, real credentials, raw
passwords, private keys, certificates, customer data, raw PDF bytes, raw
rendered pages, raw exports, or workflow-specific conventions.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `pdf_pack_declared`
- `pdf_pack_admission_validated`
- `pdf_provider_inspected`
- `pdf_document_imported`
- `pdf_document_opened`
- `pdf_metadata_inspected`
- `pdf_pages_listed`
- `pdf_page_render_planned`
- `pdf_page_render_requested`
- `pdf_text_extracted`
- `pdf_structure_extracted`
- `pdf_tables_extracted`
- `pdf_images_extracted`
- `pdf_forms_inspected`
- `pdf_annotations_inspected`
- `pdf_embedded_files_inspected`
- `pdf_signatures_inspected`
- `pdf_edit_planned`
- `pdf_edit_requested`
- `pdf_merge_split_planned`
- `pdf_merge_split_requested`
- `pdf_export_planned`
- `pdf_export_requested`
- `pdf_artifact_handle_resolved`
- `pdf_pack_policy_decision`
- `pdf_pack_service_call_requested`
- `pdf_pack_service_call_succeeded`
- `pdf_pack_service_call_failed`
- `pdf_pack_unavailable`
- `pdf_pack_snapshot_recorded`

Snapshots include descriptor version, provider capability hashes, document
format/profile hashes, document version hashes, command availability, provider
health, policy template hash, resource counters, bounded page/extraction/form/
annotation/signature/artifact summaries, event cursors, and sanitized replay
pointers. Snapshots must exclude raw credentials, passwords, private keys,
certificates, signatures, raw PDF bytes, decrypted payloads, full extracted
text, comments, attachments, rendered pages, exported artifacts, raw provider
payloads, prompts, manifests, package bytes, and unbounded page trees.

## Design Patterns

- **Facade**: SDK clients expose discovery and command builders only.
- **Command**: every operation is a typed command/result DTO.
- **Strategy**: provider adapters, renderers, extractors, OCR handoff, edit
  validators, signature validators, redaction providers, artifact providers,
  export/conversion providers, and unavailable behavior are replaceable.
- **Decorator**: trace, policy, entitlement, resource, approval, metering,
  password-reference handling, signature policy, artifact retention, and
  redaction wrap service calls.
- **Specification**: admission validates provider scope, document format,
  command availability, permissions, version preconditions, encryption state,
  signature policy, resource budget, and compatibility.
- **Observer**: provider health, trace, audit, service events, and artifact
  lifecycle events are subscribable.
- **Memento**: document version hashes, page handles, extraction cursors, edit
  plans, merge/split plans, export plans, artifact handles, snapshots, and
  replay pointers preserve recovery state.
- **Abstract Factory**: concrete PDF providers are created only by approved
  runtime-host composition roots.

## Risks And Mitigations

- Risk: pack becomes an Adobe/PDF.js/PDFium/iText wrapper. Mitigation:
  provider-neutral document/page/extraction/edit/export DTOs and Strategy
  adapters.
- Risk: regulated PDF content leaks. Mitigation: handles, redaction, bounded
  summaries, artifact boundaries, and strict observability exclusions.
- Risk: edits invalidate signatures or legal evidence. Mitigation: explicit
  signature preservation policy, plan/request separation, version
  preconditions, approval, and audit.
- Risk: PDF text/structure extraction quality varies by provider. Mitigation:
  confidence metadata, provider capability hashes, extraction modes, OCR handoff
  policy, and conformance tests.
- Risk: SDK helpers become a second execution path. Mitigation: helpers build
  canonical service commands and never call PDF APIs directly.
