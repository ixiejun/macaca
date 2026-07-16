# Knowledge Summarization Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
explicit non-goals, existing platform inventory, and GitNexus memo evidence for
`pack.knowledge.summarization.v1`. Summarization must produce extractive,
abstractive, hybrid, multi-document, conversation, and context-compression
summaries through source handles, evidence policy, quality diagnostics, and
traceable service commands. It must not expose provider prompts, raw source
documents, or application-specific summary templates.

## Source Baseline

- OpenAI evaluation, Responses, structured-output, streaming, and prompting
  documentation: <https://platform.openai.com/docs>
- Anthropic context engineering and prompt engineering guidance:
  <https://docs.anthropic.com/>
- Azure AI Language summarization:
  <https://learn.microsoft.com/en-us/azure/ai-services/language-service/summarization/overview>
- Google Vertex AI generative AI summarization examples and long-document
  patterns: <https://cloud.google.com/vertex-ai/generative-ai/docs>
- Amazon Comprehend entities, key phrases, sentiment, topics, and events:
  <https://docs.aws.amazon.com/comprehend/latest/dg/what-is.html>

## Supplier API Notes

- OpenAI-style LLM APIs contribute model-based abstractive summarization,
  structured output constraints, streaming, usage metadata, and safety policy.
  Macaca should expose summary mode, output schema, usage, and quality metadata
  without provider prompt pass-through.
- Anthropic context engineering contributes compaction, preserving decisions,
  unresolved work, constraints, and recovery-critical state. Macaca should model
  `CompressionMap`, retained facts, dropped span hashes, unresolved items, and
  recovery notes.
- Azure AI Language contributes extractive and abstractive summarization actions,
  input limits, result metadata, and long-running action behavior. Macaca should
  expose mode-specific capability and async/partial result semantics.
- Vertex AI long-document patterns contribute stuffing, chunking, MapReduce,
  overlapping chunks, rolling summaries, and synthesis. Macaca should model
  summary plans, chunk strategies, partial summaries, and final synthesis.
- Amazon Comprehend contributes entities, key phrases, events, sentiment, and
  topics as extractive/hybrid summary signals, not as final summary ownership.

## Macaca-Owned Abstractions

`pack.knowledge.summarization.v1` should define `SummarySource`,
`SummaryRequest`, `SummaryPlan`, `SummaryOutput`, `SummaryClaim`,
`SummaryEvidenceLink`, `CompressionMap`, `SummaryComparisonReport`,
`SummaryQualityReport`, and `SummaryProviderCapability`.

The DTOs must carry source handles, mode, target length, output schema,
language, redaction, evidence/citation policy, chunking/synthesis strategy,
quality thresholds, compression maps, comparison/evaluation metadata, provider
capability, and replay. Raw prompts, source documents, private spans, model
outputs, provider payloads, and unbounded summaries are rejected.

## Explicit Non-Goals

- Do not implement concrete model/cloud adapters in the research phase.
- Do not define legal brief, meeting note, support ticket, CRM, sales, or other
  application-specific summary templates in OS layers.
- Do not silently fall back to an uncited, lower-evidence, shorter, or different
  provider summary when requested capability is unavailable.
- Do not replace retrieval, citations, document parsing, graph, LLM, or model
  evaluation packs; summarization consumes their declared handles.

## Existing Macaca Platform Inventory

- Existing LLM service surfaces, context-window behavior, and agentic-loop
  context patterns can inform summarization and compression, but they do not
  complete summarization DTOs or provider contracts.
- Generic descriptors, SDK facade, trace-required service calls, unavailable
  clients, scheduler/resource DTOs, persistence snapshots, and policy command
  objects provide reusable service infrastructure.
- Citation, retrieval, document parsing, graph, and model evaluation proposals
  provide adjacent declared capabilities for evidence, source, and quality
  hooks.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
