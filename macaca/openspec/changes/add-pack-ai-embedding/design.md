# AI Embedding Pack Design

## Context

`pack.ai.embedding.v1` is a child proposal of the developer-pack industrial capability catalog. It makes text/image embedding, batch embedding, vector metadata, and model diagnostics available through Macaca's microkernel-compatible service model. The pack must behave like an operating-system capability: declarative in application manifests, policy checked before use, provider-neutral at the SDK boundary, observable through trace/audit evidence, and replaceable by built-in, plugin, remote, mock, or unavailable providers.

## Research Synthesis

Mature platforms converge on the same pattern:

- Platform privacy models: raw user data must not enter logs or diagnostics when processed by intelligent services.
- Android runtime permission pattern: AI operations that touch private data inherit source permissions.
- Windows capability declaration pattern: model-backed operations must be visible in app capability metadata.
- Apple entitlement/privacy pattern: sensitive processing requires policy and developer-declared purpose.

Macaca should adapt those ideas as pack descriptors, service commands, policy decorators, and replayable audit records. The design does not copy platform API names; it preserves Macaca's own microkernel boundary and service runtime execution path.

## Goals

- Provide text/image embedding, batch embedding, vector metadata, and model diagnostics.
- Expose stable pack id `pack.ai.embedding.v1`, command namespace `embedding.*`, permission scopes, SDK metadata, health, snapshot, and unavailable diagnostics.
- Route every operation through `SystemFacade` or focused SDK clients into the canonical service runtime path.
- Return structured `unavailable`, `unsupported`, `denied`, `conflict`, `quota_exceeded`, and `failure` results.
- Emit sanitized trace/audit events for declaration, admission, policy, entitlement/resource decisions, service calls, provider health, snapshots, and replay.

## Non-Goals

- Do not implement an application-specific feature, workflow, UI, or business rule.
- Do not put concrete provider construction in the microkernel, SDK, shells, or generic application framework.
- Do not expose raw secrets, prompts, manifests, package bytes, credentials, raw provider payloads, private keys, signatures, or unbounded output in logs, traces, snapshots, SDK diagnostics, or examples.
- Do not silently degrade to a different provider, fake success, or bypass policy when the declared provider is absent.

## Ownership And Boundaries

- Pack id: `pack.ai.embedding.v1`.
- Family: `ai`.
- Backing service owner: embedding service provider.
- SDK surface: `sdk.packs.ai.embedding`.
- Command namespace: `embedding.*`.
- Microkernel ownership: identity, service-call evidence, policy facade, trace/audit primitives only.
- Application framework ownership: manifest declaration, app-scoped permission declarations, lifecycle/effective-capability projection.
- Runtime-host ownership: provider registration and decorators only through approved composition roots.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `embedding.embed_text` | Typed command/result DTO for embed text | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `embedding.embed_image` | Typed command/result DTO for embed image | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `embedding.batch_embed` | Typed command/result DTO for batch embed | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `embedding.inspect_vector_schema` | Typed command/result DTO for inspect vector schema | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `embedding.estimate_cost` | Typed command/result DTO for estimate cost | Requires trace, policy decision, structured result, and sanitized audit evidence |

Every command must define a typed command DTO, typed success result, typed partial-result shape when streaming or asynchronous, typed error result, redaction policy, idempotency semantics where side effects exist, and replay metadata.

## Permission, Policy, Resource, Entitlement, And Approval Model

Initial permission scopes:

- `ai.embedding.invoke`
- `ai.embedding.batch`

Policy defaults:

- Scope every command to application id, tenant id, session id, task id, and trace id when available.
- Apply model/provider neutrality, budget/rate policy, prompt/output redaction, evaluation trace, and no model-name routing in OS code.
- Require explicit approval for commands that cross user-sensitive, financial, identity, device, host, network, external-recipient, or irreversible side-effect boundaries.
- Enforce resource budgets for time, memory, storage, network, provider quota, streaming output, and retained snapshots.
- Return `denied` for policy rejection, `unavailable` for absent providers or entitlements, `unsupported` for unknown commands, and `quota_exceeded` for bounded-resource rejection.

## SDK Discovery And Examples

SDK discovery must return pack metadata, lifecycle, service mappings, command schemas, permission scopes, policy templates, examples, availability, health, provider class, version compatibility, and sanitized diagnostics.

- SDK helper example: `sdk.packs.ai.embedding.embed_text(command)` builds a canonical traced service call; it never constructs providers.
- SDK helper example: `sdk.packs.ai.embedding.embed_image(command)` builds a canonical traced service call; it never constructs providers.
- SDK helper example: `sdk.packs.ai.embedding.batch_embed(command)` builds a canonical traced service call; it never constructs providers.

Examples must use generic handles and synthetic data. They must not bake in application names, provider names, credentials, business workflows, or domain-specific routing.

## Trace, Audit, Health, Snapshot, And Replay

Required event names:

- `embedding_pack_declared`
- `embedding_pack_admission_validated`
- `embedding_pack_policy_decision`
- `embedding_pack_service_call_requested`
- `embedding_pack_service_call_succeeded`
- `embedding_pack_service_call_failed`
- `embedding_pack_unavailable`
- `embedding_pack_snapshot_recorded`

Events include pack id, service id, command name, trace id, application/session/task/tenant identifiers when available, policy decision, provider class, latency, bounded resource counters, stable capability hash, and bounded error code. Snapshots include descriptor version, provider health, command availability, policy template hash, and sanitized replay pointers.

## Implementation Slices

1. Descriptor and contract slice: pack descriptor, command schemas, permissions, policy template, health/snapshot DTOs, unavailable diagnostics.
2. Admission and resolver slice: required/optional declaration handling, lifecycle checks, service mapping checks, permission validation, effective capability memento.
3. Service slice: embedding service provider command handlers or unavailable provider, lifecycle, health, snapshot, shutdown, and structured error behavior.
4. SDK slice: discovery APIs, typed command helper builders, examples, diagnostics, and Null Object behavior.
5. Observability slice: trace/audit events, replay tests, snapshot sanitization, and metrics.
6. Gates slice: OpenSpec validation, DTO compatibility, dependency-boundary tests, no-direct-provider-call tests, canonical execution-path tests, file-size gates.

## Design Patterns

- **Facade**: SDK exposes discovery and command builders; it does not construct providers.
- **Command**: every operation is a typed command/result DTO.
- **Strategy**: provider selection, unavailable behavior, policy routing, and version compatibility are replaceable.
- **Decorator**: trace, policy, resource, entitlement, approval, metering, and redaction wrap every call.
- **Specification**: admission validates pack id, lifecycle, commands, permissions, policy, and service mapping.
- **Observer**: trace, audit, health, and service events are subscribable and replayable.
- **Memento**: effective capability reports and snapshots preserve bounded recovery state.
- **Abstract Factory**: optional providers register only through approved composition roots.

## Risks And Mitigations

- Risk: broad capability becomes an OS-layer business workflow. Mitigation: keep the pack contract generic and place domain/provider semantics in replaceable services.
- Risk: SDK helpers become a second execution path. Mitigation: helpers only build canonical service-call commands and are covered by no-direct-provider-call gates.
- Risk: preview or unavailable providers look callable. Mitigation: availability validators require descriptor, service registration, command schema, permission, entitlement, and health evidence before callable state.
- Risk: observability leaks sensitive data. Mitigation: event schema permits identifiers, hashes, counters, bounded codes, and sanitized snippets only.

## Supplier Capability Matrix

| Supplier pattern | Macaca contract element |
| --- | --- |
| Batch embeddings | `EmbeddingBatchRequest`, item ids, ordered result mapping |
| Dimension selection | `VectorSchemaRef`, dimension count, metric compatibility |
| Text/image modality | `EmbeddingInput`, modality, content reference, redaction profile |
| Truncation policy | `EmbeddingTruncationPolicy`, token/byte/image bounds |
| Vector-store compatibility | schema hash, index compatibility report |

## Domain Model

- `EmbeddingInput`: text, image, or content reference with modality, purpose,
  hash, size counters, and redaction profile.
- `EmbeddingVector`: vector reference or bounded vector payload, dimension,
  numeric type, normalization state, schema id, and item id.
- `EmbeddingBatchResult`: ordered item results with per-item success, denied,
  unsupported, truncation, quota, or failure states.
- `VectorSchemaDescriptor`: dimension, metric family, normalization, numeric
  type, compatibility version, and storage recommendations.

## Additional Industrial Commands

- `embedding.validate_schema`: verify vector dimension and metric compatibility
  before handing vectors to retrieval/vector-store services.
- `embedding.normalize_vector`: optional provider-neutral normalization command
  when supported by policy.
- `embedding.inspect_batch`: query asynchronous batch status with policy-filtered
  item diagnostics.

## Embedding-Specific Risks

- Risk: embedding pack becomes vector storage. Mitigation: it emits vectors and
  schema metadata; indexing, retrieval, and graph persistence remain separate
  services.
- Risk: raw input leaks in replay. Mitigation: traces store hashes, lengths,
  modality, schema ids, and usage counters only.
- Risk: dimension mismatch corrupts downstream indexes. Mitigation: schema
  compatibility checks are required before vector-store ingestion examples are
  marked valid.
