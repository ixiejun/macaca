# Knowledge Document Parsing Pack Design

## Context

`pack.knowledge.document.parsing.v1` exposes document parsing as a Macaca OS
serviceized capability. It lets applications transform declared document handles
into canonical text, layout, tables, forms, metadata, entities, chunks, and
provenance without hardcoding Textract, Azure Document Intelligence, Google
Document AI, Tika, Unstructured, OCR engines, or file-format libraries into OS
layers.

Documents are high-risk payloads. They can contain secrets, malware, embedded
files, private images, signatures, PII, and very large outputs. Parsing must
therefore be a typed service command path with size/type gates, malware hooks,
redaction profiles, async job state, bounded outputs, trace, audit, and
replayable provenance.

## Supplier Capability Matrix

| Supplier/platform | Industrial capability | Macaca abstraction |
| --- | --- | --- |
| AWS Textract | Text detection, AnalyzeDocument, block graph, pages, lines, words, tables, forms, key-values, queries, signatures, selection elements, bounding boxes, confidence, async jobs | Parse job, page block graph, OCR token, table/form DTO, query extraction, signature/selection mark, geometry, confidence, job handle |
| Azure Document Intelligence | Layout, paragraphs, tables, selection marks, key-value pairs, prebuilt/custom models, spans, bounding regions, confidence, long-running operations | Parser profile, semantic element, table/selection DTO, model capability, text span, bounding region, async operation |
| Google Document AI | Processors, OCR, form parser, layout parser, tables, entities, page/text anchors, shards, processor versions, batch processing | Processor descriptor, entity DTO, page/text anchors, document shard, version compatibility, batch parse job |
| Apache Tika | Format detection, text extraction, metadata, embedded resources, parser config, language detection | Format detector, metadata DTO, embedded-resource handle, parser profile, language metadata |
| Unstructured-style pipelines | Partitioning, semantic elements, chunking, OCR modes, table inference, image extraction, coordinates | Semantic element, chunking policy, OCR strategy, table inference capability, image/element provenance |

## Goals

- Provide stable pack id `pack.knowledge.document.parsing.v1` and command
  namespace `document_parsing.*`.
- Support format detection, validation, synchronous parsing for bounded inputs,
  asynchronous parse jobs, text extraction, OCR, layout extraction, tables,
  forms, key-values, entities, metadata, embedded resources, canonical
  conversion, chunking, and parser capability inspection.
- Model geometry, confidence, spans, anchors, page numbers, language, provenance,
  redaction, output limits, parse jobs, and provider capabilities explicitly.
- Keep provider-specific model ids and feature flags in bounded adapter metadata.
- Require developer documentation under
  `docs/developer-packs/knowledge/document-parsing.md`.

## Non-Goals

- Do not implement concrete Textract, Azure, Google, Tika, Unstructured, OCR, or
  file conversion providers in this proposal.
- Do not perform retrieval, citations, graph extraction, or summarization; those
  are separate packs that consume parsed output through declared capabilities.
- Do not expose raw documents, raw OCR images, raw provider payloads, raw
  embedded files, credentials, private signatures, or unbounded extracted text in
  logs, traces, snapshots, SDK diagnostics, or examples.
- Do not make shell UI own parsing decisions or provider feature fallback.

## Ownership And Boundaries

- Pack id: `pack.knowledge.document.parsing.v1`.
- Family: `knowledge`.
- Backing service owner: document parsing service provider.
- SDK surface: `sdk.packs.knowledge.document.parsing`.
- Command namespace: `document_parsing.*`.
- Microkernel owns identity, policy facade, resource primitives, service-call
  evidence, trace/audit primitives, and registry metadata only.
- Application framework owns manifest declarations, app-scoped permissions, and
  effective capability projection.
- Runtime host owns provider adapter registration, service decorators, file/OCR
  bridge composition, and sanitized diagnostics through approved composition
  roots.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `document_parsing.detect_format` | Detect MIME/type, container, pages, encryption, and parser compatibility | Requires document handle, size/type policy, and no raw content logging |
| `document_parsing.validate_document` | Validate file safety, size, encryption, malware scan state, and parse eligibility | Returns structured validation diagnostics before parsing |
| `document_parsing.parse_document` | Parse bounded document synchronously when provider supports it | Requires permission, redaction, output limit, and provenance |
| `document_parsing.start_parse_job` | Start async parse for large or provider-async documents | Requires job handle, resource budget, cancellation, and snapshot metadata |
| `document_parsing.get_parse_job` | Inspect async job status and bounded partial/final results | Requires job-scope permission and redacted status |
| `document_parsing.cancel_parse_job` | Cancel async parse job when supported | Must preserve audit history and partial-state diagnostics |
| `document_parsing.extract_text` | Extract text/OCR tokens/spans | Requires text extraction permission and bounded output |
| `document_parsing.extract_layout` | Extract pages, blocks, reading order, paragraphs, headings, images, and geometry | Requires layout capability and coordinate metadata |
| `document_parsing.extract_tables` | Extract tables, cells, row/column spans, headers, confidence, and geometry | Requires table capability and output limits |
| `document_parsing.extract_forms` | Extract key-values, selection marks, signatures, and form entities | Requires form/entity capability and redaction |
| `document_parsing.extract_metadata` | Extract document metadata, language, author/title where permitted, and embedded-resource summaries | Requires metadata permission |
| `document_parsing.convert_to_canonical` | Convert provider output to canonical document elements/chunks | Requires schema compatibility and provenance |
| `document_parsing.chunk_document` | Produce chunk handles for retrieval with source offsets and redaction profile | Requires chunking policy and token/byte budgets |
| `document_parsing.inspect_parser` | Inspect provider/parser capability and supported formats/features | Returns bounded metadata only |

## DTO Model

Core DTOs:

- `DocumentSource`: document handle, source type, MIME/type, size, hash, page
  estimate, encryption state, malware scan state, language hint, and provenance.
- `ParseJob`: job handle, document handle, parser profile, feature set, status,
  progress, started/completed timestamps, cancellation state, error summary, and
  replay pointer.
- `ParserProfile`: provider class, supported formats, OCR support, layout/table/
  form/entity support, language support, async support, max pages, max bytes,
  confidence model, and capability hash.
- `DocumentPage`: page index, dimensions, rotation, language, image handle,
  text span, layout element handles, and geometry.
- `DocumentElement`: element handle, type, text span, reading order, geometry,
  confidence, page anchor, parent/child handles, and redaction class.
- `DocumentTable`: table handle, page anchors, cells, headers, row/column spans,
  confidence, geometry, and extraction method.
- `DocumentFormField`: key element, value element, selection mark/signature
  metadata, entity type, confidence, geometry, and redaction class.
- `DocumentEntity`: entity handle, type, normalized value handle, text anchors,
  confidence, source model, and redaction class.
- `DocumentChunk`: chunk handle, source document/page/element anchors, text
  handle, token/byte counts, modality, redaction profile, and provenance.
- `DocumentParseResult`: document handle, parser profile, pages, elements,
  tables, forms, entities, metadata, chunks, warnings, confidence aggregates,
  and replay pointer.

## Permission, Policy, Resource, Entitlement, And Approval Model

Permission scopes:

- `document.parse`
- `document.ocr`
- `document.extract.text`
- `document.extract.layout`
- `document.extract.table`
- `document.extract.form`
- `document.extract.metadata`
- `document.extract.embedded`
- `document.convert`
- `document.chunk`
- `document.parser.inspect`

Policy defaults:

- Every command is scoped to application id, tenant id, session id, task id, and
  trace id when available.
- Parsing requires document handle validation, type allowlist, size/page budget,
  malware scan state, encryption policy, provider capability, redaction profile,
  and output limits.
- OCR, embedded-resource extraction, and external provider parsing may require
  approval when policy marks documents as sensitive or regulated.
- Extracted full text, OCR images, signatures, and embedded files require
  stronger permissions than format detection or metadata inspection.
- Raw documents, raw provider payloads, raw images, raw embedded files, and
  unbounded text are forbidden in observability.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, family, version, lifecycle, command schemas,
parser capabilities, supported formats, permission scopes, policy templates,
size/page limits, OCR/layout/table/form/entity support, async-job support,
examples, unavailable diagnostics, health, compatibility, redaction profiles,
and documentation links.

The developer guide at
`docs/developer-packs/knowledge/document-parsing.md` must cover manifest
declarations, permissions, document handles, validation, parser profiles,
synchronous vs asynchronous parsing, OCR, layout, tables, forms, metadata,
canonical conversion, chunking, geometry/confidence/provenance, unavailable
diagnostics, provider replacement, trace/audit interpretation, and conformance
tests.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `document_parsing_pack_declared`
- `document_parsing_pack_admission_validated`
- `document_format_detected`
- `document_validation_completed`
- `document_parse_job_started`
- `document_parse_job_progressed`
- `document_parse_job_completed`
- `document_parse_job_failed`
- `document_parse_job_canceled`
- `document_text_extracted`
- `document_layout_extracted`
- `document_tables_extracted`
- `document_forms_extracted`
- `document_canonical_converted`
- `document_chunks_created`
- `document_parsing_pack_policy_decision`
- `document_parsing_pack_service_call_requested`
- `document_parsing_pack_service_call_succeeded`
- `document_parsing_pack_service_call_failed`
- `document_parsing_pack_unavailable`
- `document_parsing_pack_snapshot_recorded`

Snapshots include descriptor version, parser capability hashes, supported format
summary, active job handles, page/output budgets, provider health, command
availability, policy template hash, resource counters, and sanitized replay
pointers. Snapshots must exclude raw documents, raw OCR images, raw provider
payloads, raw embedded files, credentials, private signatures, unbounded text,
and private corpus content.

## Design Patterns

- **Facade**: SDK clients expose discovery and command builders only.
- **Command**: every operation is a typed command/result DTO.
- **Strategy**: provider adapters, OCR strategy, layout/table/form extraction
  strategy, chunking strategy, and unavailable behavior are replaceable.
- **Decorator**: trace, policy, entitlement, resource, approval, metering,
  malware/type gates, and redaction wrap service calls.
- **State**: async parse jobs use explicit lifecycle states.
- **Specification**: admission validates document handles, type/size policy,
  permissions, parser capability, output limits, and compatibility.
- **Observer**: job progress, parse events, health, trace, and audit events are
  subscribable.
- **Memento**: parse job snapshots, canonical document hashes, chunk provenance,
  and replay pointers preserve recovery state.
- **Abstract Factory**: provider adapters are created only by approved runtime
  host composition roots.

## Risks And Mitigations

- Risk: raw documents leak through observability. Mitigation: document handles,
  content hashes, redaction profiles, and bounded snippets only.
- Risk: parsing large files exhausts resources. Mitigation: size/page/output
  budgets, async jobs, cancellation, and quota gates are mandatory.
- Risk: providers return incompatible layout structures. Mitigation: canonical
  element/page/table/form DTOs preserve provider confidence and provenance.
- Risk: OCR or embedded-resource extraction increases privacy exposure.
  Mitigation: separate permissions and approval-capable policy gates.
- Risk: SDK helpers become a second execution path. Mitigation: helpers only
  build canonical service-call commands and are covered by no-direct-provider
  gates.
