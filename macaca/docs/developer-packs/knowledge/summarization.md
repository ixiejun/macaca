# Knowledge Summarization Pack

`pack.knowledge.summarization.v1` describes planning, validation, extractive
summary, abstractive summary, cited summary, multi-source summary, conversation
summary, context compression, refinement, comparison, evaluation, evidence
inspection, and provider inspection.

## Manifest Declaration

```toml
[service_contract]
optional_packs = ["pack.knowledge.summarization.v1"]
```

Use required declarations only when summarization is mandatory. Optional
declarations degrade with explicit effective capability mementos.

## Permissions

Scopes are `summarization.plan`, `summarization.run`,
`summarization.citations`, `summarization.context.compress`,
`summarization.conversation`, `summarization.refine`,
`summarization.compare`, `summarization.evaluate`,
`summarization.evidence.read`, and `summarization.provider.inspect`.

## Capability Model

DTOs include source handles, requests, plans, outputs, claims, evidence links,
compression maps, comparison reports, quality reports, and provider capability.
Raw source documents, prompts, model outputs, private spans, conversations,
credentials, provider payloads, and unbounded summaries remain outside traces,
audits, snapshots, SDK diagnostics, and examples.

## Commands

Commands cover planning, validation, summarize, summarize with citations,
summarize many, summarize conversation, compress context, refine, compare,
evaluate, inspect evidence, and inspect provider. SDK command helpers produce
canonical traced service calls and never construct providers, prompts, or model
clients.

Command DTOs use `KnowledgeCommandEnvelope`:

- `subject_ref`: source, summary, conversation, compression map, comparison, or
  evidence handle.
- `parameters`: bounded options such as mode, target length, language,
  evidence policy, output schema, and evaluation profile.
- `cursor` and `page_size`: pagination controls for partial or streaming
  metadata.
- `idempotency_key`: retry-safe key for long-running summary jobs.

Result DTOs carry a `SummarizationResultStatus`, optional data, optional page,
and optional `KnowledgeError`. Status values distinguish success, streaming,
paged, partial, validation issue, denied, unavailable, unsupported, conflict,
quota, timeout, cancellation, and failure.

## App-Facing Examples

- Plan a summary from document, retrieval, citation, graph, transcript, message,
  or prior-summary handles.
- Produce extractive or abstractive summaries using source handles only.
- Generate cited summaries when citation support is declared and available.
- Summarize many sources with bounded chunk plans and partial result metadata.
- Summarize a conversation without exposing private message text to traces.
- Compress context with a recoverable compression map.
- Refine an existing summary through a summary handle and revision hash.
- Compare summaries and evaluate quality with trace-safe report handles.
- Inspect evidence links before presenting claims.
- Handle unavailable, missing source permission, missing citation support, and
  quota-denied diagnostics without provider names, credentials,
  application-specific workflows, private documents, private conversation text,
  source spans, raw model outputs, or domain-specific templates.

## Supplier Mapping

Model-based summarization APIs map to summary request, plan, output, claim,
evidence, streaming, quality, and provider capability DTOs. Context
engineering guidance maps to compression maps and recovery-safe retained
references. Extractive and abstractive language APIs map to mode-specific
provider capability metadata. Entity, key phrase, event, sentiment, and topic
signals map to optional quality or evidence metadata. Raw prompts and model
payloads are not exposed as Macaca OS semantics.

## Trace And Audit

Trace metadata should include request id, source count, source kinds, mode,
target length, evidence policy, provider class, capability hash, output handle,
and result status. Raw credentials, source documents, prompts, model outputs,
provider payloads, private spans, private conversation text, and unbounded
summaries are forbidden in observability.

## Provider Authors

Descriptors must report modes, source kinds, languages, output formats,
citation support, context-compression support, evaluation support, streaming,
quota, resource bounds, evidence policy, quality diagnostics, snapshot/replay,
unavailable behavior, and redaction. Long-document and context-compression
strategies must be resumable and audit-friendly.

## Conformance Checklist

- Descriptor metadata includes modes, source kinds, languages, output formats,
  permissions, policy templates, diagnostics, compatibility, and redaction
  profile.
- Command DTOs stay provider-neutral and use handles for sources, summaries,
  evidence, compression maps, comparison reports, and quality reports.
- Planning and validation enforce source access, target length, output schema,
  language, evidence, quote, freshness, sensitivity, and resource policy before
  provider execution.
- Long-document synthesis and context compression expose bounded partial
  results, cancellation, resumable checkpoints, and trace-safe snapshots.
- Trace, audit, SDK diagnostics, and examples exclude raw prompts, source
  documents, model outputs, private spans, conversations, and provider payloads.
