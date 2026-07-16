# Change: Add Industrial Knowledge Document Parsing Pack

## Why

Applications need document parsing as a reusable capability for extracting text,
layout, tables, forms, key-value pairs, OCR, metadata, structure, and canonical
chunks from diverse file formats. Industrial parsing must handle asynchronous
jobs, page geometry, confidence scores, language detection, handwriting,
selection marks, tables, forms, images, attachments, embedded files, malware
and size policy, redaction, provenance, and provider replacement.

Without this pack, applications will call provider-specific OCR/parsing APIs
directly, leak raw documents into logs, duplicate format-specific parsers, and
produce non-replayable chunks for retrieval or summarization.

## Supplier And Platform API Research

This proposal maps mature document parsing APIs into Macaca abstractions:

- AWS Textract exposes synchronous/asynchronous text detection and document
  analysis, block graphs, pages, lines, words, tables, forms, key-value pairs,
  queries, signatures, selection elements, bounding boxes, polygons, confidence,
  and job status. Macaca maps these to parse jobs, page blocks, layout elements,
  table/form DTOs, query extraction, geometry, confidence, and async result
  handles.
- Azure AI Document Intelligence exposes layout extraction, paragraphs, tables,
  selection marks, key-value pairs, prebuilt models, custom extraction models,
  confidence, spans, bounding regions, and long-running operations. Macaca maps
  these to processor profiles, structure spans, table/selection DTOs, model
  capability descriptors, async jobs, and confidence metadata.
- Google Cloud Document AI exposes processors, OCR, form parser, layout parser,
  tables, entities, page anchors, text anchors, shards, processor versions, and
  batch processing. Macaca maps these to parser profiles, entities, anchors,
  document shards, processor-version compatibility, and batch parse jobs.
- Apache Tika detects file types, extracts text and metadata from many formats,
  supports embedded resources, parser configuration, language detection, and
  structured metadata. Macaca maps these to format detection, metadata extraction,
  embedded file handling, parser profiles, and safe fallback diagnostics.
- Unstructured and similar parsing pipelines expose partitioning strategies,
  semantic elements, chunking, OCR modes, table inference, image extraction, and
  element coordinates. Macaca maps these to semantic elements, chunking policy,
  OCR strategy, table inference capability, and element provenance.

The Macaca contract is provider-neutral. Provider-specific model ids, feature
flags, and output shapes remain bounded adapter metadata and are not OS-layer
routing branches.

## What Changes

- Add provider-neutral `pack.knowledge.document.parsing.v1` under the
  `knowledge` family.
- Define DTOs for document sources, parse jobs, parser profiles, pages, blocks,
  spans, layout elements, OCR tokens, tables, cells, forms, key-values,
  selection marks, entities, images, attachments, metadata, chunks, geometry,
  confidence, language, provenance, and provider capabilities.
- Define commands for detect format, parse document, start/get/cancel parse job,
  extract text, extract layout, extract tables, extract forms, extract metadata,
  convert to canonical document, chunk document, inspect parser capability, and
  validate document.
- Define permission scopes for parse, text extraction, layout/table/form
  extraction, metadata read, conversion/chunking, OCR, embedded-resource access,
  and parser administration.
- Require malware/size/type validation, redaction, bounded outputs,
  replayable provenance, async job snapshots, unavailable diagnostics, and a
  detailed developer guide.

## Impact

- Affected specs: `pack-knowledge-document-parsing`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected future code: provider-neutral proto DTOs, parsing descriptors,
  admission validators, SDK discovery metadata, focused SDK clients, document
  parsing service providers, unavailable/mock providers, trace/audit schemas,
  conformance fixtures, replay tests, and dependency-boundary gates.
- Non-goals: no application-specific document workflow, no provider-name routing
  in OS layers, no raw document/provider payload exposure, no concrete provider
  construction in kernel/SDK/shells, and no fake success when parsing providers
  or document formats are unsupported.

## References

- AWS Textract: https://docs.aws.amazon.com/textract/
- AWS Textract AnalyzeDocument:
  https://docs.aws.amazon.com/textract/latest/dg/API_AnalyzeDocument.html
- Azure AI Document Intelligence:
  https://learn.microsoft.com/en-us/azure/ai-services/document-intelligence/
- Azure layout model:
  https://learn.microsoft.com/en-us/azure/ai-services/document-intelligence/prebuilt/layout
- Google Document AI:
  https://cloud.google.com/document-ai/docs
- Google Document AI processors:
  https://cloud.google.com/document-ai/docs/processors-list
- Apache Tika: https://tika.apache.org/
- Apache Tika parser docs: https://tika.apache.org/2.9.0/parser.html
- Unstructured partitioning:
  https://docs.unstructured.io/open-source/core-functionality/partitioning
