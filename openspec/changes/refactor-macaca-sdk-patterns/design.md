## Context

`macaca-sdk` exposes declarative config parsing, agent building, persona loading, and registry helpers. It is consumed by `macaca-app`, `macaca-web`, integration tests, and application developers.

`register_from_config` is a critical path because app startup uses it to register application agents. This change must be additive-first and must not change `Kernel::register_agent` semantics.

## Goals

- Keep existing behavior 1:1 compatible.
- Add `AgentSpec` as a stable SDK declaration product.
- Add persona prototype clone/override support.
- Split validation into a chain of small validators while preserving current validation results.
- Add an SDK facade and registry adapter without changing registration behavior.
- Ensure SDK-built agent specs carry trace policy metadata.

## Non-Goals

- Do not remove `DeclarativeAgent`.
- Do not remove `AgentBuilder::build` or `AgentBuilder::build_with_manifest`.
- Do not remove `register_from_config` or `register_from_file`.
- Do not connect SDK directly to web/session/EventLog/SSE in this change.
- Do not migrate app/web consumers to the new facade in this change unless required by tests.
- Do not introduce new dependencies.
- Do not hardcode application, workflow, driver, skill, or agent names.

## Decisions

- `AgentBuilder::build_spec` is the new primary builder product.
- `AgentBuilder::build` delegates through `AgentSpec` to keep current behavior.
- `TracePolicy::Required` is the default for all `AgentSpec` values.
- `SdkValidationChain::default` implements the exact current `AgentConfig::validate` rules.
- `MacacaSdk` wraps an `AgentRegistryApi` trait; the first adapter targets `macaca_kernel::Kernel`.
- Deprecated registry helpers delegate to `MacacaSdk` after the facade exists.

## Risk Controls

- `register_from_config` has CRITICAL blast radius, so the facade remains behavior-preserving and uses the same `Kernel::register_agent` call.
- Existing public APIs are deprecated only after equivalent additive entry points exist.
- Validation chain starts with current rules only; future registry-aware validators require separate proposals.
- Trace policy is metadata-only in this change; traced runtime migration remains separate.
