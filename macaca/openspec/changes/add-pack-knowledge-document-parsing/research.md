# Knowledge Document Parsing Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
existing platform inventory, and GitNexus memo evidence for
`pack.knowledge.document.parsing.v1`. Document parsing must transform declared
document handles into canonical text, layout, table, form, entity, metadata,
chunk, geometry, confidence, and provenance DTOs through serviceized commands.
It must not expose raw documents, OCR images, embedded files, or provider block
graphs in SDK or observability.

## Source Baseline

- AWS Textract:
  <https://docs.aws.amazon.com/textract/latest/dg/what-is.html>
- Azure AI Document Intelligence:
  <https://learn.microsoft.com/en-us/azure/ai-services/document-intelligence/>
- Google Document AI:
  <https://cloud.google.com/document-ai/docs>
- Apache Tika:
  <https://tika.apache.org/>
- Unstructured documentation:
  <https://docs.unstructured.io/>

## Supplier API Notes

- AWS Textract contributes text detection, AnalyzeDocument, block graphs, pages,
  lines, words, tables, forms, key-values, queries, signatures, selection marks,
  bounding boxes, confidence, and async jobs.
- Azure Document Intelligence contributes layout, paragraphs, tables, selection
  marks, key-value pairs, prebuilt/custom models, spans, bounding regions,
  confidence, and long-running operations.
- Google Document AI contributes processors, OCR, form/layout parsers, tables,
  entities, page/text anchors, shards, processor versions, and batch processing.
- Apache Tika contributes format detection, text extraction, metadata,
  embedded-resource discovery, parser configuration, and language metadata.
- Unstructured-style pipelines contribute partitioning, semantic elements,
  chunking, OCR/table inference, image extraction, and coordinates.

## Macaca-Owned Abstractions

`pack.knowledge.document.parsing.v1` should define `DocumentSource`,
`ParseJob`, `ParserProfile`, `DocumentPage`, `DocumentElement`,
`DocumentTextSpan`, `DocumentOcrToken`, `DocumentTable`,
`DocumentTableCell`, `DocumentFormField`, `DocumentEntity`,
`DocumentMetadata`, `DocumentEmbeddedResource`, `DocumentChunk`,
`DocumentGeometry`, `DocumentConfidence`, `DocumentParseResult`, and
`DocumentParserCapability`.

The DTOs must carry document handle validation, parser profile, page/element
order, geometry, confidence, OCR/layout/table/form/entity support, embedded
resource handles, chunk provenance, async job state, output limits, and replay.
Raw files, OCR images, provider block graphs, raw embedded resources, and
unbounded full text are rejected from trace/audit/snapshot output.

## Existing Macaca Platform Inventory

- Foundation/file and media/document handles can supply document references
  later; parsing must consume handles and emit canonical parse handles.
- Generic descriptors, SDK facade, service-call tracing, unavailable clients,
  policy command objects, scheduler/resource DTOs, and persistence snapshots can
  support async parse jobs and bounded replay diagnostics.
- Current evidence does not prove parsing-specific DTOs, malware/type gates,
  providers, SDK/WASM ABI, redaction tests, or documentation.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
