# Design: Plugin SDK and Hosts v1

## Context

Plugin developers need a stable API similar to OpenClaw's plugin SDK and Hermes's `PluginContext`, but Macaca must preserve stricter OS-grade boundaries. SDK/ABI/Hosts v1 introduces the public facade and host skeletons after the control plane, capability registry, and hook bus contracts are defined.

## Goals

- Provide a stable Plugin SDK facade that hides internal crates.
- Provide contract test utilities for plugin authors and built-in plugin adapters.
- Define host skeletons for descriptor, built-in, WASM, process, and remote proxy plugin models.
- Keep skeleton hosts unavailable-safe until real execution is separately approved.
- Make host lifecycle traceable, bounded, and resource-aware.

## Non-Goals

- No full WASM execution implementation unless separately approved.
- No real process spawning unless separately approved.
- No real remote network transport unless separately approved.
- No marketplace publishing implementation.
- No SDK support for provider-specific shortcuts.

## Architecture

```text
Plugin Developer / Built-in Adapter
  -> macaca-sdk PluginSdk / PluginContext
  -> Manifest + Capability + Hook Builders
  -> Contract Test Kit
  -> Plugin Control/Capability/Hook Services
  -> PluginHostFactory
  -> Descriptor / BuiltIn / WasmUnavailable / ProcessUnavailable / RemoteProxyUnavailable
```

## Design Patterns

- **Facade**: `PluginSdk` and `PluginContext` expose a small stable developer API.
- **Builder**: manifest, capability, hook, config, secret, and fixture builders produce valid contracts.
- **Abstract Factory**: runtime-host selects host skeletons by runtime kind.
- **Proxy**: WASM/process/remote hosts expose proxy boundaries and do not leak internals.
- **State**: host lifecycle is explicit and auditable.
- **Resource Manager**: host skeletons define resource lease boundaries.
- **Null Object**: unavailable skeleton hosts return structured unavailable results.
- **Specification**: contract test kit validates plugin contracts.

## SDK Surface

The SDK should include:

- `PluginSdk`
- `PluginContext`
- `PluginManifestBuilder`
- `PluginRegistrationBuilder`
- `PluginCapabilityBuilder`
- `PluginHookBuilder`
- `PluginConfigBuilder`
- `PluginSecretRequirementBuilder`
- `PluginContractTestKit`

The SDK must use `macaca-proto` DTOs and service clients. It must not expose `macaca-kernel` or runtime-host internals.

## Host Skeletons

Host skeletons define lifecycle, trace, health, timeout, and resource policy without executing unapproved code:

- `DescriptorPluginHost`
- `BuiltInAdapterPluginHost`
- `WasmPluginHost` unavailable-safe skeleton
- `ProcessPluginHost` unavailable-safe skeleton
- `RemoteProxyPluginHost` unavailable-safe skeleton

## Trace And Logging

Host lifecycle operations must log and emit trace/audit events for host selection, prepare, start, call, hook invoke, stop, cleanup, health probe, timeout, resource denial, and unavailable results.

## Risks

- **Risk: SDK leaks internal architecture.** Mitigation: SDK only exposes proto DTOs and service clients.
- **Risk: Skeletons imply unsafe execution is ready.** Mitigation: unavailable-safe behavior and clear diagnostics until real execution phases.
- **Risk: Host factory accumulates provider-specific branches.** Mitigation: branch only on runtime kind and descriptor metadata.
