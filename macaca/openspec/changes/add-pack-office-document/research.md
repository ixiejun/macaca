# Office Document Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
explicit non-goals, existing platform inventory, and GitNexus memo evidence for
`pack.office.document.v1`. Document editing must expose structured document
operations through serviceized commands. It must not turn Google Docs, Word,
OpenXML, LibreOffice, PDF, OCR, template, legal, or reporting workflows into
OS-layer semantics.

## Source Baseline

- Google Docs API document tabs and `documents.batchUpdate`:
  <https://developers.google.com/workspace/docs/api/how-tos/tabs>
  and
  <https://developers.google.com/workspace/docs/api/reference/rest/v1/documents/batchUpdate>
- Microsoft Word JavaScript API `Range`, `ContentControl`, and tracked-change
  surfaces:
  <https://learn.microsoft.com/en-us/javascript/api/word/word.range>
  <https://learn.microsoft.com/en-us/javascript/api/word/word.contentcontrol>
  <https://learn.microsoft.com/en-us/javascript/api/word/word.trackedchange>
- OpenXML WordprocessingML structure, runs, and WordprocessingML overview:
  <https://learn.microsoft.com/en-us/office/open-xml/word/structure-of-a-wordprocessingml-document>
  <https://learn.microsoft.com/en-us/office/open-xml/word/working-with-runs>
- LibreOffice UNO Writer text documents:
  <https://wiki.documentfoundation.org/Documentation/DevGuide/Text_Documents>
  and <https://api.libreoffice.org/examples/examples.html>

## Supplier API Notes

- Google Docs contributes a remote document model with tabs, body content,
  structural elements, styles, tables, lists, inline objects, revision-aware
  writes, and atomic `batchUpdate` validation. Macaca should map this into
  document, segment, tab, range, style, mutation batch, and revision guards.
- Word JavaScript contributes host-scoped documents, ranges, content controls,
  comments, styles, and tracked-change inspection. Macaca should model host
  capability and tracked-change support as provider capability metadata, not as
  an always-available OS invariant.
- OpenXML WordprocessingML contributes strongly typed package parts,
  paragraphs, runs, tables, styles, comments, relationships, revisions, and
  deterministic offline package mutation. Macaca should represent package
  import/export and structural editing without exposing XML element names as
  the stable SDK contract.
- LibreOffice UNO contributes local automation over text documents, paragraphs,
  text ranges, styles, fields, tables, frames, and document lifecycle. Macaca
  should treat UNO as a local provider candidate behind lifecycle, resource,
  timeout, and unavailable diagnostics.

## Macaca-Owned Abstractions

`pack.office.document.v1` should define `DocumentHandle`, `DocumentSnapshot`,
`DocumentSegment`, `DocumentRange`, `DocumentBlock`, `DocumentInline`,
`DocumentTextRun`, `DocumentStyle`, `DocumentTable`, `DocumentList`,
`DocumentComment`, `DocumentRevision`, `DocumentMutation`,
`DocumentBatchResult`, and `DocumentProviderCapability`.

The DTOs must carry ownership, revision preconditions, range identity,
structural selection, style metadata, table/list models, comment/revision
metadata, mutation idempotency, import/export handles, redaction profile, and
replay pointers. Raw provider payloads, full private documents, raw package
bytes, credentials, provider-native XML/JSON, and unbounded text are rejected at
SDK and observability boundaries.

## Explicit Non-Goals

- Do not implement concrete Google Docs, Word, Office.js, OpenXML,
  LibreOffice, PDF, cloud-drive, OCR, or conversion providers in this research
  phase.
- Do not define legal, report, mail-merge, proposal, template, resume, or other
  application-specific document workflows in OS layers.
- Do not expose provider-native request bodies, WordprocessingML element names,
  UNO service names, or Google Docs JSON as stable SDK commands.
- Do not let shells or applications bypass policy with raw document mutations.

## Existing Macaca Platform Inventory

- Generic domain-pack descriptors, `SystemFacade`, trace-required service
  calls, unavailable/null-object clients, policy/resource command objects, and
  persistence snapshots provide the substrate for a future document service.
- Foundation file handles, office PDF, knowledge document parsing, and media
  rendering proposals define adjacent capabilities that document must consume
  through declared handles rather than direct coupling.
- Current evidence does not prove document DTOs, providers, SDK helpers, WASM
  ABI metadata, canonical path tests, redaction tests, or developer docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
