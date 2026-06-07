# Change: Remove kernel ownership of provider construction

## Why

`macaca-kernel` still owns direct construction and wiring of replaceable provider crates. That keeps the kernel broader than Route C allows and makes provider migration depend on kernel internals instead of service and facade boundaries.

## What Changes

- Move kernel provider-facing construction behind a temporary compatibility adapter layer.
- Deprecate direct provider-oriented kernel constructors and builder entry points while keeping them callable for migration.
- Refactor kernel-facing composition toward facade-oriented, provider-neutral entry points.
- Reduce direct `macaca-kernel` coupling to provider implementation crates where it is no longer needed.
- Keep deprecated shims searchable so later migrations can find and replace them without deleting historical call sites.

## Non-Goals

- Do not migrate LLM, Memory, Task, Driver, Skill, MCP, Gateway, Payment, Web3, or EVM providers in this change.
- Do not remove current allowlist debt unless the dependency is genuinely eliminated.
- Do not change existing user-visible Web, CLI, application, or trace behavior.
- Do not hardcode application, provider, workflow, model, driver, gateway, or business names.

## Impact

- Affected specs: `kernel-facade`, `microkernel-primitives`, `macaca-kernel-patterns`
- Affected code:
  - `macaca/crates/macaca-kernel/Cargo.toml`
  - `macaca/crates/macaca-kernel/src/kernel.rs`
  - `macaca/crates/macaca-kernel/src/kernel_builder.rs`
  - `macaca/crates/macaca-kernel/src/services.rs`
  - `macaca/crates/macaca-kernel/src/facade.rs`
  - `macaca/crates/macaca-kernel/src/provider_compat.rs`
  - `macaca/crates/macaca-kernel/src/registry.rs`
  - `macaca/crates/macaca-kernel/src/scheduler.rs`
  - `macaca/crates/macaca-kernel/src/service_bus_bridge.rs`
- Affected docs:
  - `macaca/docs/route-c-architecture-governance.md`
  - `macaca/docs/route-c-serviceization-allowlist.md`

## Migration posture

This change is a boundary cleanup, not the provider migration itself. It prepares the kernel so later service-runtime migrations can remove provider ownership without forcing a big-bang rewrite.
