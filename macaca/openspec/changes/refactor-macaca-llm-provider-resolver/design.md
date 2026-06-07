## Context

`macaca-llm` is a stage-1 infrastructure crate in `macaca/docs/design-pattern-refactor-plans/refactor-order.md`. It is consumed by task, agent, kernel, runtime, framework, web, CLI, and integration tests. Its provider/model routing behavior therefore needs small, reversible changes with strong compatibility.

Current `LlmRouter::resolve_provider_name` uses an ordered hardcoded branch for known model families. That behavior is correct enough to preserve, but the implementation does not scale as provider support grows.

## Goals

- Preserve current provider inference behavior 1:1.
- Move model-prefix provider inference into a dedicated primitive.
- Make provider inference extensible through Chain of Responsibility and Strategy.
- Keep old API callable and deprecated for migration discovery.
- Keep this slice inside `macaca-llm`.

## Non-Goals

- Do not remove `LlmRouter::resolve_provider_name`.
- Do not change `LlmProvider`.
- Do not change `LlmConfig`.
- Do not change provider request/response wire formats.
- Do not split `ResilientLlmWrapper` in this change.
- Do not migrate `macaca-framework` or `macaca-web`.
- Do not introduce new external dependencies.

## Decisions

- Add `resolver.rs` with `ProviderResolver`, `PrefixProviderResolver`, and `ResolverChain`.
- Use the existing routing order as the default resolver chain:
  - slash-containing model references route to `openrouter`
  - GPT and OpenAI o-series route to `openai`
  - Claude models route to `anthropic`
  - Qwen models route to `dashscope`
  - DeepSeek models route to `deepseek`
  - MiniMax models route to `minimax`
  - unknown models route to the model string itself
- Keep `LlmRouter::resolve_provider_name` as a deprecated compatibility helper that delegates to the default resolver chain.
- Keep `LlmRouter::new` behavior unchanged for callers by installing the default resolver chain internally.

## Alternatives Considered

- Refactor provider factory first: rejected for this slice because provider construction, base URL normalization, and wrapper composition are separate responsibilities.
- Split `ResilientLlmWrapper` first: rejected for this slice because retry/fallback/rate-limit/cost ordering is more behaviorally sensitive than prefix routing.
- Complete all macaca-llm planned slices in one change: rejected because it would violate the small reversible change rule.

## Risks / Trade-offs

- `resolve_target` has CRITICAL impact in GitNexus because it flows into web and CLI model execution paths.
  - Mitigation: keep behavior identical and run router tests plus `cargo check`.
- Deprecated compatibility can hide old call sites if warnings are ignored.
  - Mitigation: only use deprecated helper in compatibility tests under local `#[allow(deprecated)]`; migrated production code should call resolver primitives or `resolve_target`.
- New resolver API could be overdesigned.
  - Mitigation: keep the first abstraction minimal: input model string, output provider id.

## Migration Plan

1. Add OpenSpec proposal/design/tasks/spec.
2. Add resolver primitives with table-driven tests matching current router behavior.
3. Wire `LlmRouter::resolve_target` to the default resolver chain.
4. Mark `LlmRouter::resolve_provider_name` deprecated and keep a compatibility test.
5. Validate behavior with focused tests, crate check, OpenSpec validation, and GitNexus change detection.
