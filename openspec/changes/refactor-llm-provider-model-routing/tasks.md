## 1. OpenSpec and Prerequisite

- [x] 1.1 Complete bottom-level resolver prerequisite in `refactor-macaca-llm-provider-resolver`.
- [x] 1.2 Keep `LlmProvider` as the execution trait; do not deprecate it.
- [x] 1.3 Update this design/tasks/spec to describe consumer migration boundaries.
- [x] 1.4 Validate with `openspec validate refactor-llm-provider-model-routing --strict`.

## 2. Impact and Baseline

- [x] 2.1 Run GitNexus impact for `RoutedLlmAdapter`.
- [x] 2.2 Run GitNexus impact for `resolve_model_selection`.
- [x] 2.3 Run GitNexus impact for `build_react_agent`.
- [x] 2.4 Run GitNexus impact for `LlmProviderAdapter`.
- [x] 2.5 Run GitNexus impact for `LlmProxy`.
- [x] 2.6 Run GitNexus impact for `resolve_model`.
- [x] 2.7 Run baseline focused tests and checks.

## 3. Framework Routed Adapter

- [x] 3.1 Add tests proving `RoutedLlmAdapter` uses `chat_with_selection` and fallback chain when no explicit model is supplied.
- [x] 3.2 Add tests proving explicit model override routes through router `chat`.
- [x] 3.3 Keep `LlmProviderAdapter` callable for legacy direct-provider integration.

## 4. Web Framework Runner

- [x] 4.1 Confirm coordinator / executor / runtime agent construction uses `RoutedLlmAdapter`.
- [x] 4.2 Confirm `FrameworkRunner::resolve_model_selection` uses `ModelSelectionRequest`.
- [x] 4.3 Verify production framework/web code does not call deprecated `resolve_provider_name`.

## 5. App LLM Proxy Migration

- [x] 5.1 Add router-backed `LlmProxy` constructor.
- [x] 5.2 Make router-backed proxy resolve user/app/agent precedence through `ModelSelectionRequest`.
- [x] 5.3 Mark legacy `LlmProxy::new` deprecated but callable.
- [x] 5.4 Migrate proxy tests to the router-backed constructor.
- [x] 5.5 Add compatibility test for deprecated constructor under local `#[allow(deprecated)]`.

## 6. Verification

- [x] 6.1 Run `cargo fmt`.
- [x] 6.2 Run `cargo test -p macaca-llm router -- --nocapture`.
- [x] 6.3 Run `cargo test -p macaca-framework adapter --features macaca-compat -- --nocapture`.
- [x] 6.4 Run `cargo test -p macaca-app llm_proxy -- --nocapture`.
- [x] 6.5 Run `cargo check -p macaca-llm -p macaca-framework -p macaca-app -p macaca-web`.
- [x] 6.6 Run deprecated usage grep for `resolve_provider_name` and legacy `LlmProxy::new`.
- [x] 6.7 Run `openspec validate refactor-llm-provider-model-routing --strict`.
- [x] 6.8 Run `gitnexus_detect_changes(scope: "all")`.
