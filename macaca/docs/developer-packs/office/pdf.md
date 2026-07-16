# Office PDF Pack

`pack.office.pdf.v1` describes provider-neutral PDF document capabilities. The
pack is descriptor-only until a PDF provider is installed through the runtime
composition root.

## Manifest Declaration

Declare the pack as required only when PDF access is mandatory for readiness.
Optional declarations degrade with structured unavailable diagnostics.

```toml
[service_contract]
optional_packs = ["pack.office.pdf.v1"]
```

## Permissions

Use the narrowest scope: `pdf.provider.inspect`, `pdf.document.import`,
`pdf.document.open`, `pdf.metadata.read`, `pdf.page.read`, `pdf.render`,
`pdf.text.extract`, `pdf.structure.extract`, `pdf.table.extract`,
`pdf.image.extract`, `pdf.form.read`, `pdf.form.write`,
`pdf.annotation.read`, `pdf.annotation.write`, `pdf.embedded_file.read`,
`pdf.signature.read`, `pdf.document.write`, `pdf.redaction.write`,
`pdf.merge_split`, `pdf.export`, and `pdf.artifact.read`.

## Capability Model

Macaca models PDFs as scopes, opaque document handles, metadata references,
page handles, render plans, extraction plans, text spans, structure elements,
tables, images, form fields, annotations, embedded files, signature references,
redaction operations, edit plans, merge/split plans, export plans, and artifact
handles. Raw PDF bytes, decrypted payloads, rendered page images, full extracted
text, passwords, private keys, certificates, signatures, and provider payloads
stay behind provider adapters.

## Platform Comparison

Adobe Acrobat Services create/export/extract/OCR/compress/protect concepts map
to import, extraction, edit, export, and artifact DTOs. Mozilla PDF.js loading,
page handles, viewports, rendering, text content, and annotation layers map to
page, render, text, and annotation DTOs. PDFium parsing, rasterization, forms,
annotations, signatures, and modification map to provider strategies. iText,
Poppler, and PDFBox concepts map to local adapter implementations. Native
library objects and signing key management remain outside OS semantics.

## Commands

`pdf.inspect_provider`, `pdf.import_document_request`, `pdf.open_document`,
`pdf.inspect_metadata`, `pdf.list_pages`, `pdf.render_page`,
`pdf.extract_text`, `pdf.extract_structure`, `pdf.extract_tables`,
`pdf.extract_images`, `pdf.inspect_forms`, `pdf.inspect_annotations`,
`pdf.inspect_embedded_files`, `pdf.inspect_signatures`, `pdf.plan_edit`,
`pdf.edit_request`, `pdf.plan_merge_split`, `pdf.merge_split_request`,
`pdf.plan_export`, `pdf.export_request`, and `pdf.get_artifact_handle` are
descriptor-owned schema names.

## App-Facing Examples

- Inspect provider metadata before importing or opening a PDF.
- Open a PDF through a handle that records version, profile, encryption state,
  and scope.
- List pages and render only through bounded render plans with redaction
  profiles.
- Extract text, structure, tables, and images by reference; never log raw
  extracted content.
- Inspect forms, annotations, embedded files, and signatures only when capability
  metadata reports support.
- Use edit, redaction, merge/split, and export plans before side effects.
- Treat password-required, encrypted-document, signature-policy-denied,
  redaction-denied, attachment-denied, and export-denied states as structured
  results.

## App-Facing Example Matrix

Generic examples cover provider inspection, import/open, metadata inspection,
page listing, render planning/request, text extraction, structure/table/image
extraction, form inspection, annotation inspection, embedded-file inspection,
signature references, edit planning/request, merge/split planning/request,
export planning/request, and artifact handles with synthetic document, page,
signature, attachment, render, export, and artifact refs.

Diagnostic examples cover unavailable provider, missing document permission,
password required, encrypted document, stale version, page-anchor stale,
unsupported format, schema mismatch, signature-policy denied, redaction
approval, attachment denied, export denied, provider quota, network denied, and
artifact denied. Diagnostics must not include provider names, credentials, raw
passwords, private keys, certificates, customer data, raw PDF bytes, raw
exports, or workflow-specific conventions.

## Trace And Audit

Traces should record declaration, admission decision, command name, document id,
version hash, page anchor hash, provider class, capability hash, result status,
render dimensions, signature reference id, export target, and artifact id. They
must not record raw PDF bytes, passwords, private keys, certificates, rendered
pages, decrypted text, attachments, raw exports, or provider payloads.

## Provider Authors

Descriptors must report profiles, max document bytes, render limits, extraction
limits, OCR handoff, form/annotation support, embedded-file support, signature
validation behavior, encryption behavior, redaction support, merge/split
support, export formats, health, and snapshot metadata. Providers must return
structured denied, unavailable, unsupported, conflict, stale-version,
schema-mismatch, format-unsupported, encrypted-document, password-required,
signature-invalid, signature-policy-denied, redaction-denied, export-denied,
write-denied, attachment-denied, quota, timeout, cancellation,
approval-required, and failure results.

Conformance tests should cover descriptor completeness, document/page/artifact
scope validation, encryption and password behavior, format/profile
compatibility, extraction validation, render validation, form/annotation safety,
embedded-file safety, signature policy, redaction validation, merge/split and
export validation, resource bounds, policy hooks, trace and audit events,
unavailable behavior, snapshot/replay, and redaction.
