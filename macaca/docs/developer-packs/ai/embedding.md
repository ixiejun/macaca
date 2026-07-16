# AI Embedding Pack

`pack.ai.embedding.v1` provides provider-neutral text embedding, image
embedding, batch embedding, vector-schema inspection, and cost estimation. The
pack is descriptor-only until a serviceized embedding provider is registered.

Applications declare the pack; they do not log raw inputs, store raw vectors in
OS traces, or call concrete embedding providers directly.

## Manifest Declaration

```toml
[service_contract]
optional_packs = ["pack.ai.embedding.v1"]
```

Optional declarations degrade with `ai_embedding_provider_not_installed`.
Required declarations block readiness until a provider is installed and the
pack becomes callable through the canonical service path.

## Permission Scopes

- `ai.embedding.invoke`: single text or image embedding.
- `ai.embedding.batch`: bounded batch embedding with stable item ids.

## Commands

- `embedding.embed_text`: creates an embedding from a text reference.
- `embedding.embed_image`: creates an embedding from an image reference.
- `embedding.batch_embed`: embeds multiple stable input ids in deterministic
  order.
- `embedding.inspect_vector_schema`: returns dimension, numeric type, metric,
  and normalization metadata.
- `embedding.estimate_cost`: estimates bounded usage without provider calls.

## DTOs And Results

Core DTOs include `EmbeddingInput`, `EmbeddingBatchRequest`,
`EmbeddingVector`, `EmbeddingBatchResult`, `VectorSchemaDescriptor`, and
`EmbeddingUsage`. Result statuses cover success, partial, denied, unavailable,
unsupported, conflict, quota exceeded, dimension mismatch, unsupported
modality, oversized batch, and provider failure.

## Examples

Minimal declaration:

```toml
[service_contract]
optional_packs = ["pack.ai.embedding.v1"]
```

Provider-unavailable diagnostic:

```json
{
  "pack_id": "pack.ai.embedding.v1",
  "required": false,
  "reason_code": "optional_pack_unresolved",
  "message": "ai_embedding_provider_not_installed"
}
```

Canonical batch command payload:

```json
{
  "subject_ref": "collection:target",
  "parameters": {
    "batch_ref": "embedding-batch-ref",
    "schema_ref": "vector-schema-ref"
  },
  "idempotency_key": "embedding-batch-key"
}
```

## Trace And Audit

Trace evidence may include input refs, item ids, vector schema refs, dimensions,
numeric type, usage counters, capability hash, and status. It must not include
raw text, raw images, embedding vectors, concrete model names, credentials, or
provider payloads.

## Provider Replacement

Provider classes include `hosted-model`, `local-runtime`, `remote-service`,
`mock`, and `unavailable`. The SDK only discovers descriptors and builds
canonical traced service-call commands; runtime composition roots own adapters.
