## 1. Preparation

- [x] 1.1 Read `macaca/docs/agent-os-microkernel-boundaries.md`, `macaca/docs/route-c-regression-matrix.md`, `macaca/docs/route-c-phase-template.md`, `macaca/docs/route-c-architecture-governance.md`, and `docs/superpowers/plans/route-c-microkernel-ecosystem/phase-07-plugin-runtime-v0.md`.
- [x] 1.2 Review current service/package contracts in `macaca-proto`, `macaca-kernel`, `macaca-runtime-host`, `macaca-gateway`, `macaca-driver`, `macaca-memory`, `macaca-skill`, and MCP runtime host code.
- [x] 1.3 Review Route C Phase 04 package/runtime guard and Phase 02 system service contract changes for compatibility.
- [x] 1.4 Run GitNexus impact before modifying each selected symbol; warn before editing any HIGH or CRITICAL impact symbol.

## 2. Plugin Protocol Contracts

- [x] 2.1 Add `macaca/crates/macaca-proto/src/plugin.rs` with provider-neutral plugin manifest v0 contracts.
- [x] 2.2 Export plugin contracts from `macaca/crates/macaca-proto/src/lib.rs`.
- [x] 2.3 Define `PluginId`, `PluginVersion`, `DeveloperId`, `PluginManifest`, `PluginRuntimeDeclaration`, `PluginRuntimeKind`, `PluginEntryDeclaration`, `PluginProvidedService`, `PluginProvidedCapability`, `PluginRequiredService`, `PluginPermission`, `PluginResource`, `PluginSignature`, `PluginLifecycleState`, `PluginLifecycleEvent`, `PluginHealth`, and `PluginError`.
- [x] 2.4 Support gateway, driver, memory, context, skill, MCP, payment, compliance, and custom service/capability categories without hardcoding provider names.
- [x] 2.5 Add serde roundtrip tests for gateway, driver, memory/context, skill/MCP, and unsupported/custom plugin fixture manifests.
- [x] 2.6 Add validation tests proving missing permission/resource declarations and unsupported execution runtime kinds return structured errors instead of panics.

## 3. Runtime Host Plugin Facade

- [x] 3.1 Add `macaca/crates/macaca-runtime-host/src/plugin.rs` with `PluginRuntimeFacade`, `PluginManifestValidator`, `PluginRuntimeGuard`, `PluginHostFactory`, descriptor host, and lifecycle controller primitives.
- [x] 3.2 Export the plugin runtime facade from `macaca-runtime-host`.
- [x] 3.3 Implement Abstract Factory selection for descriptor/in-process built-in hosts while returning structured unavailable results for future WASM/native/process execution hosts.
- [x] 3.4 Implement Specification-style manifest, permission, resource, runtime kind, service descriptor, required service, and signature metadata validation.
- [x] 3.5 Add structured logs for manifest load, validation start/pass/reject, host factory selection, lifecycle start/pass/failure, registry call, and cleanup.
- [x] 3.6 Add detailed English comments explaining every public type/trait/function, runtime-kind boundary, non-execution invariant, and future proxy extension point.

## 4. Kernel Plugin Registry

- [x] 4.1 Add `macaca/crates/macaca-kernel/src/plugin_registry.rs` for plugin identity, lifecycle state, service descriptor ownership, capability descriptor ownership, and cleanup.
- [x] 4.2 Export the plugin registry from `macaca-kernel` without adding plugin capability behavior to the kernel.
- [x] 4.3 Enforce lifecycle transitions for installed, registered, starting, running, stopping, stopped, failed, and uninstalled states.
- [x] 4.4 Ensure a single plugin can register multiple services and capabilities as a Composite descriptor set.
- [x] 4.5 Ensure uninstall removes every service/capability descriptor owned by the plugin.
- [x] 4.6 Add registry tests for duplicate plugin id rejection, multi-service registration, invalid transition rejection, failed transition persistence, and uninstall cleanup.

## 5. Built-In Adapter Modeling

- [x] 5.1 Add built-in gateway descriptor adapter without changing current gateway execution path.
- [x] 5.2 Add built-in driver descriptor adapter without changing current driver execution path or driver trace payload shape.
- [x] 5.3 Add built-in memory/context descriptor adapter without binding plugin runtime to a concrete vector database or memory provider.
- [x] 5.4 Add built-in skill/MCP descriptor adapter without changing current skill-backed MCP runtime behavior.
- [x] 5.5 Add tests proving built-in adapter descriptors are queryable, provider-neutral, and optional/unavailable when the underlying service is absent.
- [ ] 5.6 Mark any newly bypassed direct descriptor construction paths as deprecated if a canonical plugin descriptor facade replaces them.

## 6. Lifecycle Trace And Audit

- [x] 6.1 Add plugin lifecycle trace event builders or command objects for install, register, start, running, stop, stopped, uninstall, and failure.
- [x] 6.2 Persist or emit lifecycle trace records through existing trace/audit infrastructure with plugin id, developer id, version, runtime kind, previous state, next state, service ids, capability ids, trace id, operation, status, and structured error code.
- [x] 6.3 Add tests proving every lifecycle transition emits a trace/audit record.
- [x] 6.4 Add tests proving failure transitions emit structured error records and do not leave active services registered after cleanup.
- [x] 6.5 Run a hardcode scan over new Plugin Runtime files for demo app names, workflow names, provider names, driver names, gateway names, model names, chain names, and business-specific routing.

## 7. Regression And Verification

- [x] 7.1 Run `openspec validate add-plugin-runtime-v0 --strict`.
- [x] 7.2 Run `cargo test -p macaca-proto plugin`.
- [x] 7.3 Run `cargo test -p macaca-runtime-host plugin_runtime`.
- [x] 7.4 Run `cargo test -p macaca-kernel plugin_registry`.
- [x] 7.5 Run targeted built-in adapter descriptor tests for gateway, driver, memory/context, skill, and MCP descriptors.
- [x] 7.6 Run `cargo test -p macaca-integration-tests --test route_c_baseline`.
- [x] 7.7 Run `cargo check -p macaca-runtime-host`.
- [x] 7.8 Run `cargo check --workspace`.
- [x] 7.9 Run `npx gitnexus detect-changes --repo agent` before committing and verify affected flows match expected Phase 07 scope.
