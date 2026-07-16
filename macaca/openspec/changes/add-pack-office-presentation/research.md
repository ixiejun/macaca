# Office Presentation Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
explicit non-goals, existing platform inventory, and GitNexus memo evidence for
`pack.office.presentation.v1`. Presentation support must expose deck, slide,
layout, master, notes, page element, shape, table, text, media, thumbnail,
theme, animation/transition, and import/export operations through serviceized
commands, not provider-native slide APIs or sales/template workflows.

## Source Baseline

- Google Slides API overview, presentations resource, and batch updates:
  <https://developers.google.com/workspace/slides/api/guides/overview>
  and <https://developers.google.com/workspace/slides/api/reference/rest/v1/presentations>
- PowerPoint JavaScript API shapes and core concepts:
  <https://learn.microsoft.com/en-us/office/dev/add-ins/powerpoint/shapes>
  and <https://github.com/OfficeDev/office-js-docs-pr/blob/main/docs/powerpoint/core-concepts.md>
- OpenXML PresentationML structure, slide masters, slide layouts, themes:
  <https://learn.microsoft.com/en-us/office/open-xml/presentation/structure-of-a-presentationml-document>
  <https://learn.microsoft.com/en-us/office/open-xml/presentation/working-with-slide-masters>
  <https://learn.microsoft.com/en-us/office/open-xml/presentation/working-with-slide-layouts>
- Microsoft Graph drive items, thumbnails, and format conversion:
  <https://learn.microsoft.com/en-us/graph/api/resources/driveitem>
  <https://learn.microsoft.com/en-us/graph/api/driveitem-list-thumbnails>
  <https://learn.microsoft.com/en-us/graph/api/driveitem-get-content-format>

## Supplier API Notes

- Google Slides contributes presentations, pages, page elements, layouts,
  masters, notes pages, thumbnails, and atomic `presentations.batchUpdate`
  operations. Macaca should model deck, page, element, layout/master, notes,
  mutation batch, revision guard, and thumbnail handle abstractions.
- PowerPoint JavaScript contributes host-scoped presentation, slide, shape,
  text, table, formatting, and asynchronous object model behavior. Macaca
  should surface host capability and unsupported states instead of assuming a
  complete provider-independent edit surface.
- OpenXML PresentationML contributes package parts, slides, slide masters,
  layouts, notes slides, handout masters, comments, transitions, animations,
  themes, and media relationships. Macaca should support package import/export
  and structural editing without exposing XML part names as stable SDK API.
- Microsoft Graph contributes file identity, permission, drive-item, thumbnail,
  and conversion boundaries for PowerPoint files. Macaca should separate file
  transport/access from presentation editing commands.

## Macaca-Owned Abstractions

`pack.office.presentation.v1` should define `PresentationDeck`,
`PresentationSlide`, `PresentationLayout`, `PresentationMaster`,
`PresentationNotes`, `PresentationElement`, `PresentationShape`,
`PresentationText`, `PresentationTable`, `PresentationMedia`,
`PresentationTheme`, `PresentationTransition`, `PresentationAnimation`,
`PresentationThumbnail`, `PresentationMutation`, and
`PresentationProviderCapability`.

The DTOs must carry deck identity, slide ordering, layout/master inheritance,
element geometry, text/table/media handles, notes visibility, mutation
preconditions, thumbnail/export handles, capability hashes, redaction metadata,
and replay pointers. Raw provider payloads, private slide content, package
bytes, credentials, remote URLs, and unbounded rendered images are rejected.

## Explicit Non-Goals

- Do not implement concrete Google Slides, PowerPoint, Office.js, OpenXML,
  LibreOffice Impress, PDF, cloud-drive, OCR, rendering, or conversion providers
  in this research phase.
- Do not define pitch decks, sales decks, school slides, reports, templates, or
  brand-specific workflows in OS layers.
- Do not expose provider-native batch requests, PresentationML part names,
  Graph file ids, or host object models as stable application contracts.

## Existing Macaca Platform Inventory

- Generic descriptors, `SystemFacade`, trace-required service calls,
  unavailable/null-object providers, policy/resource gates, persistence
  snapshots, file handles, media rendering, and office PDF proposals provide
  reusable substrate.
- Current evidence does not prove presentation DTOs, providers, SDK helpers,
  WASM ABI metadata, tests, dependency gates, or developer docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
