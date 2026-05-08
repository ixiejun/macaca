# Application Development Guide

Macaca applications are distributable packages that run on the Agent OS application framework. Current first-class formats are YAML applications and metadata-only WASM packages. YAML apps remain fully supported; WASM packages can be checked and loaded as metadata, but execution must return a structured runtime-unavailable result until a WASM runtime is installed.

## Package Requirements

- `package_type`: `application`
- `runtime.kind`: `yaml` or `wasm_component`
- `runtime.abi_version`: host-supported ABI version
- `metadata.package.manifest.version`: package manifest version
- `metadata.trace.required`: `true`
- `entry`: agent, workflow, component, UI surface, or another declared entry kind

## Permissions And Capabilities

Applications declare permissions in package metadata. The checker validates that each permission has a name and reason, but permission approval remains owned by policy services. Applications declare provided capabilities such as application execution, UI rendering, or optional module usage. Capability calls must still go through the system policy layer.

## Trace And Audit

Every application package should declare trace requirements. Runtime actions such as app lifecycle, agent execution, tool calls, service calls, GenUI render commands, and optional module degradation must emit trace/audit events. A package that omits trace metadata can still be metadata-compatible, but certification reports a warning.

## YAML Application Path

YAML applications are loaded through the compatibility adapter and converted to Package Manifest v0 descriptors. Third-party developers do not need to modify Macaca source code. They provide manifest files and package metadata, run certification, then install the package through the supported package path.

## WASM Stub Path

WASM application packages declare `runtime.kind = wasm_component`. Phase 13 certification verifies metadata, ABI, permissions, trace, and optional service declarations. Execution remains unavailable until the WASM Application ABI runtime exists, and the host must report that as structured `runtime_unavailable` rather than panic, hang, or silently succeed.

## Certification

Run:

```bash
cargo test -p macaca-integration-tests package_certification
```

The certification suite checks YAML, WASM, GenUI, plugin, Store, Web3, and EVM paths without real external services. A package passes only when diagnostics are explicit and traceable.
