# Change: Add AI Rerank Pack

## Why

Developers need `pack.ai.rerank.v1` as a real industrial capability for candidate reranking, score explanation, batch ranking, and evaluation metadata. The pack must not be a catalog label only: if an application declares it and the provider is installed, the SDK must expose callable typed commands; if policy, entitlement, provider, or host support is absent, Macaca must return explicit unavailable or denied diagnostics.

This child proposal intentionally narrows the broad industrial catalog into one implementable service-backed pack. It records the research basis, capability boundary, command surface, permission model, observability model, and acceptance gates needed before this pack can be called production-ready.

## Research And Borrowed Platform Patterns

The proposal borrows stable ideas from mature application platforms without copying their product-specific APIs:

- Platform privacy models: raw user data must not enter logs or diagnostics when processed by intelligent services.
- Android runtime permission pattern: AI operations that touch private data inherit source permissions.
- Windows capability declaration pattern: model-backed operations must be visible in app capability metadata.
- Apple entitlement/privacy pattern: sensitive processing requires policy and developer-declared purpose.

The Macaca interpretation is: app manifests declare pack intent; admission validates capability and permission metadata; runtime permission and approval gates happen before sensitive side effects; concrete providers stay behind service descriptors; traces and audits prove the execution path.

## What Changes

- Add the provider-neutral `pack.ai.rerank.v1` contract under the `ai` family.
- Define a concrete command namespace, initial industrial command set, result/error DTO requirements, permission scopes, policy defaults, resource/entitlement behavior, and unavailable diagnostics.
- Bind implementation ownership to rerank service provider; the kernel, SDK, shells, and generic application framework remain provider-neutral.
- Add SDK discovery metadata and examples for rerank, batch rerank, explain scores, inspect model.
- Add trace, audit, health, snapshot, replay, and boundary gates proving the pack uses the canonical service path.

## Impact

- Affected specs: `pack-ai-rerank`, `developer-pack-industrial-capability-catalog`, `sdk-system-facade`, `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral DTOs in protocol crates, descriptor/admission validators, SDK discovery/command helpers, rerank service provider, optional provider registration, trace/audit tests, and dependency-boundary gates.
- Non-goals: no application-specific workflow, no provider-name routing in OS layers, no concrete provider construction in SDK/shell/kernel, and no fake success for absent providers.

## Supplier/API Baseline To Match

- Cohere Rerank style APIs: query plus candidate documents, ranked results,
  relevance scores, top-n limits, and usage metadata.
- Voyage/Vertex/Search reranking APIs: batch ranking, candidate ids, score
  normalization, truncation policy, and model diagnostics.
- Search engine learning-to-rank systems: stable candidate ids, tie-breaking,
  feature/score explanations, offline evaluation metadata, and deterministic
  replay.

The rerank pack does not retrieve documents and does not decide application
business relevance. It reorders caller-provided candidates under explicit
policy, budget, redaction, and evaluation constraints.

## Industrial Acceptance Bar

This proposal is not complete until implementation tasks produce:

- a developer guide at `docs/developer-packs/ai/rerank.md`;
- typed query, candidate, ranking, score, explanation, truncation, batch, and
  evaluation metadata DTOs;
- deterministic tests for candidate-id stability, top-n bounds, score ordering,
  tie breaking, truncation, explanation redaction, and provider absence;
- audit replay proving raw candidate content is not logged while ranking order,
  score metadata, policy, and usage remain inspectable.
