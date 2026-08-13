## 1. Supplier API Research And Scope

- [x] 1.1 Re-read architecture governance, microkernel boundaries,
  serviceization allowlist, design-pattern guidance, and the industrial catalog
  umbrella proposal before implementation.
- [x] 1.2 Record API notes for AWS Textract text/analyze/blocks/async jobs,
  Azure Document Intelligence layout/tables/forms/selection marks/long-running
  operations, Google Document AI processors/OCR/layout/form/entity anchors,
  Apache Tika format detection/text/metadata/embedded resources, and
  Unstructured partitioning/chunking/OCR/table inference.
- [x] 1.3 Map supplier concepts to provider-neutral document source, parser
  profile, parse job, page, element, text span, OCR token, table, cell, form
  field, entity, metadata, embedded resource, chunk, geometry, confidence,
  provenance, and provider capability DTOs.
- [x] 1.4 Inventory existing service descriptors, SDK clients, admission paths,
  trace/audit schemas, optional providers, mock providers, unavailable providers,
  file/document handle services, malware/type gates, storage handles, and
  policy/resource gates that can back document parsing.
- [x] 1.5 Record GitNexus CRITICAL/HIGH findings as memo only before
  implementation commits.

## 2. Contract, Descriptor, And DTOs

- [x] 2.1 Define provider-neutral DTOs for `DocumentSource`, `ParseJob`,
  `ParserProfile`, `DocumentPage`, `DocumentElement`, `DocumentTextSpan`,
  `DocumentOcrToken`, `DocumentTable`, `DocumentTableCell`,
  `DocumentFormField`, `DocumentEntity`, `DocumentMetadata`,
  `DocumentEmbeddedResource`, `DocumentChunk`, `DocumentGeometry`,
  `DocumentConfidence`, `DocumentParseResult`, and
  `DocumentParserCapability`.
- [x] 2.2 Define typed command DTOs for `document_parsing.detect_format`,
  `document_parsing.validate_document`, `document_parsing.parse_document`,
  `document_parsing.start_parse_job`, `document_parsing.get_parse_job`,
  `document_parsing.cancel_parse_job`, `document_parsing.extract_text`,
  `document_parsing.extract_layout`, `document_parsing.extract_tables`,
  `document_parsing.extract_forms`, `document_parsing.extract_metadata`,
  `document_parsing.convert_to_canonical`, `document_parsing.chunk_document`,
  and `document_parsing.inspect_parser`.
- [x] 2.3 Define typed success, async-job, partial-result, page/table/form
  result, denied, unavailable, unsupported, validation, conflict, quota, timeout,
  canceled, and provider-failure result DTOs.
- [x] 2.4 Define descriptor metadata for pack id, supported formats, command
  schemas, permissions, policy templates, parser profiles, OCR/layout/table/form
  support, async support, size/page/output limits, redaction profile, SDK
  metadata, compatibility, diagnostics, and documentation links.
- [x] 2.5 Add descriptor hash, format detection, parser capability,
  output-limit, geometry/confidence, redaction-profile, and schema compatibility
  tests.

## 3. Admission, Permission, Policy, Resource, And Approval

- [x] 3.1 Implement declaration validation for scopes: `document.parse`,
  `document.ocr`, `document.extract.text`, `document.extract.layout`,
  `document.extract.table`, `document.extract.form`,
  `document.extract.metadata`, `document.extract.embedded`,
  `document.convert`, `document.chunk`, and `document.parser.inspect`.
- [x] 3.2 Enforce document ownership, document handle validity, type allowlist,
  malware scan state, encryption policy, size/page/output budgets, OCR policy,
  embedded-resource policy, provider capability, timeout, rate limit, approval,
  and resource budget checks before provider calls.
- [x] 3.3 Reject raw credentials, raw provider payloads, raw documents, raw OCR
  images, raw embedded files, private signatures, unbounded full text, and
  private corpus content at admission and observability boundaries.
- [x] 3.4 Model required declarations as readiness blockers and optional
  declarations as explicit degraded effective capabilities.
- [x] 3.5 Add tests proving denied, validation, quota, unsupported, and
  unavailable paths do not call concrete parsing providers.

## 4. Service Provider And Runtime Integration

- [x] 4.1 Implement or bind document parsing providers only through the service
  runtime and approved runtime-host composition roots.
- [x] 4.2 Add unavailable and mock providers with deterministic format detection,
  validation, sync parse, async job, OCR, layout, table, form, metadata,
  canonical conversion, chunking, and capability behavior.
- [x] 4.3 Add lifecycle, health, snapshot, shutdown, timeout, cancellation,
  bounded output pagination, async job state, progress events, partial results,
  and parse result handles.
- [x] 4.4 Add provider capability reporting for supported formats, OCR languages,
  handwriting support, layout/table/form/entity support, embedded resources,
  async jobs, max bytes, max pages, output limits, confidence model, rate limits,
  and health.
- [x] 4.5 Add canonical execution-path tests proving every document parsing
  command traverses SDK/facade, service runtime decorators, and provider dispatch
  exactly once.

## 5. SDK, WASM ABI, Application Framework, And Examples

- [x] 5.1 Extend SDK discovery for `pack.knowledge.document.parsing.v1` with
  command schemas, parser capability reports, examples, availability,
  diagnostics, docs metadata, policy templates, supported formats, size/page
  limits, async support, and compatibility.
- [x] 5.2 Add focused SDK helper builders that only produce canonical traced
  service calls and return Null Object unavailable diagnostics when the pack is
  absent.
- [x] 5.3 Extend WASM/application ABI metadata so applications can declare
  document parsing access, start/inspect parse jobs, and consume parse results
  only through declared permissions.
- [x] 5.4 Add generic examples for detect format, validate document, sync parse,
  async parse job, extract text, extract layout, extract tables, extract forms,
  extract metadata, convert to canonical, chunk document, inspect parser, and
  unavailable provider handling.

## 6. Trace, Audit, Replay, Security, And Gates

- [x] 6.1 Emit sanitized declaration, admission, format detection, validation,
  parse job start/progress/complete/fail/cancel, text extraction, layout
  extraction, table extraction, form extraction, canonical conversion, chunking,
  policy, resource, entitlement, approval, service-call, provider-call, health,
  snapshot, and unavailable events.
- [x] 6.2 Add replay tests proving format detection, validation, sync parse,
  async jobs, extraction commands, canonical conversion, chunking, and parser
  inspection are trace-addressable through the canonical service path.
- [x] 6.3 Add sanitization tests proving traces, audits, snapshots, SDK
  diagnostics, and examples do not leak raw credentials, raw provider payloads,
  raw documents, raw OCR images, raw embedded files, private signatures,
  unbounded extracted text, or private corpus content.
- [x] 6.4 Add dependency gates proving kernel, SDK, shells, and generic
  application framework do not import concrete parsing providers, OCR engines,
  or file-format adapters.
- [x] 6.5 Run `openspec validate add-pack-knowledge-document-parsing --strict`,
  targeted cargo tests, boundary gates, file-size gates, canonical execution-path
  tests, and audit replay checks before marking complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/knowledge/document-parsing.md` with pack
  purpose, platform comparison, manifest declaration, permission scopes,
  document handles, validation, parser profiles, sync vs async parsing, OCR,
  layout, tables, forms, metadata, canonical conversion, chunking, geometry,
  confidence, provenance, provider replacement, unavailable diagnostics,
  trace/audit interpretation, and operational limits.
- [x] 7.2 Include generic app-facing examples for detect format, validate, parse,
  async parse job, extract text/layout/tables/forms/metadata, convert to
  canonical, chunk, inspect parser, and handle unavailable provider results.
- [x] 7.3 Include provider-author guidance for descriptor metadata, parser
  profiles, supported formats, confidence models, geometry mapping, async job
  states, redaction, snapshots, quota reporting, and conformance tests.
- [x] 7.4 Cross-link the guide from SDK discovery metadata and the industrial
  pack catalog index before marking `add-pack-knowledge-document-parsing`
  complete.
