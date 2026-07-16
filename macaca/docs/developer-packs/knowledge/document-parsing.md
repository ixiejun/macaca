# Knowledge Document Parsing Pack

`pack.knowledge.document.parsing.v1` describes document validation, format
detection, OCR, layout extraction, table extraction, form extraction, metadata
extraction, canonical conversion, chunking, and parser inspection.

## Manifest Declaration

```toml
[service_contract]
optional_packs = ["pack.knowledge.document.parsing.v1"]
```

Applications provide document handles. The OS contract never carries raw
document bytes, raw OCR images, raw embedded files, or unbounded extracted text.

## Permissions

Use `document.parse`, `document.ocr`, `document.extract.text`,
`document.extract.layout`, `document.extract.table`, `document.extract.form`,
`document.extract.metadata`, `document.extract.embedded`, `document.convert`,
`document.chunk`, and `document.parser.inspect`.

## Capability Model

DTOs cover document source, parse job, parser profile, pages, elements, text
spans, OCR tokens, tables, cells, form fields, entities, metadata, embedded
resources, chunks, geometry, confidence, parse results, and parser capability.
Geometry and confidence are normalized so providers can be replaced without
leaking native block graphs.

## Platform Comparison

AWS Textract text detection, analyze-document blocks, tables, forms, key-value
pairs, selection elements, signatures, geometry, confidence, and async jobs map
to parse job, page, element, OCR token, table, form field, geometry, confidence,
and parser capability DTOs. Azure Document Intelligence layout, spans,
paragraphs, tables, selection marks, key-value pairs, and long-running
operations map to parser profiles, elements, text spans, tables, forms, and
jobs. Google Document AI processors, entities, anchors, and layout map to
parser capability, entity, text span, and geometry DTOs. Apache Tika detection,
metadata, text extraction, and embedded resources map to source validation,
metadata, text-span, and embedded-resource DTOs. Unstructured partitioning,
chunking, OCR, and table inference map to canonical conversion and chunk DTOs.

## Commands

`document_parsing.detect_format`, `validate_document`, `parse_document`,
`start_parse_job`, `get_parse_job`, `cancel_parse_job`, `extract_text`,
`extract_layout`, `extract_tables`, `extract_forms`, `extract_metadata`,
`convert_to_canonical`, `chunk_document`, and `inspect_parser` are canonical
command schema names.

## App-Facing Examples

- Detect format before parsing to check media type, encryption, page count, and
  parser compatibility.
- Validate document handles before OCR or extraction.
- Run sync parse for small bounded files and async parse jobs for larger files.
- Inspect parse jobs and cancel them through job handles.
- Extract text, layout, tables, forms, or metadata as separate commands when an
  application needs only part of the document.
- Convert provider output into canonical elements before chunking for retrieval.
- Use geometry and confidence DTOs to display or audit extraction quality.
- Inspect parser capability before requesting unsupported OCR languages,
  embedded-resource extraction, or table/form inference.
- Handle unavailable providers through structured diagnostics without reading
  raw file bytes.

## Trace And Audit

Trace metadata should include document handle id, media type, parser profile,
command name, job id, provider class, capability hash, output limit, and result
status. Raw documents, OCR images, embedded files, unbounded text, provider
payloads, and credentials must not enter observability surfaces.

## Provider Authors

Providers must report supported formats, OCR languages, handwriting, layout,
table, form, entity, embedded-resource, async-job, max-byte, max-page,
confidence-model, redaction, quota, health, and snapshot capability. Denied,
validation, quota, unsupported, unavailable, canceled, timeout, and failure
paths must not call concrete parsers after admission rejects a request.

Conformance tests should cover descriptor completeness, format detection,
document validation, output limits, geometry/confidence mapping, async job
states, redaction, unavailable behavior, and provider capability reporting.
