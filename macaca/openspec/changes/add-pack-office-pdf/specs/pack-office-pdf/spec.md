## ADDED Requirements

### Requirement: Macaca SHALL expose Office PDF as a serviceized industrial pack

Macaca SHALL expose `pack.office.pdf.v1` as a provider-neutral pack for PDF
provider inspection, document import/open, metadata inspection, page listing,
page rendering, text extraction, structure extraction, table extraction, image
extraction, forms, annotations, embedded files, signature references, edit
planning, edit requests, merge/split planning, merge/split requests, export
planning, export requests, artifact handles, health, snapshots, and replay
diagnostics. The pack SHALL be declared by applications, resolved by
catalog/admission services, and invoked only through descriptor-owned `pdf.*`
service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.office.pdf.v1` as required and a PDF provider is registered, healthy, entitled, permissioned, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, permission scopes, policy template hash, resource limits, approval rules, health metadata, compatibility metadata, and replay metadata
- **AND** SDK discovery SHALL expose callable `pdf.*` commands without leaking credentials, raw passwords, private keys, certificates, signatures, raw PDF bytes, decrypted content, full extracted text, raw rendered pages, raw exports, raw provider payloads, or provider secrets

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.office.pdf.v1` as required but provider registration, host support, credential reference, password reference, permission, entitlement, resource, policy, or approval prerequisites are absent
- **THEN** admission SHALL block readiness with typed unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, contact a concrete provider, decrypt a document, mutate a document, export an artifact, transmit content, invalidate a signature silently, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.office.pdf.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability memento
- **AND** SDK helpers and WASM ABI descriptors SHALL mark unavailable commands as non-callable while preserving structured diagnostics for application recovery

### Requirement: Office PDF commands SHALL use typed canonical service calls

Every `pack.office.pdf.v1` operation SHALL be represented as a typed
command/result DTO and SHALL traverse the canonical service runtime path with
trace context, policy, resource, entitlement, approval, lifecycle, health,
snapshot, structured error, and audit behavior. SDK helpers, WASM ABI handlers,
application admission, web, CLI, and frontend code SHALL only build or submit
canonical service calls and SHALL NOT call PDF providers directly.

#### Scenario: Provider capability is inspected
- **WHEN** `pdf.inspect_provider` is invoked with declared scope and trace context
- **THEN** Macaca SHALL return sanitized provider capability metadata for document import/open, rendering, extraction, structure, tables, images, forms, annotations, embedded files, signatures, encryption, redaction, editing, merge/split, export/conversion, OCR handoff, auth, quota, lifecycle, health, and compatibility support
- **AND** the result SHALL include typed unavailable, unsupported, degraded, retired, format-limited, render-limited, extraction-limited, form-limited, annotation-limited, signature-limited, encrypted-document-limited, export-limited, network-limited, and quota-limited states when applicable

#### Scenario: Document and page reads use bounded projections
- **WHEN** `pdf.open_document`, `pdf.inspect_metadata`, `pdf.list_pages`, `pdf.extract_text`, `pdf.extract_structure`, `pdf.extract_tables`, `pdf.extract_images`, `pdf.inspect_forms`, `pdf.inspect_annotations`, `pdf.inspect_embedded_files`, `pdf.inspect_signatures`, or `pdf.get_artifact_handle` is invoked
- **THEN** Macaca SHALL enforce document, page, form, annotation, embedded-file, signature, artifact, permission, resource, and redaction scopes before provider access
- **AND** results SHALL be bounded, paged, partial, or asynchronous when needed, redacted according to policy, and represented by handles and summaries rather than raw PDF bytes, decrypted payloads, full extracted text, attachments, or unbounded page trees

#### Scenario: Unsupported command is requested
- **WHEN** a descriptor exists but the active provider does not support the requested `pdf.*` command, PDF format/profile, encryption mode, render profile, extraction mode, form feature, annotation feature, signature feature, redaction operation, merge/split operation, export format, or artifact mode
- **THEN** Macaca SHALL return a typed unsupported or format-unsupported result with descriptor and capability diagnostics
- **AND** SDK discovery SHALL report the command or feature as non-callable for the current effective capability set

### Requirement: Office PDF DTOs SHALL be provider-neutral and hash-stable

`pack.office.pdf.v1` SHALL define provider-neutral DTOs for `PdfScope`,
`PdfProviderCapability`, `PdfDocumentHandle`, `PdfMetadata`,
`PdfPageHandle`, `PdfRenderPlan`, `PdfExtractionPlan`, `PdfTextSpan`,
`PdfStructureElement`, `PdfTable`, `PdfImage`, `PdfFormField`,
`PdfAnnotation`, `PdfEmbeddedFile`, `PdfSignatureReference`,
`PdfRedactionOperation`, `PdfEditOperation`, `PdfEditPlan`,
`PdfMergeSplitPlan`, `PdfExportPlan`, and `PdfArtifactHandle`. DTOs SHALL use
stable handles, version hashes, compatibility hashes, capability hashes,
redaction classes, sensitivity classes, event cursors, and artifact handles
rather than provider object references as OS-layer semantics.

#### Scenario: Provider-specific concepts are mapped
- **WHEN** a provider exposes Adobe extraction outputs, PDF.js pages and annotations, PDFium native document/page/form/signature handles, iText PDF/A or PAdES constructs, Poppler/PDFBox structures, OCR outputs, or conversion artifacts
- **THEN** the provider adapter SHALL map those concepts into Macaca provider-neutral DTOs
- **AND** provider-specific extensions SHALL appear only as bounded `adapter_metadata` protected by capability hashes and SHALL NOT drive OS-layer routing

#### Scenario: Hashes preserve compatibility and replay
- **WHEN** Macaca serializes descriptors, provider capabilities, document formats/profiles, document versions, page anchors, render plans, extraction plans, edit plans, merge/split plans, export plans, artifact handles, signature references, event cursors, and redaction profiles
- **THEN** it SHALL produce stable hashes suitable for compatibility checks, stale-version detection, signature-policy checks, audit correlation, and replay diagnostics
- **AND** schema evolution tests SHALL prove older compatible snapshots remain readable or return typed schema-mismatch diagnostics

### Requirement: Office PDF side effects SHALL use plan/request separation

Macaca SHALL split every mutating, exporting, redacting, transmitting, or
signature-sensitive PDF operation into a non-mutating plan command and a
side-effecting request command. `pdf.plan_edit`, `pdf.plan_merge_split`, and
`pdf.plan_export` SHALL validate operations, versions, encryption state,
signature policy, format support, resource use, redaction, approvals, artifact
retention, and idempotency before `pdf.edit_request`,
`pdf.merge_split_request`, or `pdf.export_request` can perform side effects.

#### Scenario: Edit plan validates before mutation
- **WHEN** `pdf.plan_edit` receives annotation, redaction, metadata, form-fill, flattening, page-edit, protection, optimization, or attachment operations
- **THEN** Macaca SHALL validate operation schema, target handles, document version hash, page anchor freshness, encryption/password-reference state, signature preservation or invalidation policy, format compatibility, provider support, resource budget, redaction profile, and required approvals
- **AND** it SHALL return a `PdfEditPlan` with validation diagnostics without mutating the document, decrypting raw content into observability, exporting artifacts, or invalidating signatures

#### Scenario: Edit request executes a validated plan
- **WHEN** `pdf.edit_request` is invoked with a valid plan handle, idempotency key, trace context, audit reason, current version preconditions, granted approval state, and sufficient permissions
- **THEN** Macaca SHALL execute the batch through the PDF service provider and return typed success, partial, conflict, stale-version, encrypted-document, password-required, signature-policy-denied, redaction-denied, write-denied, approval-required, quota, timeout, cancellation, or failure results
- **AND** repeated requests with the same idempotency key SHALL NOT duplicate side effects

#### Scenario: Merge, split, or export request executes a validated plan
- **WHEN** `pdf.merge_split_request` or `pdf.export_request` is invoked with a valid plan, retention policy, redaction profile, artifact scope, idempotency key, and approval state
- **THEN** Macaca SHALL generate or request only bounded `PdfArtifactHandle` results
- **AND** raw output bytes SHALL remain in the artifact boundary and SHALL NOT enter trace, audit, snapshots, SDK diagnostics, or examples

### Requirement: Office PDF SHALL enforce permission, policy, resource, entitlement, and approval gates

Every `pdf.*` command SHALL be scoped to application id, tenant id, session id,
task id, trace id, provider scope, document handle, page or artifact handle when
applicable, actor handle when available, credential reference, password
reference when required, network policy, artifact policy, and permission state.
Side-effecting commands SHALL run policy, resource, entitlement, approval,
version, encryption, signature, and idempotency checks before concrete provider
calls.

#### Scenario: Permission is denied before provider access
- **WHEN** an application lacks `pdf.provider.inspect`, `pdf.document.import`, `pdf.document.open`, `pdf.metadata.read`, `pdf.page.read`, `pdf.render`, `pdf.text.extract`, `pdf.structure.extract`, `pdf.table.extract`, `pdf.image.extract`, `pdf.form.read`, `pdf.form.write`, `pdf.annotation.read`, `pdf.annotation.write`, `pdf.embedded_file.read`, `pdf.signature.read`, `pdf.document.write`, `pdf.redaction.write`, `pdf.merge_split`, `pdf.export`, or `pdf.artifact.read`
- **THEN** Macaca SHALL return a typed denied result before invoking any provider
- **AND** audit evidence SHALL include bounded reason codes and sanitized scope handles only

#### Scenario: Encrypted or signature-sensitive document is handled
- **WHEN** a command targets an encrypted document, a signed document, a document with signature references, or an operation that may invalidate signature coverage
- **THEN** Macaca SHALL require password references or signature policy evidence as applicable
- **AND** it SHALL return typed password-required, encrypted-document, signature-invalid, or signature-policy-denied diagnostics when the operation cannot proceed safely

#### Scenario: Sensitive operation requires approval
- **WHEN** a command touches private documents, regulated content, encrypted documents, redactions, embedded-file extraction, form writes, annotation flattening, destructive edits, merge/split output, exports/conversions, remote transfers, or operations that publish artifacts
- **THEN** Macaca SHALL require approval when policy marks the operation approval-gated
- **AND** denial, expiration, or missing approval SHALL return typed approval-required diagnostics without side effects

#### Scenario: Resource or entitlement is unavailable
- **WHEN** document size, page count, render resolution, extracted text size, structure depth, table count, image count, image bytes, form field count, annotation count, embedded file count or bytes, edit operation count, output size, artifact size, provider quota, network transfer, timeout, memory, storage, streaming output, retained snapshots, entitlement, or host support is insufficient
- **THEN** Macaca SHALL return typed quota, unavailable, denied, timeout, cancellation, or host-resource diagnostics
- **AND** the provider SHALL NOT be called for side-effecting operations after a failed gate

### Requirement: Office PDF artifacts and sensitive content SHALL be bounded and redacted

`pack.office.pdf.v1` SHALL treat raw PDFs, decrypted content, extracted text,
rendered pages, forms, annotations, embedded files, signatures, certificates,
redaction previews, exports, and conversion outputs as sensitive data. The pack
SHALL expose handles, bounded summaries, cursors, redaction classes, retention
metadata, and replay pointers rather than raw sensitive payloads in
observability surfaces.

#### Scenario: Text, structure, tables, and images are extracted
- **WHEN** `pdf.extract_text`, `pdf.extract_structure`, `pdf.extract_tables`, or `pdf.extract_images` is invoked with sufficient permission
- **THEN** Macaca SHALL return bounded text spans, structure elements, table handles, image handles, confidence classes, paging cursors, artifact handles, and redaction classes
- **AND** full extracted text, raw table bodies, raw images, and unbounded structure trees SHALL NOT enter traces, audits, snapshots, or SDK diagnostics

#### Scenario: Forms, annotations, embedded files, and signatures are inspected
- **WHEN** `pdf.inspect_forms`, `pdf.inspect_annotations`, `pdf.inspect_embedded_files`, or `pdf.inspect_signatures` is invoked
- **THEN** Macaca SHALL return field handles, annotation handles, embedded file handles, signature references, validation classes, retention state, sensitivity class, and redaction class
- **AND** raw form values, comment bodies, attachments, certificates, private keys, and signature payloads SHALL remain outside observability surfaces

#### Scenario: Rendered or exported artifact metadata is inspected
- **WHEN** `pdf.render_page`, `pdf.export_request`, or `pdf.get_artifact_handle` produces or resolves an artifact
- **THEN** Macaca SHALL return artifact kind, source handle, content type, size class, checksum handle, retention state, sensitivity class, and redaction class
- **AND** raw rendered pages, exported PDFs, converted files, and preview images SHALL remain behind artifact boundaries

### Requirement: Office PDF SHALL preserve Macaca architecture boundaries

The Office PDF pack implementation SHALL preserve the microkernel, service
runtime, SDK/SystemFacade, application framework, runtime-host, plugin, and
shell boundaries defined by Macaca governance. Concrete PDF providers SHALL be
replaceable Strategy adapters created only in approved runtime-host or plugin
composition roots.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, serviceization, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete Adobe, PDF.js, PDFium, iText, Poppler, PDFBox, OCR, signing, certificate, storage, conversion, rendering, credential, or artifact provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.office.pdf.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract, permission model, trace/audit schema, snapshot shape, and structured unavailable behavior
- **AND** OS layers SHALL NOT branch on provider names, file names, document titles, form names, signature labels, compliance labels, application names, or workflow names

### Requirement: Office PDF SHALL emit sanitized trace, audit, health, snapshot, and replay evidence

`pack.office.pdf.v1` SHALL emit sanitized declaration, admission,
provider-inspection, import/open, metadata-inspection, page-list, render,
extraction, form, annotation, embedded-file, signature, edit, merge/split,
export, artifact-handle, policy, entitlement, resource, approval, health,
snapshot, unavailable, and failure events. Snapshots SHALL contain enough
bounded metadata to diagnose and replay service behavior without storing raw
sensitive content.

#### Scenario: Service call evidence is recorded
- **WHEN** any `pdf.*` command is submitted
- **THEN** Macaca SHALL record trace-required service-call evidence with command name, descriptor version, sanitized scope handles, policy decision, resource decision, provider capability hash, result class, and replay pointer
- **AND** the evidence SHALL exclude raw credentials, passwords, private keys, certificates, signatures, raw PDF bytes, decrypted payloads, full extracted text, comments, attachments, rendered pages, exported artifacts, raw provider payloads, prompts, manifests, package bytes, and unbounded page trees

#### Scenario: Snapshot supports recovery diagnostics
- **WHEN** the service runtime records a PDF snapshot
- **THEN** the snapshot SHALL include descriptor version, provider capability hashes, document format/profile hashes, document version hashes, command availability, provider health, policy template hash, resource counters, bounded page/extraction/form/annotation/signature/artifact summaries, event cursors, and sanitized replay pointers
- **AND** replay tests SHALL prove every `pdf.*` command can be correlated through the canonical service path after restart

### Requirement: Office PDF SHALL provide industrial developer documentation

The implementation SHALL include a detailed developer guide at
`docs/developer-packs/office/pdf.md` before `pack.office.pdf.v1` is marked
complete. The guide SHALL be linked from SDK discovery metadata and the
industrial pack catalog index.

#### Scenario: Developer reads the guide
- **WHEN** a developer opens `docs/developer-packs/office/pdf.md`
- **THEN** the guide SHALL explain purpose, manifest declaration, required versus optional behavior, permissions, provider scopes, document handles, encrypted-document behavior, password references, page handles, metadata, render plans, extraction plans, text spans, structure trees, tables, images, forms, annotations, embedded files, signature references, redaction operations, edit plans, merge/split plans, export plans, artifacts, unavailable diagnostics, provider replacement, operational limits, and conformance expectations
- **AND** it SHALL document every command DTO and result DTO with field-level behavior, idempotency, redaction, pagination, streaming/asynchronous artifact behavior, timeout, cancellation, approval, artifact retention, version preconditions, encryption/password-reference behavior, signature policy, OCR/conversion handoff, structured errors, and trace/audit interpretation

#### Scenario: Supplier mapping is documented
- **WHEN** the documentation describes supplier/API mapping
- **THEN** it SHALL map Adobe Acrobat Services PDF operations and extraction outputs, Mozilla PDF.js page/render/text/annotation concepts, PDFium parse/render/form/signature concepts, iText merge/split/PDF-A/PAdES concepts, Poppler/PDFBox local-provider concepts, OCR handoff, and conversion concepts to Macaca abstractions
- **AND** it SHALL explicitly document what is intentionally not exposed as OS semantics

#### Scenario: Examples are provided
- **WHEN** the guide provides examples
- **THEN** examples SHALL use only synthetic PDFs, pages, text spans, tables, images, forms, annotations, signatures, embedded files, artifacts, and unavailable diagnostics
- **AND** examples SHALL NOT include provider names, real credentials, raw passwords, private keys, certificates, customer data, raw PDF bytes, raw rendered pages, raw exports, or workflow-specific conventions
