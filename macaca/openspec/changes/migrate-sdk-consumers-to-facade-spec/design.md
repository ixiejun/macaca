## Context

The `refactor-macaca-sdk-patterns` change introduced the SDK producer primitives:

- `AgentSpec` as the builder product.
- `TracePolicy` as SDK trace metadata.
- `MacacaSdk` as the registration facade.
- `KernelAgentRegistry` as the kernel adapter behind the facade.

The remaining work is consumer migration. `macaca-app` still uses deprecated `register_from_config` during application startup, and upper tests still use `AgentBuilder::build_with_manifest`.

GitNexus impact analysis reports HIGH risk for both deprecated symbols because app startup feeds web and CLI entry flows:

- `register_from_config`: direct caller includes `macaca-app::runtime::start_app`; affected processes include `start_server` and CLI `main`.
- `build_with_manifest`: upper tests and compatibility helpers still call it.

## Goals

- Remove deprecated SDK usage from upper consumers.
- Preserve current runtime registration semantics.
- Keep deprecated SDK APIs available in `macaca-sdk` for compatibility and search-based migration discovery.
- Keep the change small and reversible.

## Non-Goals

- Do not delete deprecated SDK APIs.
- Do not change `Kernel::register_agent` behavior.
- Do not introduce a new app-specific registration abstraction.
- Do not wire `TracePolicy` into framework traced construction in this change.

## Design Decisions

### Use `MacacaSdk` for config registration

When a consumer wants to register an `AgentConfig` into a kernel, it SHALL use:

```rust
MacacaSdk::for_kernel(kernel).register_config(config).await
```

This preserves behavior because the facade delegates to `KernelAgentRegistry`, which delegates to `Kernel::register_agent`.

### Use `AgentSpec` for manual registration tests

When a test needs to manually call `kernel.register_agent(...)`, it SHALL build an `AgentSpec`, derive the manifest, then consume the spec into the agent:

```rust
let spec = AgentBuilder::from_config(config).build_spec()?;
let manifest = spec.manifest();
let agent = spec.into_agent();
kernel.register_agent(Box::new(agent), manifest).await?;
```

The manifest must be derived before `into_agent()` because `into_agent()` consumes the spec.

### Keep deprecated compatibility inside `macaca-sdk`

Deprecated APIs remain implemented and may still be tested inside `macaca-sdk`. Upper consumers should not use them because their continued use makes migration status ambiguous.

## Risks

- `macaca-app` startup is a production path; mitigate by using facade behavior that delegates to the same kernel registration path.
- Test-only migration can mask intent; mitigate with local helpers named around spec-based registration.
- Trace policy remains metadata-only; document this explicitly and do not claim traced construction is complete.
