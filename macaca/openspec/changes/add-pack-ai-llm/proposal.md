# Change: Add AI LLM Pack

## Why

Developers need `pack.ai.llm.v1` as a real industrial capability for chat, completion, routing, policy, budget, tool-call metadata, and model invocation diagnostics. The pack must not be a catalog label only: if an application declares it and the provider is installed, the SDK must expose callable typed commands; if policy, entitlement, provider, or host support is absent, Macaca must return explicit unavailable or denied diagnostics.

This child proposal intentionally narrows the broad industrial catalog into one implementable service-backed pack. It records the research basis, capability boundary, command surface, permission model, observability model, and acceptance gates needed before this pack can be called production-ready.

## Research And Borrowed Platform Patterns

The proposal borrows stable ideas from mature application platforms without copying their product-specific APIs:

- Platform privacy models: raw user data must not enter logs or diagnostics when processed by intelligent services.
- Android runtime permission pattern: AI operations that touch private data inherit source permissions.
- Windows capability declaration pattern: model-backed operations must be visible in app capability metadata.
- Apple entitlement/privacy pattern: sensitive processing requires policy and developer-declared purpose.

The Macaca interpretation is: app manifests declare pack intent; admission validates capability and permission metadata; runtime permission and approval gates happen before sensitive side effects; concrete providers stay behind service descriptors; traces and audits prove the execution path.

## What Changes

- Add the provider-neutral `pack.ai.llm.v1` contract under the `ai` family.
- Define a concrete command namespace, initial industrial command set, result/error DTO requirements, permission scopes, policy defaults, resource/entitlement behavior, and unavailable diagnostics.
- Bind implementation ownership to LLM service provider; the kernel, SDK, shells, and generic application framework remain provider-neutral.
- Add SDK discovery metadata and examples for chat, complete, route model, estimate tokens, inspect budget.
- Add trace, audit, health, snapshot, replay, and boundary gates proving the pack uses the canonical service path.

## Impact

- Affected specs: `pack-ai-llm`, `developer-pack-industrial-capability-catalog`, `sdk-system-facade`, `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral DTOs in protocol crates, descriptor/admission validators, SDK discovery/command helpers, LLM service provider, optional provider registration, trace/audit tests, and dependency-boundary gates.
- Non-goals: no application-specific workflow, no provider-name routing in OS layers, no concrete provider construction in SDK/shell/kernel, and no fake success for absent providers.

## Supplier/API Baseline To Match

- OpenAI Responses/Chat style APIs: message arrays, structured output, tool-call
  envelopes, streaming deltas, usage accounting, and cancellation.
- Anthropic Messages style APIs: system/user/assistant content blocks,
  tool-use/tool-result blocks, stop reasons, and token accounting.
- Google Gemini generateContent style APIs: multimodal parts, safety settings,
  candidates, finish reasons, and usage metadata.
- AWS Bedrock Converse style APIs: provider-neutral model invocation, guardrail
  metadata, tool configuration, streaming, and traceable usage.

Macaca's contract must expose the common denominator without hardcoding provider
or model names in OS layers. Provider adapters can map the canonical DTOs to
their native APIs behind service descriptors.

## Industrial Acceptance Bar

This proposal is not complete until implementation tasks produce:

- a developer guide at `docs/developer-packs/ai/llm.md`;
- typed message, content block, generation option, tool-call envelope,
  structured-output schema, stream-delta, usage, budget, and cancellation DTOs;
- deterministic tests for streaming order, cancellation, tool-call validation,
  structured-output schema mismatch, budget exhaustion, and redacted replay;
- audit replay proving prompts and raw provider payloads are never logged while
  usage, policy decisions, and model capability metadata remain inspectable.
