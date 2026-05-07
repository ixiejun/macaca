# Design: Plugin Runtime v0

## Context

Route C has already established microkernel boundaries, system service contracts, package manifest/runtime guard, Application ABI v0, and GenUI Runtime v0. Phase 07 introduces the extension mechanism that lets external and built-in capabilities appear as plugin-provided services while preserving OS invariants.

Plugin Runtime v0 deliberately starts with manifest, registry, lifecycle, permission/resource validation, trace, health, and built-in adapter modeling. It does not execute arbitrary third-party code yet. That restraint keeps the phase additive and safe while creating the contracts later Store/WASM/native plugin phases need.

## Goals

- Define plugin manifest v0 contracts in `macaca-proto`.
- Provide a runtime-host plugin facade that validates manifests and owns runtime-kind host creation boundaries.
- Add a kernel plugin registry that tracks plugin identity, lifecycle state, provided services, provided capabilities, and cleanup.
- Model built-in gateway/driver/memory/skill/MCP capabilities as plugin-provided service descriptors without changing their current execution paths.
- Require permission and resource declarations before registration.
- Emit structured logs and trace/audit records for plugin lifecycle transitions and failures.
- Preserve existing driver, skill, MCP, gateway, trace, chat, task, and session behavior.
- Keep the architecture ready for future process/WASM/native plugin hosts through Abstract Factory and Proxy boundaries.

## Non-Goals

- No third-party code execution in Phase 07.
- No WASM component runtime for plugins.
- No native plugin process launcher.
- No Store install flow, entitlement checks, billing, subscription, package encryption, or marketplace distribution.
- No migration that forces existing gateway/driver/memory/skill/MCP implementations to become plugins immediately.
- No kernel-owned plugin business logic.
- No application-specific or provider-specific plugin branching.

## Superpowers Brainstorm Summary

### Current Problem

Macaca OS has many extension surfaces, but without a common plugin contract each surface will grow its own manifest, permission, lifecycle, trace, and health model. That creates duplicated integration logic and makes third-party extension unsafe.

### Why Phase 07 Must Solve It

Route C Phase 08 Store/Entitlement depends on installable packages. Phase 09+ payment/Web3/EVM and future driver/gateway ecosystems depend on plugin manifest, lifecycle, and trace semantics. Phase 07 must establish those semantics before any external execution path exists.

### Options Considered

1. **Manifest-first plugin runtime with built-in adapter descriptors.**
   - Pros: safe, additive, traceable, registry-ready, no arbitrary code execution, compatible with current runtime paths.
   - Cons: v0 plugins are descriptors/adapters rather than full third-party execution units.
   - Verdict: recommended.

2. **Execute third-party WASM/native plugins immediately.**
   - Pros: faster ecosystem demo.
   - Cons: unsafe before permission/resource/lifecycle/trace/store contracts are stable; violates Route C incremental plan.
   - Verdict: rejected for Phase 07.

3. **Keep plugins inside each service crate independently.**
   - Pros: local ownership by gateway/driver/memory/skill teams.
   - Cons: duplicated manifests, inconsistent permissions, no unified lifecycle/audit, difficult Store integration.
   - Verdict: rejected.

4. **Put plugin execution and capability logic into kernel.**
   - Pros: centralized control.
   - Cons: violates kernel boundary; plugin capability changes are not kernel invariants.
   - Verdict: rejected.

### Recommended Plan

Implement Plugin Runtime v0 additively: protocol contracts first, runtime-host facade second, kernel registry third, built-in adapter descriptors fourth, lifecycle trace fifth, then targeted tests and Route C regression checks.

## Design Patterns

- **Abstract Factory**: `PluginHostFactory` creates runtime-kind host adapters without callers depending on concrete host types. Phase 07 only returns descriptor/in-process built-in hosts; future WASM/process/native hosts can plug into the same factory.
- **Adapter**: built-in gateway/driver/memory/skill/MCP capabilities are represented as plugin-provided service descriptors without changing their existing implementation paths.
- **Composite**: one `PluginManifest` can provide many services and capabilities; registry cleanup removes the whole plugin-provided service subtree atomically.
- **State**: plugin lifecycle is a typed state machine with allowed transitions and structured failure states.
- **Proxy**: future external process/WASM/native plugins will be reached through proxies; Phase 07 defines proxy-ready contracts but does not launch them.
- **Specification**: manifest validation, permission/resource requirements, signature metadata, runtime kind support, and service descriptor validity are explicit rules.
- **Facade**: runtime-host exposes a single Plugin Runtime facade that hides manifest parsing, validation, host factory selection, registry calls, and trace emission.
- **Observer**: lifecycle events are emitted to trace/audit sinks and structured logs.
- **Null Object**: unsupported/disabled/unavailable plugin hosts return structured unavailable results, never panic or hang.

## Architecture Boundary

### `macaca-proto/src/plugin.rs`

The protocol module should define data-only contracts:

- `PluginId`
- `PluginVersion`
- `DeveloperId`
- `PluginManifest`
- `PluginRuntimeDeclaration`
- `PluginRuntimeKind`
- `PluginEntryDeclaration`
- `PluginProvidedService`
- `PluginProvidedCapability`
- `PluginRequiredService`
- `PluginPermission`
- `PluginResource`
- `PluginSignature`
- `PluginLifecycleState`
- `PluginLifecycleEvent`
- `PluginHealth`
- `PluginError`

The schema must be serde-friendly, provider-neutral, and explicit about unsupported runtime kinds. It must not depend on `macaca-web`, concrete gateway/driver/memory/skill/MCP implementations, Store, Web3, EVM, payment, chain providers, or business workflows.

### `macaca-runtime-host/src/plugin.rs`

Runtime-host should provide:

- `PluginRuntimeFacade`
- `PluginManifestValidator`
- `PluginHostFactory`
- `PluginHost` trait or descriptor host abstraction
- `PluginRuntimeGuard`
- `PluginLifecycleController`
- built-in descriptor host support for Phase 07

The facade validates manifests, selects a host factory strategy, calls the kernel registry, emits lifecycle traces/logs, and returns structured unavailable/unsupported errors for future runtime kinds.

### `macaca-kernel/src/plugin_registry.rs`

Kernel should own only registry invariants:

- plugin id uniqueness;
- lifecycle state per plugin;
- service descriptor ownership by plugin;
- capability descriptor ownership by plugin;
- install/register/start/stop/uninstall state transitions;
- cleanup of service descriptors on uninstall/failure.

Kernel must not execute plugin code and must not implement gateway/driver/memory/skill/MCP behavior.

### Built-In Adapter Modeling

Existing built-in gateway/driver/memory/skill/MCP capabilities should be represented as plugin-provided service descriptors. This is an Adapter layer only:

- existing runtime code paths continue to work;
- descriptors can be queried for diagnostics and future Store/runtime wiring;
- missing optional gateways/drivers return structured unavailable and do not break base OS;
- no concrete provider or application names are embedded in plugin runtime logic.

## Lifecycle State Machine

Allowed v0 lifecycle path:

```text
installed -> registered -> starting -> running -> stopping -> stopped -> uninstalled
```

Failure transitions:

```text
installed -> failed
registered -> failed
starting -> failed
stopping -> failed
```

Failed plugins must retain an auditable error state and must not leave provided services active unless the failure happened after a successful running state and rollback semantics explicitly preserve stopped descriptors.

## Trace, Audit, And Logging

Phase 07 implementation must log and trace:

- manifest loaded;
- manifest validation started/passed/rejected;
- permission/resource guard started/passed/rejected;
- plugin installed;
- plugin registered;
- host factory selected;
- plugin starting;
- plugin running;
- plugin stopping;
- plugin stopped;
- plugin uninstall started/completed;
- service descriptor registered/removed;
- lifecycle failure.

Trace/log payloads should include plugin id, developer id, version, runtime kind, lifecycle state, previous state, next state, service ids, capability ids, resource ids, permission ids, trace id, operation name, structured status, and structured error code. Payloads must not include private keys, raw signatures beyond bounded fingerprints, secrets, provider credentials, payment credentials, encrypted package contents, or unbounded user input.

## Security And Permission Rules

- Plugins without permissions must be rejected unless they declare an explicit empty permission set and provide no privileged capability.
- Runtime kind `wasm`, `native`, or `process` may be represented but must be unavailable in Phase 07 unless explicitly modeled as descriptor-only.
- Signature metadata must be present, but cryptographic verification can remain a Store/Runtime Guard follow-up if represented as `verification_pending` or `unsupported_in_v0`.
- Plugin-provided services must declare service identity, service kind, capability ids, lifecycle coupling, and required permission ids.
- Plugin registration must fail if required services are missing and the plugin does not declare degraded/unavailable behavior.

## Compatibility And Regression

Phase 07 must preserve:

- `RC-DRIVER-001`: driver execution trace still includes driver name and action through existing runtime path.
- `RC-SKILL-001`: skill/MCP smoke path still runs and emits trace.
- `RC-TRACE-001`: plugin lifecycle trace uses the same event/trace infrastructure without breaking existing real-time trace updates.

Existing YAML applications, `/api/chat/v2`, task board, session logs, driver traces, skill/MCP traces, frontend, CLI, and GenUI must continue to compile and run through current paths.

## Risks / Trade-Offs

- **Risk: v0 feels too limited because it does not run third-party code.** Mitigation: manifest/registry/lifecycle is the safe dependency for later execution phases.
- **Risk: plugin registry becomes a service implementation hub.** Mitigation: kernel stores descriptors and lifecycle only; capability behavior stays in services/plugins.
- **Risk: adapter modeling accidentally changes built-in runtime behavior.** Mitigation: descriptors are additive diagnostics; no execution path replacement in Phase 07.
- **Risk: permissions become decorative.** Mitigation: registration rejects missing permission/resource declarations and records guard decisions.
- **Risk: lifecycle trace leaks secrets.** Mitigation: bounded identifiers/fingerprints only; no credentials or unbounded payloads.

## Open Questions

- Store-backed signature verification and entitlement enforcement are deferred to Phase 08.
- WASM/native/process plugin execution is deferred to later dedicated runtime phases.
- UI surfaces for plugin management are deferred to Web/CLI thin shell phases.
