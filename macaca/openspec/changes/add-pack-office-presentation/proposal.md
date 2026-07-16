# Change: Add Office Presentation Pack

## Why

Developers need `pack.office.presentation.v1` as an industrial presentation
capability for deck creation/import/opening, slide listing, slide structure
inspection, layout/master/theme inspection, shape/text/table/media operations,
speaker notes, comments/review metadata, animation/transition metadata, batch
edit planning, export/render artifact generation, thumbnails, and replay
diagnostics. It must not be a thin wrapper around Google Slides, Microsoft
PowerPoint, Office.js, OpenXML PresentationML, LibreOffice Impress, or one deck
format.

Presentations often contain confidential product strategy, sales forecasts,
speaker notes, customer screenshots, embedded media, unreleased branding, and
collaborator comments. Mutating slides can alter source-of-truth decks, publish
private content, or notify collaborators. Macaca must therefore expose
presentation operations only through provider-neutral typed service commands
with permission, policy, entitlement, resource, approval, version preconditions,
artifact redaction, trace, audit, health, snapshot, replay, and structured
unavailable behavior.

## Research And Supplier/API Baseline

Official references considered for this pack:

- Google Slides API exposes presentations, pages, page elements, layouts,
  masters, notes pages, thumbnails, and atomic `presentations.batchUpdate`
  operations. Reference:
  https://developers.google.com/workspace/slides/api/reference/rest/v1/presentations/batchUpdate
- PowerPoint JavaScript API exposes presentation, slide, shape, table, and other
  object model surfaces for Office Add-ins. References:
  https://learn.microsoft.com/en-us/javascript/api/powerpoint/powerpoint.slide
  and https://learn.microsoft.com/en-us/javascript/api/powerpoint/powerpoint.shape
- OpenXML PresentationML exposes presentation package structure, slides, slide
  masters, layouts, notes slides, handout masters, comments, transitions,
  animations, themes, and media parts. References:
  https://learn.microsoft.com/en-us/office/open-xml/presentation/structure-of-a-presentationml-document
  and https://learn.microsoft.com/en-us/office/open-xml/presentation/working-with-presentation-slides
- Microsoft Graph and Microsoft 365 APIs provide identity, file, permission, and
  drive-item access around PowerPoint files; content editing remains provider-
  and host-dependent. Reference: https://learn.microsoft.com/en-us/graph/

Macaca maps these supplier concepts into provider-neutral presentation scope,
deck, slide, slide layout, slide master, theme, shape, text range, table, media,
speaker notes, comment/review event, animation, transition, batch edit plan,
export plan, thumbnail/artifact handle, version/freshness metadata, and provider
capability DTOs. Concrete Google Slides, PowerPoint, Office.js, OpenXML,
LibreOffice Impress, cloud-drive, and conversion providers stay behind
replaceable providers.

## What Changes

- Add provider-neutral `pack.office.presentation.v1` under the `office` family.
- Define command namespace `presentation.*` for:
  - provider and format capability inspection
  - deck creation/import/opening and slide listing
  - deck/slide/layout/master/theme/shape/text/table/media inspection
  - speaker notes and comments/review inspection
  - animation and transition inspection where supported
  - batch edit planning and edit requests
  - notes/comment/shape/media operations
  - export/thumbnail/render artifact planning and requests
  - collaboration/change event inspection
  - deck snapshots and replay diagnostics
- Define DTOs for presentation scope, provider capability, deck handle, slide,
  slide layout, slide master, theme, shape, text range, table, media, speaker
  notes, comment, animation, transition, edit operation, edit plan, export plan,
  artifact handle, collaboration event, version/freshness metadata, and
  diagnostics.
- Define permission scopes, policy defaults, deck/slide/shape scopes, format
  compatibility, version-precondition behavior, media/artifact redaction,
  collaborator notification policy, resource/entitlement behavior, approval
  rules, SDK discovery, developer documentation, trace/audit events, snapshots,
  replay, and boundary gates.
- Require detailed developer documentation at
  `docs/developer-packs/office/presentation.md` before implementation
  completion.

## Impact

- Affected specs: `pack-office-presentation`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral protocol DTOs, pack descriptors,
  admission validators, SDK discovery and command builders, presentation service
  provider or unavailable provider, runtime-host provider adapters,
  media/render/artifact/redaction support, trace/audit schemas, replay tests,
  dependency-boundary gates, and developer documentation.
- Non-goals: no concrete Google Slides/PowerPoint/Office.js/OpenXML/LibreOffice/
  PDF/cloud-drive/conversion provider implementation in this proposal; no
  app-specific pitch deck, sales deck, courseware, marketing, or template
  workflow; no provider-name, deck-name, slide-name, layout-name, theme-name, or
  workflow-name routing in OS layers; no raw credentials, private notes,
  comments, unreleased slides, raw media, raw exports, raw provider payloads,
  prompts, manifests, or unbounded slide trees in observability; no
  SDK/shell/kernel provider construction; no fake success when provider, format
  support, permission, entitlement, approval, resource, version, or host support
  is absent.
