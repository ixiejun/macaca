# Change: Migrate macaca-sdk consumers to facade/spec primitives

## Why

`macaca-sdk` now exposes `AgentSpec` and `MacacaSdk` facade primitives, while upper consumers still call deprecated SDK helpers such as `register_from_config` and `AgentBuilder::build_with_manifest`.

Migrating consumers removes deprecated usage from production and upper tests without deleting compatibility APIs, making later migration searches reliable.

## What Changes

- Migrate `macaca-app` application startup from deprecated SDK registry helpers to `MacacaSdk::for_kernel(...).register_config(...)`.
- Migrate upper tests from `AgentBuilder::build_with_manifest` to `AgentBuilder::build_spec` plus `AgentSpec` conversion.
- Keep deprecated SDK APIs in `macaca-sdk` for compatibility and migration discovery.
- Add a deprecated-usage scan requirement so upper consumer code does not reintroduce old SDK entry points.

## Impact

- Affected specs: `macaca-sdk-consumers`
- Affected code: `macaca-app`, `macaca-kernel` tests, `macaca-integration-tests`
- Compatibility impact: no deprecated SDK APIs are removed; registration behavior remains delegated to the same kernel registration path.
