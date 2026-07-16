# Office Presentation Pack Design

## Context

`pack.office.presentation.v1` exposes presentation capabilities as a Macaca OS
serviceized capability. It lets applications create, import, open, inspect,
edit, annotate, export, and replay slide decks without embedding Google Slides,
PowerPoint, Office.js, OpenXML PresentationML, LibreOffice Impress, cloud-drive
APIs, deck templates, or application-specific presentation workflows into
generic OS layers.

Presentations are collaborative, visual source-of-truth assets. Reads can leak
strategy, customer screenshots, speaker notes, or unreleased designs; writes can
alter narrative structure, change embedded media, or notify collaborators. The
pack therefore models writes and exports as validated plans and requests with
slide/version preconditions, format compatibility, media redaction, approval,
trace/audit evidence, replay, and provider replacement.

## Supplier Capability Matrix

| Supplier/platform | Industrial capability | Macaca abstraction |
| --- | --- | --- |
| Google Slides API | Presentations, pages, page elements, layouts, masters, notes pages, thumbnails, atomic batchUpdate | Deck, slide, shape/page element, layout/master, notes, thumbnail, batch edit plan |
| PowerPoint JavaScript API | Presentation, slides, shapes, tables, text, object model inside Office hosts | Deck handle, slide, shape, table, text range, host capability, provider adapter |
| OpenXML PresentationML | Presentation packages, slides, slide masters/layouts, notes, comments, transitions, animations, themes, media parts | Presentation package adapter, slide tree, master/layout/theme, notes/comments, transition/animation metadata, media artifact |
| Microsoft Graph / M365 file APIs | Identity, permissions, drive-item access, file content transport | Credential/file scope, artifact handle, provider permission boundary |

The pack exposes provider-neutral contracts. Provider adapters translate to
cloud presentation APIs, local file packages, desktop automation bridges, or
conversion services. OS layers must not branch on provider names, deck names,
slide titles, layouts, themes, formats, or business presentation workflows.

## Goals

- Provide stable pack id `pack.office.presentation.v1` and command namespace
  `presentation.*`.
- Support provider inspection, deck create/import/open, slide listing, structure
  inspection, layout/master/theme inspection, shape/text/table/media inspection,
  speaker notes and comments/review inspection, animation/transition inspection,
  batch edit planning, edit requests, export/thumbnail planning, export requests,
  collaboration event inspection, snapshots, health, and replay.
- Preserve safety with deck/slide/shape scope validation, format compatibility,
  version preconditions, batch validation, notes/comment privacy, media/artifact
  retention, approval, quotas, and sanitized audit.
- Keep concrete presentation providers behind replaceable service providers.
- Require developer documentation at
  `docs/developer-packs/office/presentation.md`.

## Non-Goals

- Do not implement concrete Google Slides, PowerPoint, Office.js, OpenXML,
  LibreOffice Impress, PDF, cloud-drive, OCR, rendering, or conversion providers
  in this proposal.
- Do not define application-specific sales, pitch, marketing, courseware,
  report, design review, or deck-template workflows.
- Do not execute document, spreadsheet, PDF, media rendering, storage, email, or
  notification semantics directly; those belong to separate packs/services and
  may be linked by handles.
- Do not expose raw credentials, private speaker notes, comments, customer data,
  raw slide text, raw media, raw exports, raw provider payloads, prompts,
  manifests, package bytes, private keys, signatures, or unbounded slide trees in
  observability.
- Do not silently edit slides, insert media, update notes, change animations,
  export, overwrite decks, or notify collaborators without typed request, policy
  checks, version preconditions, and approval where required.

## Ownership And Boundaries

- Pack id: `pack.office.presentation.v1`.
- Family: `office`.
- Backing service owner: presentation service provider.
- SDK surface: `sdk.packs.office.presentation`.
- Command namespace: `presentation.*`.
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
| `presentation.inspect_provider` | Inspect provider/format capability | Returns sanitized deck, slide, shape, layout, animation, export, quota, and health metadata |
| `presentation.create_deck_request` | Create a deck from metadata/template handle | Requires idempotency key, format policy, write permission, and audit |
| `presentation.import_deck_request` | Import deck from file/artifact handle | Requires artifact permission, format validation, conversion policy, and audit |
| `presentation.open_deck` | Resolve deck handle and version metadata | Requires deck scope and bounded metadata |
| `presentation.list_slides` | List slides and bounded visibility/layout metadata | Requires slide permission and redaction |
| `presentation.inspect_structure` | Inspect deck, slides, layouts, masters, themes, notes, comments, media, and transitions | Requires projection limits and redaction |
| `presentation.inspect_slide` | Inspect one slide's shapes, text, tables, media, notes, animation, and transition metadata | Requires slide scope, depth limits, and redaction |
| `presentation.inspect_assets` | Inspect deck media, images, videos, fonts, and linked assets | Requires asset permission, retention, and redaction |
| `presentation.inspect_notes` | Inspect speaker notes where supported | Requires notes permission, paging, and redaction |
| `presentation.inspect_reviews` | Inspect comments, review notes, approvals, and change events where supported | Requires review permission, paging, and redaction |
| `presentation.plan_edit` | Plan validated batch deck/slide/shape/text/media/notes/transition edits | Validates operations, handles, versions, notifications, and approvals |
| `presentation.edit_request` | Request validated batch edits | Requires plan handle, idempotency key, write permission, version preconditions, and audit |
| `presentation.plan_export` | Plan deck/slide/thumbnail/render artifact generation | Validates format, slide scope, sensitivity, retention, and approvals |
| `presentation.export_request` | Request export artifact from a validated plan | Returns bounded artifact handle and audit metadata |
| `presentation.inspect_events` | Inspect collaboration/change events where supported | Requires event filters, redaction, paging, and retention |
| `presentation.get_artifact_handle` | Resolve export/render/media artifact metadata | Requires artifact permission, retention, and redaction |

Every command must define typed command DTOs, typed success results, typed
paged/partial results, typed denied/unavailable/unsupported/conflict/
stale-version/schema-mismatch/format-unsupported/export-denied/write-denied/
asset-denied/notes-denied/quota/timeout/cancellation/approval-required/failure
results, redaction profile, idempotency semantics for side effects, and replay
metadata.

## DTO Model

Core DTOs:

- `PresentationScope`: provider scope handle, deck handle, workspace/file handle,
  credential reference, network policy, artifact policy, permission state,
  rate-limit profile, and health.
- `PresentationProviderCapability`: provider class, create/open/import support,
  slide support, layout/master/theme support, shape support, text/table support,
  media support, notes support, comments/review support, animation/transition
  support, export support, collaboration event support, auth modes, rate limits,
  lifecycle, and health.
- `DeckHandle`: deck handle, provider scope, title handle, format, version hash,
  freshness, permission state, sensitivity class, and redaction class.
- `SlideHandle`: slide handle, deck handle, index, title handle, layout handle,
  visibility, version hash, thumbnail handle, and redaction class.
- `SlideLayout`: layout handle, master handle, placeholder metadata, theme
  references, compatibility hash, and redaction class.
- `SlideMaster`: master handle, theme handle, layout handles, style references,
  version hash, and redaction class.
- `PresentationTheme`: theme handle, color/font/effect references, version hash,
  and sensitivity class.
- `PresentationShape`: shape handle, slide handle, shape kind, bounds class,
  z-order class, style references, text/table/media references, animation
  summary, version hash, and redaction class.
- `PresentationTextRange`: text range handle, shape/slide handle, text handle,
  style references, placeholder metadata, version precondition, and sensitivity
  class.
- `PresentationTable`: table handle, shape handle, row/column count class, cell
  range handles, style handle, and redaction class.
- `PresentationMedia`: media handle, slide/shape handle, media kind, content type,
  size class, linked/embed state, checksum handle, retention, and redaction
  class.
- `PresentationNotes`: notes handle, slide handle, body handle, author/source
  metadata, version hash, and redaction class.
- `PresentationReviewEvent`: event handle, deck/slide/shape handle, event kind,
  actor handle, timestamp, comment/review handle, changed fields, redaction
  class, and cursor.
- `PresentationAnimation`: animation handle, slide/shape handle, animation kind,
  trigger metadata, timing class, compatibility hash, and redaction class.
- `PresentationTransition`: transition handle, slide handle, transition kind,
  timing class, compatibility hash, and redaction class.
- `PresentationEditOperation`: operation handle, operation kind, target deck/
  slide/shape/text/media/notes handle, payload handle, and validation metadata.
- `PresentationEditPlan`: plan handle, deck handle, operation list hash, version
  preconditions, notification policy, required approvals, idempotency key, and
  validation diagnostics.
- `PresentationExportPlan`: plan handle, deck/slide scope, output format,
  rendering profile, retention, redaction, required approvals, idempotency key,
  and validation diagnostics.
- `PresentationArtifactHandle`: artifact handle, source deck/slide/media handle,
  artifact kind, content type, size class, checksum handle, retention, redaction
  class, and replay pointer.

Provider-specific extensions may appear only as bounded `adapter_metadata`
behind capability hashes and must not drive OS-layer routing.

## Permission, Policy, Resource, Entitlement, And Approval Model

Permission scopes:

- `presentation.provider.inspect`
- `presentation.deck.create`
- `presentation.deck.import`
- `presentation.deck.open`
- `presentation.slide.read`
- `presentation.structure.read`
- `presentation.asset.read`
- `presentation.notes.read`
- `presentation.review.read`
- `presentation.deck.write`
- `presentation.asset.write`
- `presentation.notes.write`
- `presentation.export`
- `presentation.events.read`
- `presentation.artifact.read`

Policy defaults:

- Every command is scoped to application id, tenant id, session id, task id,
  trace id, provider scope, deck handle, slide/shape handle when applicable, and
  actor handle when available.
- Edit, notes write, media insert, transition/animation update, and export
  commands require plan/request separation, idempotency key, version
  preconditions, format compatibility, artifact policy, notification policy,
  credential reference, and audit reason.
- Private decks, speaker notes, customer screenshots, comments, embedded media,
  unreleased branding, collaborator-visible changes, exports, destructive edits,
  and external notifications may require approval.
- Slide trees, notes, comments, media, exports, thumbnails, and artifacts require
  redaction and bounded output. Raw deck bodies and raw media must not enter
  observability.
- Remote operations require network permission, provider quota, rate limits,
  timeout, cancellation, and structured unavailable behavior.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, family, version, lifecycle, command schemas,
format support, slide support, layout/master/theme support, shape support,
text/table/media support, notes support, review support, animation/transition
support, export support, collaboration event support, permission scopes, policy
templates, resource limits, approval rules, provider capability hashes, health,
compatibility, diagnostics, examples, redaction profiles, and documentation
links.

The developer guide at `docs/developer-packs/office/presentation.md` must cover:

- manifest declaration and optional/required behavior
- provider scopes, deck handles, supported formats, slides, layouts, masters,
  themes, shapes, text ranges, tables, media, notes, comments/reviews,
  animations, transitions, edit plans, export plans, artifacts, events, provider
  capabilities, and unavailable states
- batch edit plan/request lifecycle, notes/review lifecycle, export lifecycle,
  version conflicts, schema/format mismatch, media/artifact redaction,
  notification policy, approvals, quotas, provider replacement, trace/audit
  interpretation, and conformance tests

Examples must use synthetic decks, slides, shapes, media, notes, and artifacts.
They must not include provider names, real credentials, private notes, customer
data, raw media, raw exports, or workflow-specific conventions.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `presentation_pack_declared`
- `presentation_pack_admission_validated`
- `presentation_provider_inspected`
- `presentation_deck_created`
- `presentation_deck_imported`
- `presentation_deck_opened`
- `presentation_slides_listed`
- `presentation_structure_inspected`
- `presentation_slide_inspected`
- `presentation_assets_inspected`
- `presentation_notes_inspected`
- `presentation_reviews_inspected`
- `presentation_edit_planned`
- `presentation_edit_requested`
- `presentation_export_planned`
- `presentation_export_requested`
- `presentation_events_inspected`
- `presentation_artifact_handle_resolved`
- `presentation_pack_policy_decision`
- `presentation_pack_service_call_requested`
- `presentation_pack_service_call_succeeded`
- `presentation_pack_service_call_failed`
- `presentation_pack_unavailable`
- `presentation_pack_snapshot_recorded`

Snapshots include descriptor version, provider capability hashes, deck format/
version hashes, command availability, provider health, policy template hash,
resource counters, bounded deck/slide/shape/notes/media summaries, artifact
summaries, event cursors, and sanitized replay pointers. Snapshots must exclude
raw credentials, tokens, private notes, comments, customer data, raw media, raw
exports, raw provider payloads, prompts, manifests, package bytes, private keys,
signatures, and unbounded slide trees.

## Design Patterns

- **Facade**: SDK clients expose discovery and command builders only.
- **Command**: every operation is a typed command/result DTO.
- **Strategy**: provider adapters, format readers, slide resolvers, edit
  validators, media/artifact providers, export renderers, redaction, artifact
  retention, and unavailable behavior are replaceable.
- **Decorator**: trace, policy, entitlement, resource, approval, metering,
  network policy, credential redaction, media/artifact redaction, and mutation
  safety wrap service calls.
- **Specification**: admission validates provider scope, deck/format support,
  command availability, permissions, version preconditions, format
  compatibility, provider state, quota, and compatibility.
- **Observer**: deck changes, comments/reviews, collaboration events, provider
  health, trace, and audit events are subscribable.
- **Memento**: deck version hashes, slide handles, edit plans, export plans,
  artifact handles, event cursors, snapshots, and replay pointers preserve
  recovery state.
- **Abstract Factory**: concrete presentation providers are created only by
  approved runtime-host composition roots.

## Risks And Mitigations

- Risk: pack becomes a Google Slides or PowerPoint wrapper. Mitigation:
  provider-neutral deck/slide/shape/layout/notes/export DTOs and Strategy
  adapters.
- Risk: private notes, comments, or media leak. Mitigation: handles, redaction,
  bounded summaries, and strict observability exclusions.
- Risk: edits corrupt source-of-truth decks. Mitigation: validated batch plans,
  version preconditions, idempotency, approval, and audit.
- Risk: format differences break portability. Mitigation: explicit capability
  DTO, format compatibility hashes, schema-mismatch diagnostics, and conformance
  tests.
- Risk: SDK helpers become a second execution path. Mitigation: helpers build
  canonical service commands and never call presentation APIs directly.
