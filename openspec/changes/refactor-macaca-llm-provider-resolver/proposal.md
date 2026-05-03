# Change: Refactor macaca-llm provider resolver

## Why

`macaca-llm::LlmRouter` currently owns provider registration, model selection, fallback route construction, provider construction, and hardcoded model-prefix provider resolution in one module. The prefix resolution branch is the first pressure point because every new model/provider family requires editing router internals.

This change introduces a small resolver primitive so provider inference becomes an explicit chain of strategies while preserving current routing behavior.

## What Changes

- Add a `ProviderResolver` abstraction and default resolver chain in `macaca-llm`.
- Move current built-in model prefix rules out of `LlmRouter` into resolver primitives.
- Route `LlmRouter::resolve_target` through the default resolver chain.
- Keep the existing provider-name inference API callable but mark it deprecated so future migrations can grep old usage.
- Preserve current model routing semantics and provider registration behavior.

## Impact

- Affected specs: `macaca-llm-provider-routing`
- Affected code: `macaca/crates/macaca-llm`
- Compatibility: existing `LlmRouter`, `resolve_target`, `resolve_selection`, and chat dispatch behavior remain available.
- Non-impact: no changes to provider wire protocols, `LlmProvider`, `LlmConfig`, framework/web bootstrapping, task scheduling, trace, session, driver, skill, MCP, or application-specific behavior.
