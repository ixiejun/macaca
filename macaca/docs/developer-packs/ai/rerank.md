# AI Rerank Pack

`pack.ai.rerank.v1` provides provider-neutral reranking, batch reranking,
score explanation, and model capability inspection. The pack is
descriptor-only until a serviceized rerank provider is registered.

Applications submit query and candidate references with stable ids; they do not
place raw query text, raw candidate bodies, concrete ranker names, or provider
payloads in OS-layer diagnostics.

## Manifest Declaration

```toml
[service_contract]
optional_packs = ["pack.ai.rerank.v1"]
```

Optional declarations degrade with `ai_rerank_provider_not_installed`.

## Permission Scopes

- `ai.rerank.invoke`: rerank and batch rerank.
- `ai.rerank.explain`: score explanation and inspect-model metadata.

## Commands

- `rerank.rerank`: ranks candidates for one query.
- `rerank.batch_rerank`: ranks candidates for multiple query refs.
- `rerank.explain_scores`: returns redacted explanation refs.
- `rerank.inspect_model`: reports provider-neutral capability metadata.

## DTOs And Results

Core DTOs include `RerankRequest`, `RerankQuery`, `RerankCandidate`,
`RerankResult`, `RerankExplanation`, `RerankBatchResult`, and
`RerankEvalMetadata`. Statuses include success, partial, denied, unavailable,
unsupported, conflict, quota exceeded, duplicate candidate, candidate limit
exceeded, explanation unavailable, and provider failure.

## Examples

Minimal declaration:

```toml
[service_contract]
optional_packs = ["pack.ai.rerank.v1"]
```

Unavailable diagnostic:

```json
{
  "pack_id": "pack.ai.rerank.v1",
  "required": false,
  "reason_code": "optional_pack_unresolved",
  "message": "ai_rerank_provider_not_installed"
}
```

Canonical command payload:

```json
{
  "subject_ref": "query:ref",
  "parameters": {
    "request_ref": "rerank-request-ref",
    "candidate_set_ref": "candidate-set-ref",
    "top_n": "10"
  },
  "idempotency_key": "rerank-request-key"
}
```

## Trace And Audit

Trace evidence may include stable query refs, candidate refs, rank order,
normalized score counters, explanation refs, tie-breaker labels, and bounded
error codes. It must not include raw queries, candidate text, credentials,
concrete provider payloads, or unbounded explanations.

## Provider Replacement

Provider classes include `hosted-model`, `local-runtime`, `plugin`, `mock`, and
`unavailable`. Provider selection and score generation remain service-owned and
decorated by policy, trace, resource, entitlement, and redaction gates.
