# Change: Add Office Document Pack

## Why

Developers need `pack.office.document.v1` as an industrial rich-document
capability for document creation, import, structural inspection, text/range
reading, paragraph/table/list/style operations, comments, suggestions/redlines,
revision inspection, batch edit planning, export, rendering preview handles, and
replay diagnostics. It must not be a thin wrapper around Microsoft Word, Google
Docs, OpenXML, LibreOffice UNO, or one document format.

Documents often contain confidential contracts, personal data, comments,
tracked changes, signatures, embedded media, and external links. Mutating a
document can change legal or business meaning, notify collaborators, or overwrite
source-of-truth files. Macaca must expose document operations only through
provider-neutral typed service commands with permission, policy, entitlement,
resource, approval, version preconditions, redaction, trace, audit, health,
snapshot, replay, and structured unavailable behavior.

## Research And Supplier/API Baseline

Official references considered for this pack:

- Google Docs API exposes document structures and validates `documents.batchUpdate`
  requests atomically before applying edits. Reference:
  https://developers.google.com/workspace/docs/api/reference/rest/v1/documents/batchUpdate
- Microsoft Word JavaScript API exposes Word document object access for content,
  ranges, styles, comments, and tracked-change related APIs. References:
  https://learn.microsoft.com/en-us/office/dev/add-ins/word/word-add-ins-programming-overview
  and https://learn.microsoft.com/en-us/javascript/api/word/word.trackedchange
- Open XML SDK / WordprocessingML exposes strongly typed Word document packages,
  paragraphs, runs, tables, styles, comments, revisions, and exportable document
  structures. Reference:
  https://learn.microsoft.com/en-us/office/open-xml/word/overview
- LibreOffice UNO text document APIs expose paragraphs, text ranges, styles,
  fields, tables, and document automation. Reference:
  https://wiki.documentfoundation.org/Documentation/DevGuide/Text_Documents

Macaca maps these supplier concepts into provider-neutral document handle,
document structure, section, paragraph, run, table, list, range, style, comment,
revision, batch edit plan, export plan, artifact handle, collaboration event,
version/freshness metadata, and provider capability DTOs. Concrete Word, Google
Docs, OpenXML, LibreOffice, cloud-drive, and conversion providers stay behind
replaceable providers.

## What Changes

- Add provider-neutral `pack.office.document.v1` under the `office` family.
- Define command namespace `document.*` for:
  - provider and format capability inspection
  - document creation/import/opening
  - structure/range/text/table/style/comment/revision inspection
  - batch edit planning and edit requests
  - comments and redline/suggestion requests
  - revision acceptance/rejection planning where supported
  - export/render artifact planning and requests
  - collaboration/change event inspection
  - document snapshots and replay diagnostics
- Define DTOs for document scope, provider capability, document handle,
  document structure, section, paragraph, run, table, list, range, style,
  comment, revision, edit operation, edit plan, export plan, artifact handle,
  collaboration event, version/freshness metadata, and diagnostics.
- Define permission scopes, policy defaults, document/range scopes, format
  compatibility, version-precondition behavior, collaboration notification
  policy, artifact redaction, resource/entitlement behavior, approval rules, SDK
  discovery, developer documentation, trace/audit events, snapshots, replay, and
  boundary gates.
- Require detailed developer documentation at
  `docs/developer-packs/office/document.md` before implementation completion.

## Impact

- Affected specs: `pack-office-document`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral protocol DTOs, pack descriptors,
  admission validators, SDK discovery and command builders, office document
  service provider or unavailable provider, runtime-host provider adapters,
  artifact/render/redaction support, trace/audit schemas, replay tests,
  dependency-boundary gates, and developer documentation.
- Non-goals: no concrete Word/Google Docs/OpenXML/LibreOffice/PDF/cloud-drive
  provider implementation in this proposal; no app-specific contract/legal/report
  workflow; no provider-name, document-name, template-name, style-name, or
  workflow-name routing in OS layers; no raw credentials, private comments,
  full document text, embedded media, raw provider payloads, prompts, manifests,
  or unbounded document trees in observability; no SDK/shell/kernel provider
  construction; no fake success when provider, format support, permission,
  entitlement, approval, resource, version, or host support is absent.
