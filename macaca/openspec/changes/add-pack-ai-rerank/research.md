# AI Rerank Pack Research

## Purpose

This note records borrowed platform patterns, Macaca mapping, existing platform
inventory, and GitNexus memo evidence for `pack.ai.rerank.v1`. The pack must
provide query/candidate reranking and score explanation through provider-neutral
commands. It must not own search indexing, retrieval, ranking business policy,
or provider-native reranker payloads.

## Source Baseline

- Cohere Rerank API documentation:
  <https://docs.cohere.com/docs/reranking-with-cohere>
- Jina AI reranker documentation:
  <https://jina.ai/reranker/>
- Azure AI Search semantic ranking documentation:
  <https://learn.microsoft.com/en-us/azure/search/semantic-search-overview>
- Google Vertex AI Search ranking and grounding concepts:
  <https://cloud.google.com/generative-ai-app-builder/docs>
- OpenAI and general model-inference APIs inform shared usage, quota, policy,
  and audit behavior but are not copied as provider-native SDK contracts.

## Borrowed Platform Patterns

- Rerank APIs converge on query text, ordered candidate lists, top-n limits,
  scores, indexes, metadata, and optional explanations. Macaca should preserve
  candidate ids and deterministic tie-breakers for replay.
- Providers vary in score range, normalization, explanation availability, and
  maximum candidates. Macaca should expose provider capability and normalized
  score metadata rather than pretending scores are globally comparable.
- Reranking operates on retrieved candidates. Retrieval, search, citations, and
  document parsing remain separate packs/services.
- Explanations may reveal raw query/candidate text. Macaca should return
  bounded explanation references and redacted score diagnostics.
- Batch rerank requires per-query and per-candidate partial-failure mapping.

## Macaca Mapping

- Descriptor: `pack.ai.rerank.v1`, command namespace `rerank.*`, scopes
  `ai.rerank.invoke` and `ai.rerank.explain`.
- Commands: `rerank.rerank`, `rerank.batch_rerank`,
  `rerank.explain_scores`, and `rerank.inspect_model`.
- DTOs: `RerankRequest`, `RerankQuery`, `RerankCandidate`, `RerankResult`,
  `RerankExplanation`, `RerankBatchResult`, and `RerankEvalMetadata`.
- Policy: validate candidate count, top-n bounds, duplicate ids, hidden
  candidates, query/candidate sensitivity, explanation scope, resource budget,
  entitlement, and provider capability before dispatch.
- Trace/audit: record query hash, candidate ids/hashes, top-n, score schema,
  tie-breaker policy, provider class, latency, and bounded errors only.

## Existing Macaca Platform Inventory

- `macaca-memory` query pipeline already separates keyword/vector retrieval from
  embedding provider diagnostics. That boundary is useful because rerank must
  remain post-retrieval scoring, not retrieval ownership.
- No dedicated rerank service, DTO, SDK helper, or provider exists in current
  evidence. Implementation tasks remain unchecked.
- Generic service descriptors, `SystemFacade`, service-call trace middleware,
  unavailable clients, persistence snapshots, and policy command objects provide
  reusable infrastructure patterns for a future rerank provider.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
