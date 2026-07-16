## ADDED Requirements

### Requirement: Macaca SHALL provide Knowledge Document Parsing Pack as a serviceized capability

Macaca SHALL provide `pack.knowledge.document.parsing.v1` as a
provider-neutral industrial pack for document format detection, validation,
text/OCR extraction, layout extraction, table extraction, form/key-value
extraction, metadata extraction, canonical conversion, chunking, async parse
jobs, provider capability inspection, and unavailable diagnostics. Applications
SHALL declare the pack in manifests, admission SHALL resolve it into effective
capabilities, and all operations SHALL run through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.knowledge.document.parsing.v1` as required and a document parsing service provider is registered, healthy, entitled, format-compatible, parser-compatible, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, parser capability metadata, permission scopes, policy templates, size/page limits, async support, health, diagnostics, documentation links, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing raw credentials, raw provider payloads, raw documents, raw OCR images, raw embedded files, private signatures, unbounded text, or private corpus content

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.knowledge.document.parsing.v1` as required but provider, format support, permission, entitlement, document handle, parser capability, resource budget, or policy support is absent
- **THEN** admission SHALL block readiness with structured unavailable, denied, unsupported, validation, conflict, or quota diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.knowledge.document.parsing.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report
- **AND** SDK command helpers SHALL return Null Object unavailable diagnostics instead of creating callable service calls

### Requirement: Document parsing commands SHALL use typed canonical service calls

Every `pack.knowledge.document.parsing.v1` operation SHALL be represented as a
typed command/result DTO and SHALL traverse the canonical service runtime path
with trace, policy, resource, entitlement, approval, health, snapshot, document
validation, parser capability checks, redaction, replay, and structured error
behavior.

#### Scenario: Document is validated before parsing
- **WHEN** `document_parsing.validate_document` is invoked with a document handle
- **THEN** Macaca SHALL validate ownership, type allowlist, size/page budget, encryption policy, malware scan state, parser capability, and redaction profile
- **AND** it SHALL return typed validation diagnostics without logging raw document content

#### Scenario: Document is parsed synchronously
- **WHEN** `document_parsing.parse_document` is invoked for a bounded document and supported parser profile
- **THEN** Macaca SHALL route the command through SDK/facade helpers into the service runtime and document parsing provider
- **AND** it SHALL return typed pages, elements, text spans, tables, forms, metadata, confidence, provenance, output handles, and sanitized replay evidence

#### Scenario: Async parse job is used
- **WHEN** a document exceeds sync limits or the provider supports only long-running operations
- **THEN** Macaca SHALL use `document_parsing.start_parse_job`, expose job status through `document_parsing.get_parse_job`, and support cancellation through `document_parsing.cancel_parse_job` when provider capability allows
- **AND** parse job progress, partial results, completion, failure, cancellation, and replay pointers SHALL be auditable

#### Scenario: Command is denied before provider call
- **WHEN** policy, permission, entitlement, approval, resource, malware, type, parser capability, or redaction checks reject a parsing command
- **THEN** Macaca SHALL return a typed denied, validation, conflict, or quota result before invoking the concrete provider
- **AND** audit evidence SHALL include a bounded reason code without raw documents, raw OCR images, raw embedded files, raw provider payloads, credentials, private signatures, or unbounded extracted text

### Requirement: Document parsing DTOs SHALL model documents, jobs, pages, elements, tables, forms, metadata, chunks, geometry, and confidence

`pack.knowledge.document.parsing.v1` SHALL define portable DTOs for document
sources, parser profiles, parse jobs, pages, text spans, OCR tokens, layout
elements, tables, cells, form fields, entities, metadata, embedded resources,
chunks, geometry, confidence, parse results, provider capability, and
diagnostics. Provider-specific fields SHALL remain bounded adapter metadata and
SHALL NOT become OS-layer routing branches.

#### Scenario: Developer inspects parser capability
- **WHEN** SDK discovery or `document_parsing.inspect_parser` exposes parser metadata
- **THEN** it SHALL include supported formats, OCR languages, handwriting support, layout/table/form/entity support, embedded-resource support, async support, max bytes, max pages, output limits, confidence model, lifecycle, and health
- **AND** raw provider topology beyond policy, credentials, raw documents, and raw provider payloads SHALL NOT be exposed

#### Scenario: Developer extracts tables and forms
- **WHEN** `document_parsing.extract_tables` or `document_parsing.extract_forms` is invoked
- **THEN** Macaca SHALL return typed table cells, row/column spans, headers, key-value pairs, selection marks, signatures, entities, geometry, confidence, page anchors, and provenance
- **AND** low-confidence or unsupported structures SHALL be represented as typed warnings rather than fake successful extraction

#### Scenario: Developer chunks parsed document
- **WHEN** `document_parsing.chunk_document` is invoked with a canonical parse result and chunking policy
- **THEN** Macaca SHALL return chunk handles with source page/element anchors, offsets, token/byte counts, modality, redaction profile, and provenance
- **AND** chunk output SHALL be bounded by policy and reusable by retrieval/citation packs through declared capabilities

### Requirement: Document Parsing Pack SHALL enforce permissions, document safety, redaction, and output limits

`pack.knowledge.document.parsing.v1` SHALL define permission scopes for parsing,
OCR, text extraction, layout extraction, table extraction, form extraction,
metadata extraction, embedded-resource extraction, conversion, chunking, and
parser inspection. Policy SHALL run before side effects and SHALL account for
document ownership, type allowlist, malware state, encryption, size/page/output
budgets, provider capability, resource budgets, and approval.

#### Scenario: Missing OCR permission blocks OCR
- **WHEN** an application has text extraction permission but invokes OCR on image-only pages without `document.ocr`
- **THEN** Macaca SHALL return a typed denied result and SHALL NOT invoke OCR provider capability
- **AND** trace/audit evidence SHALL identify the missing scope by stable code

#### Scenario: Embedded file extraction is restricted
- **WHEN** a document contains embedded files or attachments
- **THEN** Macaca SHALL require `document.extract.embedded`, malware/type policy, output limits, and redaction profile before exposing embedded-resource handles
- **AND** raw embedded files SHALL NOT enter traces, audits, snapshots, SDK diagnostics, or examples

#### Scenario: Output limit is exceeded
- **WHEN** extracted text, layout elements, tables, entities, images, or chunks exceed policy limits
- **THEN** Macaca SHALL return a typed partial or quota result with bounded diagnostics and replay pointer
- **AND** it SHALL NOT emit unbounded output in observability

### Requirement: Document Parsing Pack SHALL expose industrial metadata and developer documentation

`pack.knowledge.document.parsing.v1` SHALL expose descriptor metadata for parser
capabilities, supported formats, command schemas, permission scopes, policy
templates, size/page/output limits, OCR/layout/table/form/entity support,
async-job support, resource budgets, SDK examples, lifecycle state,
compatibility, health probes, snapshots, unavailable diagnostics, redaction
profiles, and developer documentation.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.knowledge.document.parsing.v1`
- **THEN** it SHALL return command namespace `document_parsing.*`, parser capabilities, supported commands, permissions, policy templates, supported formats, examples, lifecycle, availability, health, diagnostics, compatibility, redaction profile, and documentation links
- **AND** examples SHALL use generic handles and synthetic data rather than application-specific workflows, provider names, credentials, raw documents, or business routing

#### Scenario: Developer documentation is published
- **WHEN** the pack implementation is marked complete
- **THEN** `docs/developer-packs/knowledge/document-parsing.md` SHALL document manifest declaration, permissions, document handles, validation, parser profiles, sync vs async parsing, OCR, layout, tables, forms, metadata, canonical conversion, chunking, geometry, confidence, provenance, provider replacement, unavailable diagnostics, trace/audit interpretation, and operational limits
- **AND** SDK discovery metadata and the industrial catalog index SHALL link to that guide

### Requirement: Document Parsing Pack SHALL be traceable, auditable, replayable, and sanitized

`pack.knowledge.document.parsing.v1` SHALL emit sanitized trace/audit events and
bounded snapshots for declaration, admission, format detection, validation,
parse job lifecycle, text/layout/table/form extraction, metadata extraction,
canonical conversion, chunking, policy/resource decisions, provider calls,
unavailable states, and replay.

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a document parsing pack snapshot
- **THEN** the snapshot SHALL include descriptor version, parser capability hashes, supported format summary, active job handles, page/output budgets, provider health, command availability, policy template hash, resource counters, and sanitized replay pointers
- **AND** it SHALL exclude raw documents, raw OCR images, raw provider payloads, raw embedded files, credentials, private signatures, unbounded extracted text, and private corpus content

#### Scenario: Parse job is audited
- **WHEN** a parse job starts, progresses, completes, fails, or is canceled
- **THEN** Macaca SHALL emit a sanitized audit event with stable document handle, job handle, parser profile hash, feature set, output bounds, policy decision, provider capability hash, result code, and replay pointer
- **AND** the event SHALL exclude raw document content and raw provider payloads

### Requirement: Document parsing implementation SHALL preserve Macaca boundaries

The `pack.knowledge.document.parsing.v1` implementation SHALL remain owned by
document parsing service providers behind the service runtime. The microkernel,
SDK, shells, and generic application framework SHALL remain provider-neutral and
free of application-specific or provider-specific routing branches.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete document parsing provider, OCR engine, or file-format adapter imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.knowledge.document.parsing.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, capability hash, and result codes rather than provider-specific business branches
