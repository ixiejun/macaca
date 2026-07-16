# Office PDF Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
explicit non-goals, existing platform inventory, and GitNexus memo evidence for
`pack.office.pdf.v1`. PDF support must expose document inspection, render,
extract, convert, merge, split, compress, protect, OCR, forms, annotations,
accessibility, signatures, and compliance operations through typed service
commands. It must not expose raw PDF bytes, provider APIs, private keys, or
application-specific legal/finance/identity workflows.

## Source Baseline

- Adobe Acrobat Services PDF Services and PDF Extract APIs:
  <https://developer.adobe.com/document-services/docs/apis/>
  and <https://developer.adobe.com/document-services/docs/overview/pdf-extract-api/>
- Mozilla PDF.js loading, page, viewport, canvas rendering, text, and
  annotation examples: <https://mozilla.github.io/pdf.js/examples/>
- PDFium source/API capability references:
  <https://pdfium.googlesource.com/pdfium/>
  and <https://pdfium.patagames.com/class-library/>
- iText PDF library and signature/compliance capabilities:
  <https://itextpdf.com/>
  <https://itextpdf.com/solutions/electronic-signatures-pdf>
- Poppler/PDFBox-style local provider capabilities are treated as
  implementation candidates behind provider-neutral DTOs.

## Supplier API Notes

- Adobe Acrobat Services contributes create, export, OCR, compress, linearize,
  protect, merge, split, accessibility auto-tag/check, structured JSON extract,
  Markdown extract, tables, and figures. Macaca should model cloud job handles,
  structured extract artifacts, conversion, accessibility, and quota/failure
  diagnostics without Adobe-specific commands.
- PDF.js contributes browser/Node loading, page handles, viewport calculation,
  canvas rendering, text extraction, annotation layers, progressive loading,
  and origin/browser limitations. Macaca should expose render/extract handles
  and provider resource bounds, not browser-specific layer models.
- PDFium contributes native parsing, rasterization, text extraction/search,
  forms, annotations, signature verification, modification, lifecycle, and
  progressive rendering constraints. Macaca should model local provider
  lifecycle, sandboxing, and deterministic unavailable behavior.
- iText contributes creation/manipulation, page copy/merge/split, PDF/A
  profiles, PAdES signature workflows, annotation flattening, and compliance.
  Macaca should model compliance and signature operations with key-reference
  handles, never raw private keys.

## Macaca-Owned Abstractions

`pack.office.pdf.v1` should define `PdfDocumentHandle`, `PdfPage`,
`PdfPageRange`, `PdfRenderRequest`, `PdfRenderArtifact`,
`PdfTextExtraction`, `PdfStructuredExtraction`, `PdfTableExtraction`,
`PdfAnnotation`, `PdfFormField`, `PdfMergePlan`, `PdfSplitPlan`,
`PdfConversionPlan`, `PdfProtectionPolicy`, `PdfSignatureRequest`,
`PdfComplianceProfile`, `PdfAccessibilityReport`, and
`PdfProviderCapability`.

The DTOs must carry document ownership, page bounds, render scale, extraction
limits, OCR profile, form/annotation metadata, merge/split/conversion plans,
signature key references, compliance states, accessibility diagnostics,
redaction metadata, async job state, and replay pointers. Raw PDF bytes, raw
provider payloads, raw extracted private text, private keys, signatures, and
unbounded rendered pixels are rejected.

## Explicit Non-Goals

- Do not implement concrete Adobe, PDF.js, PDFium, iText, Poppler, PDFBox, OCR,
  signing, cloud storage, or conversion providers in this research phase.
- Do not define legal contract, invoice, identity, health, finance, or
  e-signature product workflows in OS-layer code.
- Do not expose raw PDF internals, provider-native APIs, private keys, or
  provider-specific routing as stable SDK behavior.

## Existing Macaca Platform Inventory

- Generic descriptors, `SystemFacade`, trace-required service calls,
  unavailable/null-object behavior, policy/resource gates, persistence
  snapshots, secrets-reference handles, foundation file handles, and media
  rendering can support future PDF service providers.
- Current evidence does not prove PDF DTOs, providers, SDK helpers, WASM ABI
  metadata, tests, dependency gates, or developer docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
