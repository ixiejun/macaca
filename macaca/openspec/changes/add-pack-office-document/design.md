# Office Document Pack Design

## Context

`pack.office.document.v1` exposes rich document capabilities as a Macaca OS
serviceized capability. It lets applications create, import, open, inspect,
edit, comment, redline, export, and replay documents without embedding Microsoft
Word, Google Docs, OpenXML, LibreOffice UNO, cloud-drive APIs, document template
names, or application-specific document workflows into generic OS layers.

Documents are collaborative source-of-truth assets. Reads can leak contracts or
personal data; writes can alter legal meaning, modify tracked changes, or notify
collaborators. The pack therefore models writes as validated batch plans and
requests with range/version preconditions, format compatibility, redaction,
approval, trace/audit evidence, replay, and provider replacement.

## Supplier Capability Matrix

| Supplier/platform | Industrial capability | Macaca abstraction |
| --- | --- | --- |
| Google Docs API | Structured documents, document tabs/body/content, atomic `batchUpdate`, styles, tables, lists, inline objects | Document structure, range, batch edit plan, style/table/list operations, version preconditions |
| Microsoft Word JavaScript API | Word document object model, ranges, content controls, comments, styles, tracked changes | Document handle, range, comment, style, revision, collaboration event |
| OpenXML / WordprocessingML | Package parts, paragraphs, runs, tables, styles, comments, revisions, strongly typed document structures | Provider-neutral document AST, style catalog, revision metadata, import/export artifact |
| LibreOffice UNO Writer | Text documents, paragraphs, ranges, fields, styles, tables, automation APIs | Document model adapter, text range, style family, field/table metadata, provider capability |

The pack exposes provider-neutral contracts. Provider adapters translate to
cloud documents, local files, OpenXML packages, conversion services, or desktop
automation bridges. OS layers must not branch on provider names, document names,
templates, styles, formats, or business document workflows.

## Goals

- Provide stable pack id `pack.office.document.v1` and command namespace
  `document.*`.
- Support provider inspection, create/import/open, structure inspection, range
  reading, style/table/list/comment/revision inspection, batch edit planning,
  edit requests, comment requests, redline/suggestion requests, revision
  accept/reject planning where supported, export/render planning, export
  requests, collaboration event inspection, snapshots, health, and replay.
- Preserve safety with document/range scope validation, format compatibility,
  version preconditions, batch validation, collaboration notification policy,
  artifact retention, approval, quotas, and sanitized audit.
- Keep concrete document providers behind replaceable service providers.
- Require developer documentation at
  `docs/developer-packs/office/document.md`.

## Non-Goals

- Do not implement concrete Word, Google Docs, OpenXML, LibreOffice, PDF,
  cloud-drive, OCR, or conversion providers in this proposal.
- Do not define application-specific legal, report, contract, resume, invoice,
  publishing, review, or document-template workflows.
- Do not execute spreadsheet, presentation, PDF, forms, storage, email, or
  notification semantics directly; those belong to separate packs/services and
  may be linked by handles.
- Do not expose raw credentials, full document text, private comments, tracked
  change content, embedded media, raw exports, raw provider payloads, prompts,
  manifests, package bytes, private keys, signatures, or unbounded document trees
  in observability.
- Do not silently edit, comment, redline, export, overwrite, or notify
  collaborators without typed request, policy checks, version preconditions, and
  approval where required.

## Ownership And Boundaries

- Pack id: `pack.office.document.v1`.
- Family: `office`.
- Backing service owner: office document service provider.
- SDK surface: `sdk.packs.office.document`.
- Command namespace: `document.*`.
- Microkernel owns identity, policy facade, resource primitives, service-call
  evidence, trace/audit primitives, and registry metadata only.
- Application framework owns manifest declarations, app-scoped permissions, and
  effective capability projection.
- Runtime host owns provider adapter registration, credential bridges, artifact
  stores, conversion/desktop bridges, decorators, and sanitized diagnostics
  through approved composition roots.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `document.inspect_provider` | Inspect provider/format capability | Returns sanitized format, structure, edit, comment, revision, export, quota, and health metadata |
| `document.create_document_request` | Create a new document from metadata/template handle | Requires idempotency key, format policy, write permission, and audit |
| `document.import_document_request` | Import a document from file/artifact handle | Requires file/artifact permission, format validation, conversion policy, and audit |
| `document.open_document` | Resolve a document handle and version metadata | Requires document scope and bounded metadata |
| `document.inspect_structure` | Inspect sections, headings, body, tables, lists, styles, comments, and revisions | Requires projection, depth limits, and redaction |
| `document.read_range` | Read bounded text/content from a range | Requires range scope, content bounds, and redaction |
| `document.inspect_styles` | Inspect styles/themes/numbering/list definitions | Requires style permission and bounded output |
| `document.inspect_comments` | Inspect comments and threads where supported | Requires comment permission, paging, and redaction |
| `document.inspect_revisions` | Inspect tracked changes/suggestions where supported | Requires revision permission, paging, and redaction |
| `document.plan_edit` | Plan validated batch edits | Validates operations, ranges, styles, version preconditions, notifications, and approvals |
| `document.edit_request` | Request validated batch edits | Requires plan handle, idempotency key, write permission, version preconditions, and audit |
| `document.comment_request` | Request adding/updating/resolving comments | Requires comment permission, visibility policy, notifications, and audit |
| `document.redline_request` | Request suggestion/redline operations where supported | Requires revision permission, compatibility, approvals, and audit |
| `document.plan_revision_resolution` | Plan accept/reject revisions where supported | Validates revision handles, version preconditions, and approvals |
| `document.revision_resolution_request` | Request validated revision accept/reject | Requires plan handle, idempotency key, and audit |
| `document.plan_export` | Plan export/render artifact generation | Validates format, page/range scope, sensitivity, retention, and approvals |
| `document.export_request` | Request export artifact from a validated plan | Returns bounded artifact handle and audit metadata |
| `document.inspect_events` | Inspect collaboration/change events where supported | Requires event filters, redaction, paging, and retention |
| `document.get_artifact_handle` | Resolve export/render/import artifact metadata | Requires artifact permission, retention, and redaction |

Every command must define typed command DTOs, typed success results, typed
paged/partial results, typed denied/unavailable/unsupported/conflict/
stale-version/schema-mismatch/format-unsupported/export-denied/write-denied/
revision-unsupported/quota/timeout/cancellation/approval-required/failure
results, redaction profile, idempotency semantics for side effects, and replay
metadata.

## DTO Model

Core DTOs:

- `DocumentScope`: provider scope handle, document handle, workspace/file handle,
  credential reference, network policy, artifact policy, permission state,
  rate-limit profile, and health.
- `DocumentProviderCapability`: provider class, create/open/import support,
  structure support, range support, style support, table/list support, comment
  support, revision support, export support, collaboration event support, auth
  modes, rate limits, lifecycle, and health.
- `DocumentHandle`: document handle, provider scope, title handle, format,
  version hash, freshness, permission state, sensitivity class, and redaction
  class.
- `DocumentStructure`: document handle, section summaries, heading outline,
  body summary, table/list/style/comment/revision summaries, version hash, and
  projection metadata.
- `DocumentRange`: range handle, document handle, section/paragraph/table/cell
  scope, start/end anchors, version precondition, and redaction class.
- `DocumentParagraph`: paragraph handle, range handle, style handle, text handle,
  run count class, list metadata, revision summary, and redaction class.
- `DocumentRun`: run handle, text handle, style references, field/link metadata,
  revision metadata, and sensitivity class.
- `DocumentTable`: table handle, row/column count class, cell range handles,
  style handle, and redaction class.
- `DocumentStyle`: style handle, style kind, parent style, properties handle,
  numbering/list metadata, compatibility hash, and sensitivity class.
- `DocumentComment`: comment handle, document/range handle, author handle,
  body handle, state, visibility, thread metadata, version hash, and redaction
  class.
- `DocumentRevision`: revision handle, document/range handle, revision kind,
  author handle, timestamp, content handle, state, version hash, and redaction
  class.
- `DocumentEditOperation`: operation handle, operation kind, target range/style/
  table/comment/revision handle, payload handle, and validation metadata.
- `DocumentEditPlan`: plan handle, document handle, operation list hash, version
  preconditions, notification policy, required approvals, idempotency key, and
  validation diagnostics.
- `DocumentExportPlan`: plan handle, document/range/page scope, output format,
  rendering profile, retention, redaction, required approvals, idempotency key,
  and validation diagnostics.
- `DocumentArtifactHandle`: artifact handle, source document/range handle,
  artifact kind, content type, size class, checksum handle, retention, redaction
  class, and replay pointer.
- `DocumentCollaborationEvent`: event handle, document/range handle, event kind,
  actor handle, timestamp, changed fields, comment/revision handle, redaction
  class, and cursor.

Provider-specific extensions may appear only as bounded `adapter_metadata`
behind capability hashes and must not drive OS-layer routing.

## Permission, Policy, Resource, Entitlement, And Approval Model

Permission scopes:

- `document.provider.inspect`
- `document.create`
- `document.import`
- `document.open`
- `document.structure.read`
- `document.range.read`
- `document.style.read`
- `document.comment.read`
- `document.comment.write`
- `document.revision.read`
- `document.revision.write`
- `document.edit`
- `document.export`
- `document.events.read`
- `document.artifact.read`

Policy defaults:

- Every command is scoped to application id, tenant id, session id, task id,
  trace id, provider scope, document handle, range handle when applicable, and
  actor handle when available.
- Edit, comment, redline, revision-resolution, and export commands require typed
  request or plan/request separation, idempotency key, version preconditions,
  format compatibility, artifact policy, notification policy, credential
  reference, and audit reason.
- Private documents, contracts, personal data, comments, revisions, embedded
  media, collaborator-visible changes, exports, destructive edits, and external
  notifications may require approval.
- Document trees, comments, revisions, text ranges, exports, and artifacts
  require redaction and bounded output. Raw full document bodies must not enter
  observability.
- Remote operations require network permission, provider quota, rate limits,
  timeout, cancellation, and structured unavailable behavior.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, family, version, lifecycle, command schemas,
format support, structure support, range support, style support, comment support,
revision support, export support, collaboration event support, permission
scopes, policy templates, resource limits, approval rules, provider capability
hashes, health, compatibility, diagnostics, examples, redaction profiles, and
documentation links.

The developer guide at `docs/developer-packs/office/document.md` must cover:

- manifest declaration and optional/required behavior
- provider scopes, document handles, supported formats, structures, sections,
  paragraphs, runs, tables, lists, ranges, styles, comments, revisions, edit
  plans, export plans, artifacts, events, provider capabilities, and unavailable
  states
- batch edit plan/request lifecycle, comment/redline lifecycle, revision
  resolution lifecycle, export lifecycle, version conflicts, schema/format
  mismatch, artifact redaction, notification policy, approvals, quotas, provider
  replacement, trace/audit interpretation, and conformance tests

Examples must use synthetic documents, ranges, comments, revisions, and
artifacts. They must not include provider names, real credentials, private
comments, customer data, full document text, raw exports, or workflow-specific
conventions.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `document_pack_declared`
- `document_pack_admission_validated`
- `document_provider_inspected`
- `document_created`
- `document_imported`
- `document_opened`
- `document_structure_inspected`
- `document_range_read`
- `document_styles_inspected`
- `document_comments_inspected`
- `document_revisions_inspected`
- `document_edit_planned`
- `document_edit_requested`
- `document_comment_requested`
- `document_redline_requested`
- `document_revision_resolution_planned`
- `document_revision_resolution_requested`
- `document_export_planned`
- `document_export_requested`
- `document_events_inspected`
- `document_artifact_handle_resolved`
- `document_pack_policy_decision`
- `document_pack_service_call_requested`
- `document_pack_service_call_succeeded`
- `document_pack_service_call_failed`
- `document_pack_unavailable`
- `document_pack_snapshot_recorded`

Snapshots include descriptor version, provider capability hashes, document
format/version hashes, command availability, provider health, policy template
hash, resource counters, bounded document/range/comment/revision summaries,
artifact summaries, event cursors, and sanitized replay pointers. Snapshots must
exclude raw credentials, tokens, private comments, personal data, raw full
document text, raw embedded media, raw exports, raw provider payloads, prompts,
manifests, package bytes, private keys, signatures, and unbounded document trees.

## Design Patterns

- **Facade**: SDK clients expose discovery and command builders only.
- **Command**: every operation is a typed command/result DTO.
- **Strategy**: provider adapters, format readers, range resolvers, edit
  validators, comment/revision strategies, export renderers, redaction,
  artifact retention, and unavailable behavior are replaceable.
- **Decorator**: trace, policy, entitlement, resource, approval, metering,
  network policy, credential redaction, artifact redaction, and mutation safety
  wrap service calls.
- **Specification**: admission validates provider scope, document/format
  support, command availability, permissions, version preconditions, format
  compatibility, provider state, quota, and compatibility.
- **Observer**: document changes, comments, revisions, collaboration events,
  provider health, trace, and audit events are subscribable.
- **Memento**: document version hashes, range anchors, edit plans, export plans,
  artifact handles, event cursors, snapshots, and replay pointers preserve
  recovery state.
- **Abstract Factory**: concrete document providers are created only by approved
  runtime-host composition roots.

## Risks And Mitigations

- Risk: pack becomes a Word or Google Docs wrapper. Mitigation:
  provider-neutral document/range/style/comment/revision/export DTOs and
  Strategy adapters.
- Risk: confidential document content leaks. Mitigation: handles, redaction,
  bounded summaries, and strict observability exclusions.
- Risk: edits corrupt source-of-truth documents. Mitigation: validated batch
  plans, version preconditions, idempotency, approval, and audit.
- Risk: format differences break portability. Mitigation: explicit capability
  DTO, format compatibility hashes, schema-mismatch diagnostics, and conformance
  tests.
- Risk: SDK helpers become a second execution path. Mitigation: helpers build
  canonical service commands and never call document APIs directly.
