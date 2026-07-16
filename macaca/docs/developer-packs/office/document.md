# Office Document Pack

`pack.office.document.v1` describes provider-neutral word-processing document
capabilities. The pack is descriptor-only until a document provider is installed
through the runtime composition root.

## Manifest Declaration

Declare the pack as required only when document access is mandatory for
readiness. Optional declarations degrade with structured unavailable diagnostics.

```toml
[service_contract]
optional_packs = ["pack.office.document.v1"]
```

## Permissions

Use the narrowest scope: `document.provider.inspect`, `document.create`,
`document.import`, `document.open`, `document.structure.read`,
`document.range.read`, `document.style.read`, `document.comment.read`,
`document.comment.write`, `document.revision.read`, `document.revision.write`,
`document.edit`, `document.export`, `document.events.read`, and
`document.artifact.read`.

## Capability Model

Macaca models documents as scopes, opaque document handles, version hashes,
structures, paragraphs, runs, tables, lists, ranges, styles, comments, revisions,
edit plans, export plans, artifact handles, and collaboration events. Full text,
private comments, embedded media, provider-native batch updates, credentials,
and provider payloads stay behind provider adapters and must not appear in
traces, snapshots, or SDK diagnostics.

## Platform Comparison

Google Docs document structure, tabs, body content, and `documents.batchUpdate`
map to document structures, ranges, styles, and edit plans. Microsoft Word
JavaScript API ranges, content controls, comments, and tracked changes map to
range, comment, and revision DTOs. OpenXML/WordprocessingML paragraphs, runs,
tables, styles, comments, and revisions map to strongly typed projection DTOs.
LibreOffice UNO text ranges, fields, styles, and automation map to provider
adapter strategies. Native object models and provider-specific workflow names
remain implementation details.

## Commands

`document.inspect_provider`, `document.create_document_request`,
`document.import_document_request`, `document.open_document`,
`document.inspect_structure`, `document.read_range`, `document.inspect_styles`,
`document.inspect_comments`, `document.inspect_revisions`, `document.plan_edit`,
`document.edit_request`, `document.comment_request`, `document.redline_request`,
`document.plan_revision_resolution`,
`document.revision_resolution_request`, `document.plan_export`,
`document.export_request`, `document.inspect_events`, and
`document.get_artifact_handle` are descriptor-owned schema names. SDK helpers
build canonical traced service calls; providers execute behind the service
runtime.

## App-Facing Examples

- Inspect provider metadata before opening a document.
- Create or import a document with an idempotency key and a scoped document
  handle.
- Read structure, styles, comments, and revisions through bounded references.
- Read ranges by anchor hash and treat stale anchors as validation failures.
- Use `document.plan_edit` before `document.edit_request`; plans carry version
  preconditions and approval references for sensitive edits.
- Use comment, redline, and revision-resolution commands only when provider
  capability reports support.
- Export through `document.plan_export` and consume artifact handles instead of
  raw exports.
- Handle unavailable diagnostics without falling back to provider-specific APIs.

## App-Facing Example Matrix

Generic examples cover provider inspection, document create/import/open,
structure inspection, range reading, style/comment/revision inspection, edit
planning/request, comment request, redline request, revision resolution, export
planning/request, event inspection, and artifact handles with synthetic
document, range, revision, event, and artifact refs.

Diagnostic examples cover unavailable provider, missing document permission,
stale version, range-anchor stale, unsupported format, schema mismatch, export
denied, write approval, revision unsupported, provider quota, network denied,
comment redacted, and artifact denied. Diagnostics must not include provider
names, credentials, private comments, personal data, full document text, raw
exports, or workflow-specific conventions.

## Trace And Audit

Traces should record declaration, admission decision, command name, document id,
version hash, range anchor hash, provider class, capability hash, result status,
and artifact id. They must not record raw full text, private comments, personal
data, embedded media, raw exports, credentials, or provider payloads.

## Provider Authors

Descriptors must report formats, structure depth limits, range limits, style
support, table/list support, comment and revision support, export formats,
collaboration events, page limits, rate limits, health, and snapshot metadata.
Providers must return structured denied, unavailable, unsupported, conflict,
stale-version, schema-mismatch, format-unsupported, export-denied,
write-denied, revision-unsupported, quota, timeout, cancellation,
approval-required, and failure results without exposing native payloads.

Conformance tests should cover descriptor completeness, document and range scope
validation, format compatibility, edit validation, version conflicts,
comment/revision safety, export validation, artifact redaction, resource bounds,
policy hooks, trace and audit events, unavailable behavior, snapshot/replay, and
redaction.
