# Change: Add Knowledge Summarization Pack

## Why

Developers need `pack.knowledge.summarization.v1` as an industrial
summarization capability, not a thin wrapper around one model prompt. Real
applications need extractive summaries, abstractive summaries, multi-document
synthesis, conversation and meeting summaries, context compression, claim and
evidence tracking, summary comparison, quality signals, and policy-aware
redaction. They also need predictable behavior when a provider, model, document
permission, citation pack, budget, or host capability is absent.

Summarization is especially risky for an Agent OS because it can silently lose
context, invent unsupported claims, leak sensitive source text, or compress away
audit-critical details. Macaca must therefore expose a provider-neutral
serviceized pack with typed inputs, typed outputs, source constraints, evidence
links, quality metadata, resource limits, trace/audit events, replay, and
developer documentation.

## Research And Supplier/API Baseline

Official supplier and platform references considered for this pack:

- OpenAI API docs describe GPT models as usable for summarization and recommend
  model prompting through the Responses API and prompt-engineering guidance.
  References: https://developers.openai.com/api/docs/concepts and
  https://developers.openai.com/api/docs/guides/prompt-engineering
- Anthropic engineering notes describe summarizing and compressing message
  history while preserving critical architectural decisions and unresolved work.
  Reference:
  https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents
- Azure AI Language Summarization documents extractive summarization and
  abstractive summarization, including the distinction between selecting
  sentences and generating new paraphrased text. References:
  https://learn.microsoft.com/en-us/azure/ai-services/language-service/summarization/overview
  and
  https://learn.microsoft.com/en-us/azure/ai-services/language-service/summarization/how-to/text-summarization
- Azure SDK docs for `AbstractiveSummaryAction` state that abstractive
  summarization generates summary sentences rather than concatenating extracted
  source sentences. Reference:
  https://learn.microsoft.com/en-us/python/api/azure-ai-textanalytics/azure.ai.textanalytics.abstractivesummaryaction
- Google Cloud Vertex AI samples and codelabs cover generative summarization,
  long document summarization patterns, stuffing, MapReduce, overlapping chunks,
  and rolling summaries. References:
  https://docs.cloud.google.com/vertex-ai/docs/samples/aiplatform-sdk-summarization
  and https://codelabs.developers.google.com/text-summ-large-docs-stuffing
- Amazon Comprehend documentation covers NLP document insights such as entities,
  key phrases, events, sentiment, and topics, which are useful provider signals
  for extractive, evidence-aware, or hybrid summaries. References:
  https://docs.aws.amazon.com/comprehend/latest/dg/what-is.html and
  https://docs.aws.amazon.com/comprehend/latest/dg/concepts-insights.html

The Macaca abstraction must not clone any provider API. It maps supplier
capabilities into provider-neutral summarization strategies, source/evidence
contracts, quality diagnostics, redaction policy, and traceable service calls.

## What Changes

- Add provider-neutral `pack.knowledge.summarization.v1` under the `knowledge`
  family.
- Define command namespace `summarization.*` for:
  - summary job planning and validation
  - extractive summarization
  - abstractive summarization
  - hybrid summarization
  - multi-document synthesis
  - conversation/meeting summarization
  - context compression and rolling summaries
  - summary refinement
  - citation/evidence attachment
  - summary comparison
  - summary quality evaluation
  - evidence inspection
  - provider capability inspection
- Define DTOs for source handles, source spans, summary plans, summary outputs,
  evidence links, claims, compression maps, quality scores, refinement plans,
  compare reports, provider capabilities, and diagnostics.
- Define permission scopes, policy defaults, source access gates, resource
  budgets, approval rules, entitlement checks, structured unavailable behavior,
  SDK discovery, developer documentation, trace/audit events, snapshots, replay,
  and boundary gates.
- Require detailed developer documentation at
  `docs/developer-packs/knowledge/summarization.md` before implementation
  completion.

## Impact

- Affected specs: `pack-knowledge-summarization`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral protocol DTOs, pack descriptors,
  admission validators, SDK discovery and command builders, summarization
  service provider or unavailable provider, runtime-host provider adapters,
  trace/audit schemas, replay tests, dependency-boundary gates, and developer
  documentation.
- Non-goals: no concrete OpenAI, Anthropic, Azure, Google, AWS, or local model
  implementation in this proposal; no application-specific summary template or
  workflow; no provider-name routing in OS layers; no raw source text or prompts
  in observability; no SDK/shell/kernel provider construction; no fake success
  when provider, entitlement, permission, source access, or host support is
  absent.
